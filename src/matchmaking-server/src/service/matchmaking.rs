use core::error;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use common::{
    elo,
    model::messages::{CreateGameRequest, CreateGameResponse, Id, PostGameResultsRequest},
    reqwest::{Client, Url},
};
use rusqlite::Connection;
use tokio::{
    sync::{broadcast, mpsc::Receiver, Mutex},
    task::JoinHandle,
};
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

use crate::{
    entrypoint::MatchmakingConfig,
    model::messages::{ClientResponse, MatchmakingRequest, Player},
};

/// A player in the matchmaking queue with their ELO rating
#[derive(Clone, Debug)]
struct QueuedPlayer {
    player: Player,
    elo: i32,
}

pub struct Game {
    pub id: Id,
    pub player1: Player,
    pub player2: Player,
    pub server_address: Url,
}

struct MatchmakingServiceState {
    pub config: MatchmakingConfig,
    pub queue: VecDeque<QueuedPlayer>,
    pub users_in_queue: HashSet<Id>,
}

impl MatchmakingServiceState {
    pub fn add_user(&mut self, queued_player: QueuedPlayer) {
        let user_id = queued_player.player.id;
        if self.users_in_queue.contains(&user_id) {
            warn!("User {:?} was already in the queue", user_id);
            return;
        }
        info!("Adding user {:?} to queue with ELO {}", user_id, queued_player.elo);
        self.queue.push_back(queued_player);
        self.users_in_queue.insert(user_id);
    }
}

pub struct MatchmakingService {}

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

    // TODO: this is a controller. Separate threads into their own "services"? 🤔
impl MatchmakingService {
    // ELO matchmaking threshold - players within this rating difference can be matched
    const ELO_MATCH_THRESHOLD: i32 = 200;
    // Maximum ELO difference to consider (expands if no matches found)
    const MAX_ELO_DIFF: i32 = 500;

    // TODO: How can we reduce the size of this state?
    async fn read_queue(state: Arc<Mutex<MatchmakingServiceState>>) -> Result<()> {
        let mut state = state.lock().await;
        let mut unmatched_players: VecDeque<QueuedPlayer> = VecDeque::new();
        let mut matches: Vec<(QueuedPlayer, QueuedPlayer)> = vec![];

        // ELO-based matching: try to match players with similar ratings
        while let Some(player) = state.queue.pop_front() {
            // Find the best match (closest ELO rating within threshold)
            let mut best_match_idx: Option<usize> = None;
            let mut best_elo_diff = Self::MAX_ELO_DIFF;

            for (idx, candidate) in unmatched_players.iter().enumerate() {
                let elo_diff = (player.elo - candidate.elo).abs();
                if elo_diff <= Self::ELO_MATCH_THRESHOLD && elo_diff < best_elo_diff {
                    best_match_idx = Some(idx);
                    best_elo_diff = elo_diff;
                }
            }

            if let Some(idx) = best_match_idx {
                let matched_player = unmatched_players.remove(idx).unwrap();
                info!(
                    "Matched {:?} (ELO {}) and {:?} (ELO {}) with ELO diff {}",
                    player.player.id, player.elo,
                    matched_player.player.id, matched_player.elo,
                    best_elo_diff
                );
                matches.push((player, matched_player));
            } else {
                // No good match found, add to unmatched queue
                unmatched_players.push_back(player);
            }
        }

        for (player1, player2) in matches.iter() {
            state.users_in_queue.remove(&player1.player.id);
            state.users_in_queue.remove(&player2.player.id);

            // Create game
            let response = Self::create_game(&state.config, (player1.player.id, player2.player.id)).await?;

            // Notify players
            let message = ClientResponse::MatchFound {
                game_id: response.game_id,
                server_address: response.address,
            };
            player1.player.sender.send(message.clone()).await?;
            player2.player.sender.send(message.clone()).await?;
        }
        state.queue = unmatched_players;

        Ok(())
    }

    pub fn new() -> Self {
        MatchmakingService {}
    }

