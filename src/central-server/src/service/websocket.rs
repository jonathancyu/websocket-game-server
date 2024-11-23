use std::{
    net::{Ipv6Addr, SocketAddr},
    sync::Arc,
};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, info};

use crate::model::messages::{
    ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse, QueuedPlayer,
};

pub struct WebSocketHandler {}
impl WebSocketHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn listen(
        &self,
        mm_sender: Arc<Mutex<Sender<MatchmakingRequest>>>,
        mm_listener: Arc<Mutex<Receiver<MatchmakingResponse>>>,
    ) {
        let url = "0.0.0.0".to_owned();
        let queue_socket_port = "3001".to_owned();
        let addr = format!("{}:{}", url, queue_socket_port);
        let ws_listener = TcpListener::bind(addr.clone()).await.unwrap_or_else(|e| {
            panic!("Failed to bind to {}: {}", addr, e);
        });
        info!("Initialized ws listener: {}", addr);
        while let Ok((stream, address)) = ws_listener.accept().await {
            info!("Got something");
            tokio::spawn(WebSocketHandler::handle_connection(
                stream,
                address,
                mm_sender.clone(),
            ));
        }
        info!("Exited ws listener");
    }

    async fn handle_connection(
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Arc<Mutex<mpsc::Sender<MatchmakingRequest>>>,
    ) {
        info!("New ws connection: {}", address);
        // TODO: implement with protobuf (prost)
        let stream = accept_async(stream).await.unwrap();
        let (mut ws_sender, mut ws_receiver) = stream.split();
        while let Some(msg) = ws_receiver.next().await {
            let msg: Message = msg.unwrap();
            if msg.is_text() {
                debug!("Got message {:?}", &msg);
                // Deserialize request
                let body = msg.to_text().unwrap();
                let request = serde_json::from_str(body);
                let (handle_sender, _handle_receiver) = mpsc::channel::<MatchmakingResponse>(100);

                // NOTE: there must be a better way
                let response: ClientResponse = match request {
                    Ok(request) => match request {
                        ClientRequest::JoinQueue { user_id } => {
                            let mm_request = MatchmakingRequest::JoinQueue(QueuedPlayer {
                                id: user_id,
                                sender: handle_sender,
                            });
                            let mm_sender = mm_sender.lock().await;

                            match mm_sender.send(mm_request).await {
                                Ok(_) => ClientResponse::JoinedQueue,
                                Err(message) => ClientResponse::Error {
                                    message: message.to_string(),
                                },
                            }
                        }
                        ClientRequest::GetServer { user_id: _ } => ClientResponse::JoinServer {
                            server_ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                        },
                    },
                    Err(_) => ClientResponse::Error {
                        message: "Could not deserialize request".to_string(),
                    },
                };
                let response =
                    serde_json::to_string(&response).expect("Could not SERIALIZE value :(");

                ws_sender.send(Message::Text(response)).await.unwrap();
            }
        }
    }
}
