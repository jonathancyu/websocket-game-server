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
pub struct Connection {
    user_id: UserId,
    mm_to_ws: Channel<MatchmakingResponse>,
}

pub struct WebSocketState {
    user_handles: HashMap<UserId, Connection>,
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
    InternalQS,
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
                            tokio::spawn(Self::handle_connection(
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
    async fn handle_connection(
        state: Arc<Mutex<SocketState>>,
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Sender<InternalRQ>,
    );

    async fn poll_pushed_messages(
        state: Arc<Mutex<WebSocketState>>,
        connection_id: Option<UserId>,
    ) -> Option<SocketResponse<ExternalRS>>;

    async fn handle_socket_message(
        message: Option<Result<Message, tungstenite::Error>>,
        state: Arc<Mutex<WebSocketState>>,
        mm_sender: Sender<InternalRQ>,
    ) -> Result<SocketResponse<ExternalRS>, &'static str>;

    async fn handle_client_request(
        connection: &Connection,
        request: ClientRequest,
        mm_sender: Sender<InternalRQ>,
    ) -> ExternalRS;
}

#[async_trait]
impl
    WebsocketHandlerTrait<
        WebSocketState,
        Connection,
        ClientRequest,
        ClientResponse,
        MatchmakingRequest,
        MatchmakingResponse,
    > for WebSocketHandler
{
    fn get_state(&self) -> Arc<Mutex<WebSocketState>> {
        self.state.clone()
    }
    async fn handle_connection(
        state: Arc<Mutex<WebSocketState>>,
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Sender<MatchmakingRequest>,
    ) {
        info!("New ws connection: {}", address);
        // TODO: implement with protobuf (prost)
        // ISSUE: We have N threads spawning instead of spawning 1 thread to read N connections
        // (but lock state the whole time)

        let stream = accept_async(stream).await.unwrap();
        let (mut ws_sender, mut ws_receiver) = stream.split();
        let mut interval = time::interval(Duration::from_secs(1));
        let mut connection_id: Option<UserId> = None;

        loop {
            tokio::select! {
                // Poll connection for any push messages
                _ = interval.tick() => {
                    let response = WebSocketHandler::poll_pushed_messages(state.clone(), connection_id.clone()).await;
                    if let Some(response) = response {
                        let response = serde_json::to_string(&response).expect("Could not serialize response.");
                        ws_sender.send(Message::Text(response)).await.unwrap();
                    }
                }
                // Otherwise, handle incoming messages
                msg = ws_receiver.next() => {
                    let result = WebSocketHandler::handle_socket_message(
                        msg,
                        state.clone(),
                        mm_sender.clone()
                    ).await;

                    match result  {
                        Ok(response) => {
                            if let Some(connection_id) = connection_id.clone() {
                                if connection_id != response.user_id {
                                    warn!("Somehow connection ID != response.user_id");
                                }
                            } else {
                                connection_id = Some(response.clone().user_id);
                            }

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
    async fn poll_pushed_messages(
        state: Arc<Mutex<WebSocketState>>,
        connection_id: Option<UserId>,
    ) -> Option<SocketResponse<ClientResponse>> {
        let Some(connection_id) = connection_id else {
            warn!("Cannot poll connection w/o connection_id");
            return None;
        };
        // Get connection handle
        let connection = state
            .lock()
            .await
            .user_handles
            .get(&connection_id)
            .expect("User has connection_id but is not in user_handles")
            .clone();

        // See if MM sent any messages
        let message = connection.mm_to_ws.receiver.lock().await.recv().await?;

        // If message was sent, forward to user
        Some(SocketResponse {
            user_id: connection_id,
            message: match message {
                MatchmakingResponse::QueueJoined => ClientResponse::JoinedQueue,
                MatchmakingResponse::MatchFound(game) => ClientResponse::MatchFound {
                    game_id: game.id,
                    server_address: game.server_address,
                },
            },
        })
    }

    async fn handle_socket_message(
        message: Option<Result<Message, tungstenite::Error>>,
        state: Arc<Mutex<WebSocketState>>,
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

        // Lookup user's Connection by user_id
        let connection = state
            .lock()
            .await
            .user_handles
            .entry(user_id.clone())
            .or_insert_with(|| Connection {
                user_id: user_id.clone(),
                mm_to_ws: Channel::from(mpsc::channel(100)),
            })
            .clone();

        debug!("Got message {:?}", &msg);

        Ok(SocketResponse {
            user_id,
            message: WebSocketHandler::handle_client_request(
                &connection,
                request.request,
                mm_sender,
            )
            .await,
        })
    }

    async fn handle_client_request(
        connection: &Connection,
        request: ClientRequest,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> ClientResponse {
        match request {
            ClientRequest::JoinQueue => {
                let mm_request = MatchmakingRequest::JoinQueue(Player {
                    id: connection.user_id.clone(),
                    sender: connection.mm_to_ws.sender.clone(),
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