    pub async fn run(
        &self,
        config: MatchmakingConfig,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        ws_receiver: Arc<Mutex<Receiver<MatchmakingRequest>>>,
    ) {
        let rest_address = config.rest_address.clone(); // Copy rest_address address before moving config into
        let game_server_url =
            Url::parse(&config.game_server_url).expect("Failed to parse game server url");
        // state
        let state = Arc::new(Mutex::new(MatchmakingServiceState {
            config: config.clone(),
            queue: VecDeque::new(),
            users_in_queue: HashSet::new(),
        }));

        // Thread to poll and push messages back to the websocket service
        let forward_socket_shutdown_receiver = shutdown_receiver.resubscribe();
        let socket_state = state.clone();
        let forward_socket_handle = tokio::spawn(async move {
            Self::forward_socket_thread(socket_state, forward_socket_shutdown_receiver, ws_receiver)
                .await
        });

        // REST thread
        let rest_shutdown_receiver = shutdown_receiver.resubscribe();
        let rest_handle: JoinHandle<()> = tokio::spawn(async move {
            Self::rest_endpoint_thread(&rest_address, rest_shutdown_receiver, state).await
        });

        forward_socket_handle
            .await
            .expect("Socket listener exited non-gracefully");

        rest_handle
            .await
            .expect("REST endpoint exited non-gracefully");
    }

    async fn forward_socket_thread(
        state: Arc<Mutex<MatchmakingServiceState>>,
        mut shutdown_receiver: broadcast::Receiver<()>,
        ws_receiver: Arc<Mutex<Receiver<MatchmakingRequest>>>,
    ) {
        let mut receiver = ws_receiver.lock().await;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(50));

