use std::net::SocketAddr;

use axum::{http::StatusCode, Json};
use futures_util::{
    future::{ok, ready},
    SinkExt, StreamExt, TryStreamExt,
};
use serde::Deserialize;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Result},
};
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Test");
    let url = "0.0.0.0".to_owned();
    let queue_socket_port = "3001".to_owned();
    // Websockets thread
    let ws_listener = TcpListener::bind(format!("{}:{}", url, queue_socket_port))
        .await
        .unwrap();
    while let Ok((stream, address)) = ws_listener.accept().await {
        tokio::spawn(handle_websocket_connection(stream, address));
    }

    // REST endpoints
    // let app: Router = Router::new()
    //     .route("/", get(root))
    //     .route("/join_queue", post(join_queue));
    // let listener = tokio::net::TcpListener::bind(url.to_owned() + ":3000")
    //     .await
    //     .unwrap();
    // axum::serve(listener, app).await.unwrap();
}

async fn handle_websocket_connection(stream: TcpStream, address: SocketAddr) {
    info!("New ws connection: {}", address);
    let ws_stream = accept_async(stream).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    while let Some(msg) = ws_receiver.next().await {
        let msg: Message = msg.unwrap();
        if msg.is_text() {
            debug!("Got message {:?}", &msg);
            let text = msg.to_text().unwrap();
            let response = Message::text(format!("Got {}", text));
            ws_sender.send(response).await.unwrap();
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
