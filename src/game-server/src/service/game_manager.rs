use std::{collections::HashMap, sync::Arc};

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::{
    model::messages::{CreateGameRequest, CreateGameResponse, Id},
    utility::shutdown_signal,
};
use tokio::sync::{broadcast, Mutex};
use tracing::info;
use uuid::Uuid;

use crate::model::internal::Player;

#[derive(Debug, Clone)]
struct Game {
    id: Id,
    players: (Player, Player),
}

pub struct GameManager {
    games: HashMap<Id, Arc<Mutex<Game>>>,
}

/*
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
    let result = format!("hi {}", payload.user_id);
    (StatusCode::OK, Json(result))
}

#[derive(Deserialize)]
struct QueueRequest {
    pub user_id: u64,
}
*/

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            games: HashMap::new(),
        }
    }

    pub async fn listen(&mut self, url: String) {
        let app: Router = Router::new()
            .route("/", get(Self::root))
            .route("/create_game", post(Self::create_game));
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        info!("Listening on {}", url);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();
    }
    async fn root() -> &'static str {
        "Hello, World!"
    }

    async fn create_game(
        Json(_request): Json<CreateGameRequest>,
    ) -> (StatusCode, Json<CreateGameResponse>) {
        (
            StatusCode::CREATED,
            Json(CreateGameResponse {
                game_id: Id(Uuid::new_v4()),
            }),
        )
    }
}

impl Default for GameManager {
    fn default() -> Self {
        Self::new()
    }
}
