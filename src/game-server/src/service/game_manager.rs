use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use common::{
    model::messages::{CreateGameRequest, CreateGameResponse, GetGameRequest, GetGameResponse, Id},
    utility::shutdown_signal,
};
use tokio::sync::{broadcast, mpsc::Receiver, Mutex};
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

use crate::model::internal::{GameRequest, Player};

#[derive(Debug, Clone)]
struct Game {
    id: Id,
    players: (Id, Id),
}

struct GameManagerState {
    pub games: HashMap<Id, Arc<Mutex<Game>>>,
    pub player_assignment: HashMap<Id, Id>,
}
pub struct GameManager {}

impl GameManager {
    pub fn new() -> Self {
        GameManager {}
    }

    pub async fn listen(
        &mut self,
        address: String,
        from_socket: Arc<Mutex<Receiver<GameRequest>>>,
    ) {
        let state = Arc::new(Mutex::new(GameManagerState {
            games: HashMap::new(),
            player_assignment: HashMap::new(),
        }));
        // Serve REST endpoint
        self.serve_rest_endpoint(address, state).await;

        // Spawn game handler
    }

    async fn serve_rest_endpoint(&self, address: String, state: Arc<Mutex<GameManagerState>>) {
        let app: Router = Router::new()
            .layer(TraceLayer::new_for_http())
            .route("/", get(Self::root))
            .route("/create_game", post(Self::create_game))
            .route("/game/{game_id}", get(Self::get_game))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind(address.clone())
            .await
            .unwrap();
        info!("Game manager listening on {}", address);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();
    }

    async fn root() -> &'static str {
        "Hello, World!"
    }

    async fn create_game(
        State(state): State<Arc<Mutex<GameManagerState>>>,
        Json(request): Json<CreateGameRequest>,
    ) -> Response {
        // TODO:
        let mut state = state.lock().await;
        // Unpack player IDs
        let [player_1, player_2] = request.players.as_slice() else {
            panic!("Expected 2 player IDs")
        };
        // Check if players are already in a game
        if state.player_assignment.contains_key(player_1)
            || state.player_assignment.contains_key(player_2)
        {
            return (StatusCode::CONFLICT, "A player is already in a game").into_response();
        }

        // Insert new game
        let id = Id::new();
        let game = Game {
            id: id.clone(),
            players: (player_1.clone(), player_2.clone()),
        };
        state.games.insert(id.clone(), Arc::new(Mutex::new(game)));

        (
            StatusCode::CREATED,
            Json(CreateGameResponse { game_id: id }),
        )
            .into_response()
    }

    async fn get_game(
        Path(game_id): Path<Id>,
        State(state): State<Arc<Mutex<GameManagerState>>>,
    ) -> Response {
        let state = state.lock().await;
        if !state.games.contains_key(&game_id) {
            return StatusCode::NOT_FOUND.into_response();
        }
        let game = state.games.get(&game_id).unwrap().lock().await;

        (
            StatusCode::OK,
            Json(GetGameResponse {
                game_id,
                players: game.players.clone(),
            }),
        )
            .into_response()
    }
}

impl Default for GameManager {
    fn default() -> Self {
        Self::new()
    }
}
