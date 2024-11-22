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
use tokio::net::{TcpListener, TcpStream};
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
    let result = tokio::try_join!(
        websocket_listener(sender.clone()),
        matchmaking_thread(receiver.clone())
    );
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("Thread exited with error: {}", err)
        }
    }

    // Websockets thread

    // REST endpoints
    // let app: Router = Router::new()
    //     .route("/", get(root))
    //     .route("/join_queue", post(join_queue));
    // let listener = tokio::net::TcpListener::bind(url.to_owned() + ":3000")
    //     .await
    //     .unwrap();
    // axum::serve(listener, app).await.unwrap();
}

async fn websocket_listener(
    sender: Arc<Mutex<mpsc::Sender<QueueMessage>>>,
) -> Result<(), &'static str> {
    let url = "0.0.0.0".to_owned();
    let queue_socket_port = "3001".to_owned();
    let ws_listener = TcpListener::bind(format!("{}:{}", url, queue_socket_port))
        .await
        .unwrap();
    info!("Initialized ws listener");
    while let Ok((stream, address)) = ws_listener.accept().await {
        tokio::spawn(handle_websocket_connection(stream, address, sender.clone()));
    }

    Ok(())
}

async fn matchmaking_thread(
    receiver: Arc<Mutex<mpsc::Receiver<QueueMessage>>>,
) -> Result<(), &'static str> {
    let mut service = MatchmakingService::new();
    let receiver = receiver.lock().unwrap();
    info!("Initialized matchmaking service");
    while let Ok(message) = receiver.recv() {
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
