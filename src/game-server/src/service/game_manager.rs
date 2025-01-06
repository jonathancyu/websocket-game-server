use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use common::model::messages::{CreateGameRequest, CreateGameResponse, GetGameResponse, Id};
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver},
    Mutex,
};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use crate::model::internal::{GameRequest, Player};

use super::game_thread::{GameConfiguration, GameThread};

#[derive(Debug)]
struct GameHandle {
    id: Id,
    players: (Id, Id),
    to_game: mpsc::Sender<GameRequest>,
}

struct GameManagerState {
    games: HashMap<Id, Arc<Mutex<GameHandle>>>,
    player_assignment: HashMap<Id, Id>,
    shutdown_receiver: broadcast::Receiver<()>,
}
pub struct GameManager {}

impl GameManager {
    pub fn new() -> Self {
        GameManager {}
    }

    pub async fn listen(
        &self,
        address: String,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        from_socket: Receiver<GameRequest>,
    ) {
        let mut shutdown_receiver = shutdown_receiver.resubscribe();
        let state = Arc::new(Mutex::new(GameManagerState {
            games: HashMap::new(),
            player_assignment: HashMap::new(),
            shutdown_receiver: shutdown_receiver.resubscribe(),
        }));
        // Serve REST endpoint
        self.serve_rest_endpoint(address, state.clone(), shutdown_receiver.resubscribe())
            .await;

        // Spawn main thread to route game messages to game threads
        Self::game_router_thread(state.clone(), &mut shutdown_receiver, from_socket).await;
        // TODO: some sort of collector to cleanup dead games? or threads clean themselves
    }

    // Game logic loop
    async fn game_router_thread(
        state: Arc<Mutex<GameManagerState>>,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        mut from_socket: Receiver<GameRequest>,
    ) {
        loop {
            tokio::select! {
                result = from_socket.recv() => {
                    if let Some(request) = result {
                        Self::route_request(state.clone(), request).await;
                    }
                },
                _ = shutdown_receiver.recv() => {
                    break;
                }
            };
        }
    }

    async fn route_request(state: Arc<Mutex<GameManagerState>>, request: GameRequest) {
        let state = state.lock().await;
        let player_id = request.player.id.clone();
        match state.games.get(&player_id) {
            Some(game) => {
                let game = game.lock().await;
                game.to_game.send(request).await.unwrap_or_else(|_| {
                    panic!(
                        "Failed to route request to game for player {:?}, game {:?}",
                        player_id, game.id
                    )
                });
            }
            None => warn!("No game for player {:?}", player_id),
        };
    }

    // REST functions
    async fn serve_rest_endpoint(
        &self,
        address: String,
        state: Arc<Mutex<GameManagerState>>,
        mut shutdown_receiver: broadcast::Receiver<()>,
    ) {
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
            .with_graceful_shutdown(async move {
                shutdown_receiver
                    .recv()
                    .await
                    .expect("Failed to receive shutdown signal");
            })
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
        // Create game config
        let configuration = GameConfiguration {
            players: (player_1.clone(), player_2.clone()),
            games_to_win: request.games_to_win,
        };

        // Insert new game
        let id = Id::new();
        let (to_game, from_socket) = mpsc::channel(100); // TODO:
                                                         // what's the size here
        let game_handle = GameHandle {
            id: id.clone(),
            players: (player_1.clone(), player_2.clone()),
            to_game,
        };
        state
            .games
            .insert(id.clone(), Arc::new(Mutex::new(game_handle)));

        // Spawn game thread
        let thread_shutdown_receiver = state.shutdown_receiver.resubscribe();
        tokio::spawn(GameThread::thread_loop(
            configuration,
            thread_shutdown_receiver.resubscribe(),
            from_socket,
        ))
        .await
        .expect("Failed to spawn game thread");
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
