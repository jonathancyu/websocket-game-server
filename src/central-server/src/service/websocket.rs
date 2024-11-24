use std::{
    collections::HashMap,
    net::{Ipv6Addr, SocketAddr},
    sync::Arc,
};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{connect, Message},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    model::messages::{
        ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse, QueuedPlayer,
        SocketRequest, UserId,
    },
    utility::channel::Channel,
};

struct Connection {
    user_id: UserId,
    mm_to_ws: Channel<MatchmakingResponse>,
}

struct WebSocketState {
    user_handles: HashMap<SocketAddr, Connection>,
}
impl WebSocketState {}

pub struct WebSocketHandler {
    pub url: String,
    pub port: String,
    state: Arc<Mutex<WebSocketState>>,
}
impl WebSocketHandler {
    pub fn new(url: String, port: String) -> Self {
        Self {
            url,
            port,
            state: Arc::new(Mutex::new(WebSocketState {
                user_handles: HashMap::new(),
            })),
        }
    }
    fn format_address(&self) -> String {
        format!("{}:{}", self.url, self.port)
    }

    pub async fn listen(
        &mut self,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        mm_sender: Sender<MatchmakingRequest>,
        mm_listener: Arc<Mutex<Receiver<MatchmakingResponse>>>,
    ) {
        // TODO: Implement polling mm listener
        let address = self.format_address();
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
                            tokio::spawn(WebSocketHandler::handle_connection(
                                self.state.clone(),
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
        state: Arc<Mutex<WebSocketState>>,
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Sender<MatchmakingRequest>,
    ) {
        info!("New ws connection: {}", address);
        // TODO: implement with protobuf (prost)
        let stream = accept_async(stream).await.unwrap();
        let (mut ws_sender, mut ws_receiver) = stream.split();
        loop {
            // Receive message
            let msg = ws_receiver.next().await;
            let msg = match msg {
                None => {
                    debug!("Websocket closed");
                    // TODO: Likely should be handled on the matchmaking end

                    // mm_sender
                    //     .send(MatchmakingRequest::Disconnected(connection.user_id.clone()))
                    //     .await
                    //     .expect("Failed to send leave queue");
                    break;
                }
                Some(msg) => msg,
            }
            .expect("Couldn't unwrap msg");

            if !msg.is_text() {
                warn!("Got non message of type {:?}, skipping", msg);
                continue;
            }

            // Deserialize request
            let body = msg.to_text().unwrap();
            debug!(body);
            let request: SocketRequest =
                serde_json::from_str(body).expect("Could not deserialize request.");
            let mut state = state.lock().await;
            state
                .user_handles
                .entry(address)
                .or_insert_with(|| Connection {
                    user_id: UserId(Uuid::new_v4()),
                    mm_to_ws: Channel::from(mpsc::channel(100)),
                });
            let connection = state.user_handles.get(&address).unwrap();

            debug!("Got message {:?}", &msg);

            let response = match request.request {
                ClientRequest::JoinQueue => {
                    let mm_request = MatchmakingRequest::JoinQueue(QueuedPlayer {
                        id: connection.user_id.clone(),
                        sender: connection.mm_to_ws.sender.clone(),
                    });
                    mm_sender.send(mm_request).await.unwrap();
                    ClientResponse::JoinedQueue
                }
                ClientRequest::Ping => ClientResponse::QueuePing { time_elapsed: 0u32 },
                ClientRequest::GetServer => ClientResponse::JoinServer {
                    server_ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                },
            };
            let response = serde_json::to_string(&response).expect("Could not serialize response.");
            ws_sender.send(Message::Text(response)).await.unwrap();
        }
    }
}
