use std::collections::{HashMap, HashSet};

use common::{
    message::game_server::{ClientRequest, ClientResponse, RoundResultResponse},
    model::{
        game::{self, Move},
        messages::Id,
    },
};
use itertools::Itertools;
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver},
};
use tracing::{debug, info, warn};

use crate::model::internal::{GameRequest, PlayerHandle};

#[derive(Clone)]
pub struct GameConfiguration {
    pub players: (Id, Id),
    pub games_to_win: u8,
}

struct Player {
    pub id: Id,
    pub sender: mpsc::Sender<ClientResponse>,
    pub wins: u8,
}

impl Player {
    pub fn from(handle: PlayerHandle) -> Self {
        Player {
            id: handle.id,
            sender: handle.sender,
            wins: 0,
        }
    }
}

struct GameState {
    phase: GamePhase,
    configuration: GameConfiguration,
    rounds_played: u8,
    players: HashMap<Id, Player>,
}

impl GameState {
    pub fn new(configuration: GameConfiguration) -> Self {
        GameState {
            phase: GamePhase::WaitingForPlayers {
                connected: HashSet::new(),
            },
            configuration,
            rounds_played: 0,
            players: HashMap::new(),
        }
    }

    pub async fn update(&mut self, request: GameRequest) {
        let player_id = request.player.id;
        match self.phase {
            GamePhase::WaitingForPlayers { ref connected } => {
                let mut connected = connected.clone();
                match request.request {
                    ClientRequest::JoinGame => {
                        connected.insert(player_id);
                        let player = Player::from(request.player);
                        player
                            .sender
                            .send(ClientResponse::GameJoined)
                            .await
                            .expect("Failed to send client response");
                        self.players.insert(player_id, player);
                    }
                    _ => {
                        warn!("Got non-JoinGame message in WaitingForPlayers phase");
                        return;
                    }
                }
                if connected.len() < 2 {
                    self.phase = GamePhase::WaitingForPlayers { connected };
                    return;
                }

                // Players are ready, prompt for moves
                debug!("All players connected, notifying.");
                for player in self.players.values() {
                    player
                        .sender
                        .send(ClientResponse::PendingMove)
                        .await
                        .expect("Failed to send message to player");
                }
                self.phase = GamePhase::PendingMoves {
                    moves: HashMap::new(),
                };
            }
            GamePhase::PendingMoves { ref moves } => {
                // TODO: can we just modify moves as mut? pls
                let mut moves = moves.clone();
                let ClientRequest::Move { value } = request.request else {
                    warn!("Got non-Move message in PendingMoves phase");
                    return;
                };
                if moves.contains_key(&player_id) {
                    warn!(
                        "Player {} submitted a move but already has a move present.",
                        player_id
                    );
                    return;
                }

                // Apply player's move
                moves.insert(player_id, value);
                if moves.len() < 2 {
                    self.phase = GamePhase::PendingMoves { moves };
                    return;
                }

                // Evaluate game result
                let (player_1, player_2) = moves
                    .iter()
                    .collect_tuple()
                    .expect("Expected two player-move pairs");

                // Update self
                match Self::get_winner(player_1, player_2) {
                    Some(winner_id) => {
                        let winner = self.players.get_mut(&winner_id).expect("Winner not found");
                        winner.wins += 1;
                        self.rounds_played += 1;

                        let config = self.configuration.clone();
                        if winner.wins >= config.games_to_win {
                            self.notify_round_result(winner_id, moves).await;
                            self.notify_match_result(winner_id).await;
                            self.phase = GamePhase::Done;
                            return;
                        } else {
                            self.notify_round_result(winner_id, moves).await;
                        }
                    }
                    None => {
                        self.rounds_played += 1;
                    }
                };
                self.phase = GamePhase::PendingMoves {
                    moves: HashMap::new(),
                };
            }
            GamePhase::Done => {
                warn!(
                    "Got request {:?} even though game is in Done state",
                    request
                );
            }
        }
    }

    async fn notify_round_result(&self, winner: Id, moves: HashMap<Id, Move>) {
        for player in self.players.values() {
            let other_move = moves
                .iter()
                .find(|(id, _)| **id != player.id)
                .map(|(_, mv)| mv)
                .expect("Other player's move not found")
                .clone();

            player
                .sender
                .send(match player.id == winner {
                    true => ClientResponse::RoundResult(RoundResultResponse {
                        result: game::Outcome::Win,
                        other_move,
                    }),
                    false => ClientResponse::RoundResult(RoundResultResponse {
                        result: game::Outcome::Loss,
                        other_move,
                    }),
                })
                .await
                .expect("Failed to notify round result");
        }
    }
    async fn notify_match_result(&self, winner: Id) {
        for player in self.players.values() {
            let result = match player.id == winner {
                true => game::Outcome::Win,
                false => game::Outcome::Loss,
            };
            player
                .sender
                .send(ClientResponse::MatchResult {
                    result,
                    wins: player.wins,
                    total: self.rounds_played,
                })
                .await
                .expect("Failed to notify round result");
        }
    }

    fn get_winner(player_1: (&Id, &Move), player_2: (&Id, &Move)) -> Option<Id> {
        let (id1, move1) = player_1;
        let (id2, move2) = player_2;
        match move1.beats(move2) {
            Some(player1_wins) => {
                if player1_wins {
                    Some(*id1)
                } else {
                    Some(*id2)
                }
            }
            None => None,
        }
    }
}

#[derive(Debug)]
enum GamePhase {
    WaitingForPlayers { connected: HashSet<Id> },
    PendingMoves { moves: HashMap<Id, Move> },
    Done,
}

pub struct GameThread {}
impl GameThread {
    pub async fn thread_loop(
        configuration: GameConfiguration,
        shutdown_receiver: broadcast::Receiver<()>,
        mut from_socket: Receiver<GameRequest>,
    ) {
        debug!("Starting thread for game {:?}", configuration.players);
        let mut state = GameState::new(configuration.clone());
        let mut shutdown_receiver = shutdown_receiver.resubscribe();
        loop {
            tokio::select! {
                request = from_socket.recv() => {
                    if let Some(request) = request {
                        state.update(request).await;
                    }
                    if matches!(state.phase, GamePhase::Done) {
                        info!("Game over, exiting");
                        break;
                    }
                }
                _ = shutdown_receiver.recv() => {
                    break;
                }
            }
        }
    }
}
