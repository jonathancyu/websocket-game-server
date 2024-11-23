use std::{
    net::{Ipv6Addr, SocketAddr},
    sync::{mpsc, Arc, Mutex},
};

use axum::{http::StatusCode, Json};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use server::{
    model::messages::{QueueMessage, Request, Response},
    service::matchmaking::MatchmakingService,
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Result},
};
use tracing::{debug, error, info};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    let _ = tracing::subscriber::set_global_default(subscriber);
    let (sender, receiver) = mpsc::channel::<QueueMessage>();
    let sender = Arc::new(Mutex::new(sender));
    let receiver = Arc::new(Mutex::new(receiver));
    // Spawn thread for matchmaking
    let matchmaker_handle: JoinHandle<()> = tokio::spawn(async move {
        matchmaking_thread(receiver.clone())
            .await
            .expect("Error in matchmaking thread");
    });

    // Listen for websocket connections
    let result = tokio::try_join!(websocket_listener(sender.clone()),);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("Thread exited with error: {}", err)
        }
    }

    matchmaker_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
}

async fn websocket_listener(
    sender: Arc<Mutex<mpsc::Sender<QueueMessage>>>,
) -> Result<(), &'static str> {
    let url = "0.0.0.0".to_owned();
    let queue_socket_port = "3001".to_owned();
    let addr = format!("{}:{}", url, queue_socket_port);
    let ws_listener = TcpListener::bind(addr.clone()).await.unwrap_or_else(|e| {
        panic!("Failed to bind to {}: {}", addr, e);
    });
    info!("Initialized ws listener: {}", addr);
    while let Ok((stream, address)) = ws_listener.accept().await {
        info!("Got something");
        tokio::spawn(handle_websocket_connection(stream, address, sender.clone()));
    }
    info!("Exited ws listener");

    Ok(())
}

async fn matchmaking_thread(
    receiver: Arc<Mutex<mpsc::Receiver<QueueMessage>>>,
) -> Result<(), &'static str> {
    let mut service = MatchmakingService::new();
    let receiver = receiver.lock().unwrap();
    info!("Initialized matchmaking service");
    while let Ok(message) = receiver.recv() {
        print!("got {:?}", message);
        let _ = service.add_user(message);
    }
    Ok(())
}

async fn handle_websocket_connection(
    stream: TcpStream,
    address: SocketAddr,
    sender: Arc<Mutex<mpsc::Sender<QueueMessage>>>,
) {
    info!("New ws connection: {}", address);
    // TODO: implement with protobuf (prost)
    let ws_stream = accept_async(stream).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    while let Some(msg) = ws_receiver.next().await {
        let msg: Message = msg.unwrap();
        if msg.is_text() {
            debug!("Got message {:?}", &msg);
            let body = msg.to_text().unwrap();
            let request = serde_json::from_str(body);
            let response: Response = match request {
                Ok(request) => match request {
                    Request::JoinQueue { user_id } => {
                        match sender.lock().unwrap().send(QueueMessage { user_id }) {
                            Ok(_) => Response::JoinedQueue,
                            Err(message) => Response::Error {
                                message: message.to_string(),
                            },
                        }
                    }
                    Request::GetServer { user_id: _ } => Response::JoinServer {
                        server_ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                    },
                },
                Err(_) => Response::Error {
                    message: "Could not deserialize request".to_string(),
                },
            };
            let response = serde_json::to_string(&response).expect("Could not SERIALIZE value :(");

            ws_sender.send(Message::Text(response)).await.unwrap();
        }
    }
}

async fn root() -> &'static str {
    "Hello world"
}

async fn join_queue(Json(payload): Json<QueueRequest>) -> (StatusCode, Json<String>) {
    // TODO: JWT to validate user
    let result = format!("hi {}", payload.user_id);
    (StatusCode::OK, Json(result))
}

#[derive(Deserialize)]
struct QueueRequest {
    pub user_id: u64,
}
