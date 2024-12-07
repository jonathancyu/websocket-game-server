use std::{
    collections::HashMap,
    net::{Ipv6Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        broadcast,
        mpsc::{self, Sender},
        Mutex,
    },
    time,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{self, Message},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    model::messages::{
        ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse, Player,
        SocketRequest, SocketResponse, UserId,
    },
    utility::channel::Channel,
};

#[derive(Clone)]
pub struct Connection<RS> {
    user_id: UserId,
    to_socket: Channel<RS>,
}

pub struct WebSocketState {
    user_handles: HashMap<UserId, Connection<MatchmakingResponse>>,
}
impl WebSocketState {}

pub struct WebSocketHandler {
    state: Arc<Mutex<WebSocketState>>,
}
#[async_trait]
pub trait WebsocketHandlerTrait<
    SocketState,
    ConnectionState,
    ExternalRQ,
    ExternalRS,
    InternalRQ: Send + 'static,
    InternalRS,
>
{
    // TODO: rename after refactor
    fn get_state(&self) -> Arc<Mutex<SocketState>>;
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
        state: Arc<Mutex<SocketState>>,
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Sender<InternalRQ>,
    );

    // Read internal message to potentially push a message back to the user.
    async fn handle_internal_message(
        connection: Connection<InternalRS>,
    ) -> Option<SocketResponse<ExternalRS>>;

    // Read message from connection, return immediate response
    async fn handle_external_message(
        connection: Connection<InternalRS>,
        message: Option<Result<Message, tungstenite::Error>>,
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
        WebSocketState,
        Connection<MatchmakingResponse>,
        ClientRequest,
        ClientResponse,
        MatchmakingRequest,
        MatchmakingResponse,
    > for WebSocketHandler
{
    fn get_state(&self) -> Arc<Mutex<WebSocketState>> {
        self.state.clone()
    }

    async fn connection_thread(
        state: Arc<Mutex<WebSocketState>>,
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Sender<MatchmakingRequest>,
    ) {
        info!("New ws connection: {}", address);

        let stream = accept_async(stream).await.unwrap();
        let (mut ws_sender, mut ws_receiver) = stream.split();
        let mut interval = time::interval(Duration::from_secs(1));
        // TODO: handshake, then resolve connection from state
        let mut connection_id: Option<UserId> = None;
        let user_id = UserId(Uuid::new_v4());

        // Lookup user's Connection by user_id
        let connection = {
            state
                .lock()
                .await
                .user_handles
                .entry(user_id.clone())
                .or_insert_with(|| Connection {
                    user_id: user_id.clone(),
                    to_socket: Channel::from(mpsc::channel(100)),
                })
                .clone()
        };

        loop {
            tokio::select! {
                // Poll connection for any push messages
                _ = interval.tick() => {
                    let response = WebSocketHandler::handle_internal_message(connection.clone()).await;
                    if let Some(response) = response {
                        let response = serde_json::to_string(&response).expect("Could not serialize response.");
                        ws_sender.send(Message::Text(response)).await.unwrap();
                    }
                }

                // Otherwise, handle incoming messages
                msg = ws_receiver.next() => {
                    let result = WebSocketHandler::handle_external_message(
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

    // TODO: Should just pass in the connection since it's cloneable.
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
        message: Option<Result<Message, tungstenite::Error>>,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> Result<SocketResponse<ClientResponse>, &'static str> {
        let msg = match message {
            None => {
                debug!("Websocket closed");
                // TODO: Likely should be handled on the matchmaking end

                // mm_sender
                //     .send(MatchmakingRequest::Disconnected(connection.user_id.clone()))
                //     .await
                //     .expect("Failed to send leave queue");
                return Err("Websocket closed");
            }
            Some(msg) => msg,
        }
        .expect("Couldn't unwrap msg");

        if !msg.is_text() {
            return Err("Got non-text message :(");
        }

        // Deserialize request
        let body = msg.to_text().unwrap();
        debug!(body);
        let request: SocketRequest =
            serde_json::from_str(body).expect("Could not deserialize request.");
        // If client provided user_id, use it. Otherwise give them a new one.
        let user_id = match request.user_id {
            Some(user_id) => user_id,
            None => UserId(Uuid::new_v4()),
        };

        debug!("Got message {:?}", &msg);

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
