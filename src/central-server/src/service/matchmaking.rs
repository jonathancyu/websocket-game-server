use core::error;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use common::{
    model::messages::{CreateGameRequest, CreateGameResponse, Id},
    reqwest::{Client, Url},
};
use tokio::sync::{broadcast, mpsc::Receiver, Mutex};
use tracing::{debug, error, info, warn};

use crate::model::messages::{ClientResponse, MatchmakingRequest, Player};

pub struct Game {
    pub id: Id,
    pub player1: Player,
    pub player2: Player,
    pub server_address: Url,
}

pub struct MatchmakingService {
    game_server_url: Url,
    queue: VecDeque<Player>,
    users_in_queue: HashSet<Id>,
    games: Vec<Game>,
}

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

impl MatchmakingService {
    async fn read_queue(&mut self) -> Result<()> {
        // BUG: need to lock queue, couldn't we be inserting into it?
        let mut unmatched_players: VecDeque<Player> = VecDeque::new();
        let mut matches: Vec<(Player, Player)> = vec![];
        while let Some(player) = self.queue.pop_front() {
            if let Some(enemy) = unmatched_players.pop_front() {
                info!("Matched {:?} and {:?}", player.id, enemy.id);
                matches.push((player, enemy));
                continue;
            }
            unmatched_players.push_back(player);
        }
        for (player1, player2) in matches.iter() {
            self.users_in_queue.remove(&player1.id);
            self.users_in_queue.remove(&player2.id);

            // Create game
            let response = self.create_game(vec![player1.id, player2.id]).await?;

            // TODO: do we need to keep track of this? only thing we store in here is
            // the player's sender, and players will disconnect immediately anyways
            let game = Game {
                id: response.game_id,
                player1: player1.clone(),
                player2: player2.clone(),
                server_address: Url::parse(&response.address)
                    .expect("Failed to parse created game's URL"),
            };

            // Notify players
            let message = ClientResponse::MatchFound {
                game_id: response.game_id,
                server_address: response.address,
            };
            player1.sender.send(message.clone()).await?;
            player2.sender.send(message.clone()).await?;

            // Add game
            self.games.push(game);
        }
        self.queue = unmatched_players;

        Ok(())
    }

    pub fn new(game_server_url: Url) -> Self {
        Self {
            game_server_url,
            queue: VecDeque::new(),
            users_in_queue: HashSet::new(),
            games: Vec::new(),
        }
    }

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

    pub async fn listen(
        &mut self,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        ws_receiver: Arc<Mutex<Receiver<MatchmakingRequest>>>,
    ) -> Result<()> {
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
                    self.handle_message(message).await;
                }
                _ = interval.tick() => {
                    self.read_queue().await?;
                }
            }
        }
        info!("Exiting matchmaking service");
        Ok(())
    }

    async fn create_game(&self, players: Vec<Id>) -> Result<CreateGameResponse> {
        let request = CreateGameRequest {
            players,
            games_to_win: 3,
        };
        let url = self.game_server_url.join("create_game")?;
        // TODO: retry logic?
        Ok(Client::new()
            .post(url)
            .json(&request)
            .send()
            .await?
            .json::<CreateGameResponse>()
            .await?)
    }

    async fn handle_message(&mut self, message: Option<MatchmakingRequest>) {
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
                self.add_user(player);
                let result = sender.send(ClientResponse::JoinedQueue).await;
                if let Err(err) = result {
                    error!("Got error when sending MatchmakingResponse: {}", err);
                }
            }
            MatchmakingRequest::LeaveQueue(user_id) => match self.users_in_queue.get(&user_id) {
                Some(_) => {
                    let position = self
                        .queue
                        .iter()
                        .enumerate()
                        .find(|(_, user)| user.id == user_id);
                    if let Some((position, user)) = position {
                        info!("Removing user {:?} from queue", user.id);
                        self.queue.remove(position);
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
