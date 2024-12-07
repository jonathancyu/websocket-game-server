use std::{
    collections::HashMap,
    net::{Ipv6Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        broadcast,
        mpsc::{self, Sender},
        Mutex,
    },
    time,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    model::messages::{
        ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse, Player,
        SocketRequest, SocketResponse, UserId,
    },
    utility::channel::Channel,
};

#[derive(Clone)]
pub struct Connection<RS>
where
    RS: Clone,
{
    user_id: UserId,
    to_socket: Channel<RS>,
}
impl<RS> Connection<RS>
where
    RS: Clone,
{
    pub fn new(user_id: UserId, to_socket: Channel<RS>) -> Self {
        Connection { user_id, to_socket }
    }
}

pub struct WebSocketState<T: Clone> {
    user_handles: HashMap<UserId, Connection<T>>,
}

pub struct WebSocketHandler {
    state: Arc<Mutex<WebSocketState<MatchmakingResponse>>>,
}

#[async_trait]
pub trait WebsocketHandlerTrait<ConnectionState, ExternalRQ, ExternalRS, InternalRQ, InternalRS>
where
    Self: 'static,
    ExternalRQ: Deserialize<'static>,
    ExternalRS: Serialize + Send,
    InternalRQ: Clone + Send + 'static,
    InternalRS: Clone + Send + 'static,
{
    // TODO: rename after refactor
    fn get_state(&self) -> Arc<Mutex<WebSocketState<InternalRS>>>;
    async fn listen(
        &mut self,
        address: String,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        mm_sender: Sender<InternalRQ>,
    ) {
        let ws_listener = TcpListener::bind(address.clone())
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to bind to {}: {}", address, e);
            });
        info!("Initialized ws listener: {}", address);
        loop {
            tokio::select! {
                result = ws_listener.accept() => {
                    match result {
                        Err(e) => {
                            error!("Failed to accept connection from {} with error: {}", address, e);
                        }
                        Ok((stream, address)) => {
                            tokio::spawn(Self::connection_thread(
                                self.get_state(),
                                stream,
                                address,
                                mm_sender.clone(),
                            ));
                        }
                    }
                },
                _ = shutdown_receiver.recv() => {
                    break;
                }
            };
        }
        info!("Exited ws listener");
    }

    // Thread to handle connection lifetime
    async fn connection_thread(
        state: Arc<Mutex<WebSocketState<InternalRS>>>,
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Sender<InternalRQ>,
    ) {
        info!("New ws connection: {}", address);

        let stream = accept_async(stream).await.unwrap();
        let (mut ws_sender, mut ws_receiver) = stream.split();
        let mut interval = time::interval(Duration::from_secs(1));
        // TODO: handshake, then resolve connection from state
        let mut connection_id: Option<UserId> = None;
        let user_id = UserId(Uuid::new_v4());

        // Lookup user's Connection by user_id
        let connection = state
            .lock()
            .await
            .user_handles
            .entry(user_id.clone())
            .or_insert_with(|| Connection::new(user_id, Channel::from(mpsc::channel(100))))
            .clone();

        loop {
            tokio::select! {
                // Poll connection for any push messages
                _ = interval.tick() => {
                    let response = Self::handle_internal_message(connection.clone()).await;
                    if let Some(response) = response {
                        let response = serde_json::to_string(&response).expect("Could not serialize response.");
                        ws_sender.send(Message::Text(response)).await.unwrap();
                    }
                }

                // Otherwise, handle incoming messages
                msg = ws_receiver.next() => {
                    let Some(msg) = msg else {
                        debug!("WS received empty message, TODO"); // TODO: what to do
                        break;
                    };
                    let msg = msg.expect("Couldn't unwrap message");

                    let result = Self::handle_external_message(
                        connection.clone(),
                        msg,
                        mm_sender.clone()
                    ).await;

                    match result  {
                        Ok(response) => {
                            let response = serde_json::to_string(&response).expect("Could not serialize response.");
                            ws_sender.send(Message::Text(response)).await.unwrap();
                        },
                        Err(_) => break,
                    };
                }
            }
        }
    }

    // Read internal message to potentially push a message back to the user.
    async fn handle_internal_message(
        connection: Connection<InternalRS>,
    ) -> Option<SocketResponse<ExternalRS>>;

    // Read message from connection, return immediate response
    async fn handle_external_message(
        connection: Connection<InternalRS>,
        message: Message,
        mm_sender: Sender<InternalRQ>,
    ) -> Result<SocketResponse<ExternalRS>, &'static str>;

    // Logic to handle a client's request
    async fn respond_to_request(
        connection: Connection<InternalRS>,
        request: ClientRequest,
        mm_sender: Sender<InternalRQ>,
    ) -> ExternalRS;
}

#[async_trait]
impl
    WebsocketHandlerTrait<
        Connection<MatchmakingResponse>,
        ClientRequest,
        ClientResponse,
        MatchmakingRequest,
        MatchmakingResponse,
    > for WebSocketHandler
{
    fn get_state(&self) -> Arc<Mutex<WebSocketState<MatchmakingResponse>>> {
        self.state.clone()
    }

    async fn handle_internal_message(
        connection: Connection<MatchmakingResponse>,
    ) -> Option<SocketResponse<ClientResponse>> {
        // See if MM sent any messages
        let message = connection.to_socket.receiver.lock().await.recv().await?;

        // If message was sent, forward to user
        Some(SocketResponse {
            user_id: connection.user_id,
            message: match message {
                MatchmakingResponse::QueueJoined => ClientResponse::JoinedQueue,
                MatchmakingResponse::MatchFound(game) => ClientResponse::MatchFound {
                    game_id: game.id,
                    server_address: game.server_address,
                },
            },
        })
    }

    async fn handle_external_message(
        connection: Connection<MatchmakingResponse>,
        message: Message,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> Result<SocketResponse<ClientResponse>, &'static str> {
        if !message.is_text() {
            return Err("Got non-text message :(");
        }

        // Deserialize request
        let body = message.to_text().unwrap();
        debug!(body);
        let request: SocketRequest =
            serde_json::from_str(body).expect("Could not deserialize request.");
        // If client provided user_id, use it. Otherwise give them a new one.
        let user_id = match request.user_id {
            Some(user_id) => user_id,
            None => UserId(Uuid::new_v4()),
        };

        debug!("Got message {:?}", &message);
        Ok(SocketResponse {
            user_id,
            message: WebSocketHandler::respond_to_request(connection, request.request, mm_sender)
                .await,
        })
    }

    async fn respond_to_request(
        connection: Connection<MatchmakingResponse>,
        request: ClientRequest,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> ClientResponse {
        match request {
            ClientRequest::JoinQueue => {
                let mm_request = MatchmakingRequest::JoinQueue(Player {
                    id: connection.user_id.clone(),
                    sender: connection.to_socket.sender.clone(),
                });
                mm_sender.send(mm_request).await.unwrap();
                ClientResponse::AckJoinQueue
            }
            ClientRequest::Ping => ClientResponse::QueuePing { time_elapsed: 0u32 },
            ClientRequest::GetServer => ClientResponse::JoinServer {
                server_ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            },
        }
    }
}
impl WebSocketHandler {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WebSocketState {
                user_handles: HashMap::new(),
            })),
        }
    }
}
