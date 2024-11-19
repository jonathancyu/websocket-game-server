use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

#[tokio::main]
async fn main() {
    let app: Router = Router::new()
        .route("/", get(root))
        .route("/join_queue", post(join_queue));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