        info!("Initialized matchmaking service");
        loop {
            // Listen for queue messages / shutdown signal
            tokio::select! {
                _ = shutdown_receiver.recv() => {
                    break
                }
                message = receiver.recv() => {
                    Self::handle_message(state.clone(), message).await;
                }
                _ = interval.tick() => {
                    Self::read_queue(state.clone()).await.expect("Failed to read internal queue");
                }
            }
        }
        info!("Exiting matchmaking service");
    }

    async fn rest_endpoint_thread(
        address: &String,
        mut shutdown_receiver: broadcast::Receiver<()>,
        state: Arc<Mutex<MatchmakingServiceState>>,
    ) {
        let app: Router = Router::new()
            .layer(TraceLayer::new_for_http())
            .route("/", get(Self::root))
            .route("/game/result", post(Self::post_game_result))
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

    async fn post_game_result(
        State(state): State<Arc<Mutex<MatchmakingServiceState>>>,
        Json(request): Json<PostGameResultsRequest>,
    ) -> Response {
        let db_path = state.lock().await.config.db_url.clone();
        match Self::write_game_result_and_update_elo(db_path, request).await {
            Ok(_) => StatusCode::CREATED.into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    async fn write_game_result_and_update_elo(
        db_path: String,
        request: PostGameResultsRequest,
    ) -> Result<()> {
        // Insert results into db
        let connection = Connection::open(&db_path)?;
        connection.execute(
            "INSERT INTO match_results (id, player_1_score, player_2_score) VALUES (?1, ?2, ?3)",
            (
                &request.game_id.to_string(),
                request.games_won.0,
                request.games_won.1,
            ),
        )?;

        // Get or create players and their ELO ratings
        let player_1_id = request.players.0.to_string();
        let player_2_id = request.players.1.to_string();

        // Ensure players exist in the players table
        connection.execute(
            "INSERT OR IGNORE INTO players (id, elo) VALUES (?1, ?2)",
            (&player_1_id, elo::default_rating()),
        )?;
        connection.execute(
            "INSERT OR IGNORE INTO players (id, elo) VALUES (?1, ?2)",
            (&player_2_id, elo::default_rating()),
        )?;

        // Get current ELO ratings
        let player_1_elo: i32 = connection.query_row(
            "SELECT elo FROM players WHERE id = ?1",
            [&player_1_id],
            |row| row.get(0),
        )?;

        let player_2_elo: i32 = connection.query_row(
            "SELECT elo FROM players WHERE id = ?1",
            [&player_2_id],
            |row| row.get(0),
        )?;

        // Determine actual scores (1.0 for win, 0.5 for draw, 0.0 for loss)
        let (player_1_score, player_2_score) = match request.games_won {
            (p1_score, p2_score) if p1_score > p2_score => (1.0, 0.0),
            (p1_score, p2_score) if p1_score < p2_score => (0.0, 1.0),
            _ => (0.5, 0.5), // Draw
        };

        // Calculate new ELO ratings
        let new_player_1_elo = elo::calculate_new_rating(
            player_1_elo,
            player_2_elo,
            player_1_score,
            None,
        );
        let new_player_2_elo = elo::calculate_new_rating(
            player_2_elo,
            player_1_elo,
            player_2_score,
            None,
        );

        // Update ELO ratings in database
        connection.execute(
            "UPDATE players SET elo = ?1 WHERE id = ?2",
            (new_player_1_elo, &player_1_id),
        )?;
        connection.execute(
            "UPDATE players SET elo = ?1 WHERE id = ?2",
            (new_player_2_elo, &player_2_id),
        )?;

        info!(
            "Updated ELO: Player {}: {} -> {}, Player {}: {} -> {}",
            player_1_id, player_1_elo, new_player_1_elo,
            player_2_id, player_2_elo, new_player_2_elo
        );

        Ok(())
    }

    async fn create_game(
        config: &MatchmakingConfig,
        players: (Id, Id),
    ) -> Result<CreateGameResponse> {
        let game_id = &Id::new();
        let games_to_win = 1u8;
        // Create entry in database
        let connection = Connection::open(&config.db_url)?;
        connection.execute(
            "INSERT INTO match (
                id,
                player_1_id,
                player_2_id,
                games_to_win
            ) VALUES (
                ?1, ?2, ?3, ?4
            )",
            (
                game_id.to_string(),
                players.0.to_string(),
                players.1.to_string(),
                games_to_win,
            ),
        )?;

        // POST to game server to create a game threwd
        let request = CreateGameRequest {
            players: vec![players.0, players.1],
            games_to_win,
        };
        let url = Url::parse(&config.game_server_url)?.join("create_game")?;
        // TODO: retry logic?
        Ok(Client::new()
            .post(url)
            .json(&request)
            .send()
            .await?
            .json::<CreateGameResponse>()
            .await?)
    }

    async fn load_player_elo(db_path: &str, player_id: &Id) -> Result<i32> {
        let connection = Connection::open(db_path)?;
        let player_id_str = player_id.to_string();

        // Try to get existing ELO rating, or use default if player doesn't exist
        let elo: i32 = connection.query_row(
            "SELECT elo FROM players WHERE id = ?1",
            [&player_id_str],
            |row| row.get(0),
        ).unwrap_or_else(|_| {
            // Player doesn't exist, create them with default ELO
            let default_elo = elo::default_rating();
            if let Err(e) = connection.execute(
                "INSERT INTO players (id, elo) VALUES (?1, ?2)",
                (&player_id_str, default_elo),
            ) {
                warn!("Failed to create player {} in database: {}", player_id_str, e);
            }
            default_elo
        });

        Ok(elo)
    }

    async fn handle_message(
        state: Arc<Mutex<MatchmakingServiceState>>,
        message: Option<MatchmakingRequest>,
    ) {
        debug!("msg: {:?}", message);
        let Some(message) = message else {
            info!("Got empty message");
            return;
        };
        match message {
            MatchmakingRequest::JoinQueue(player) => {
                let sender = player.sender.clone();
                if sender.is_closed() {
                    warn!("Sender {:?} is closed!", player.id);
                }

                // Load ELO rating from database (release lock before async DB operation)
                let db_path = {
                    let state_guard = state.lock().await;
                    state_guard.config.db_url.clone()
                };
                let player_id = player.id;

                let elo = match Self::load_player_elo(&db_path, &player_id).await {
                    Ok(rating) => rating,
                    Err(e) => {
                        error!("Failed to load ELO for player {:?}: {}", player_id, e);
                        elo::default_rating() // Fallback to default
                    }
                };

                let mut state_guard = state.lock().await;
                let queued_player = QueuedPlayer {
                    player,
                    elo,
                };
                state_guard.add_user(queued_player);

                let result = sender.send(ClientResponse::JoinedQueue).await;
                if let Err(err) = result {
                    error!("Got error when sending MatchmakingResponse: {}", err);
                }
            }
            MatchmakingRequest::LeaveQueue(user_id) => {
                let mut state_guard = state.lock().await;
                match state_guard.users_in_queue.get(&user_id) {
                    Some(_) => {
                        let position = state_guard
                            .queue
                            .iter()
                            .enumerate()
                            .find(|(_, queued_player)| queued_player.player.id == user_id);
                        if let Some((position, _)) = position {
                            info!("Removing user {:?} from queue", user_id);
                            state_guard.queue.remove(position);
                            state_guard.users_in_queue.remove(&user_id);
                        } else {
                            warn!(
                                "User {:?} was in users_in_queue but not in actual queue",
                                user_id
                            );
                        }
                    }
                    None => warn!("User {:?} not in queue", user_id),
                }
            }
        };
    }
}

impl Default for MatchmakingService {
    fn default() -> Self {
        Self::new()
    }
}
