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

pub struct Game {
    pub id: Id,
    pub player1: Player,
    pub player2: Player,
    pub server_address: Url,
}

struct MatchmakingServiceState {
    pub config: MatchmakingConfig,
    pub queue: VecDeque<Player>,
    pub users_in_queue: HashSet<Id>,
}

impl MatchmakingServiceState {
    pub fn add_user(&mut self, player: Player) {
        let user_id = player.clone().id;
        if self.users_in_queue.contains(&user_id) {
            warn!("User {:?} was already in the queue", user_id);
            return;
        }
        info!("Adding user {:?} to queue", user_id);
        self.queue.push_back(player);
        self.users_in_queue.insert(user_id);
    }
}

pub struct MatchmakingService {}

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

// TODO: this is a controller. Separate threads into their own "services"? 🤔
impl MatchmakingService {
    // TODO: How can we reduce the size of this state?
    async fn read_queue(state: Arc<Mutex<MatchmakingServiceState>>) -> Result<()> {
        let mut state = state.lock().await;
        let mut unmatched_players: VecDeque<Player> = VecDeque::new();
        let mut matches: Vec<(Player, Player)> = vec![];
        while let Some(player) = state.queue.pop_front() {
            if let Some(enemy) = unmatched_players.pop_front() {
                info!("Matched {:?} and {:?}", player.id, enemy.id);
                matches.push((player, enemy));
                continue;
            }
            unmatched_players.push_back(player);
        }
        for (player1, player2) in matches.iter() {
            state.users_in_queue.remove(&player1.id);
            state.users_in_queue.remove(&player2.id);

            // Create game
            let response = Self::create_game(&state.config, (player1.id, player2.id)).await?;

            // Notify players
            let message = ClientResponse::MatchFound {
                game_id: response.game_id,
                server_address: response.address,
            };
            player1.sender.send(message.clone()).await?;
            player2.sender.send(message.clone()).await?;
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
        let connection = Connection::open(db_path)?;
        connection.execute(
            "INSERT INTO match_results (id, player_1_score, player_2_score) VALUES (?1, ?2, ?3)",
            (
                &request.game_id.to_string(),
                request.games_won.0,
                request.games_won.1,
            ),
        )?;

        // Update ELO
        // TODO: impl

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

    async fn handle_message(
        state: Arc<Mutex<MatchmakingServiceState>>,
        message: Option<MatchmakingRequest>,
    ) {
        let mut state = state.lock().await;
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
                state.add_user(player);
                let result = sender.send(ClientResponse::JoinedQueue).await;
                if let Err(err) = result {
                    error!("Got error when sending MatchmakingResponse: {}", err);
                }
            }
            MatchmakingRequest::LeaveQueue(user_id) => match state.users_in_queue.get(&user_id) {
                Some(_) => {
                    let position = state
                        .queue
                        .iter()
                        .enumerate()
                        .find(|(_, user)| user.id == user_id);
                    if let Some((position, user)) = position {
                        info!("Removing user {:?} from queue", user.id);
                        state.queue.remove(position);
                    } else {
                        warn!(
                            "User {:?} was in users_in_queue but not in actual queue",
                            user_id
                        );
                    }
                }
                None => warn!("User {:?} not in queue", user_id),
            },
        };
    }
}

impl Default for MatchmakingService {
    fn default() -> Self {
        Self::new()
    }
}
