use std::collections::{HashMap, HashSet};

use common::model::messages::Id;
use itertools::Itertools;
use tokio::sync::{broadcast, mpsc::Receiver};
use tracing::{debug, warn};

use crate::model::{
    external::{ClientRequest, ClientResponse},
    internal::{GameRequest, Move, Player},
};

pub struct GameConfiguration {
    players: HashMap<Id, Player>,
    games_to_win: u8,
}

struct GameState {
    phase: GamePhase,
    wins: (u8, u8),
    rounds_played: u8,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            phase: GamePhase::WaitingForPlayers {
                connected: HashSet::new(),
            },
            wins: (0, 0),
            rounds_played: 0,
        }
    }

    pub fn with_phase(&self, phase: GamePhase) -> Self {
        GameState {
            phase,
            wins: self.wins,
            rounds_played: self.rounds_played,
        }
    }
}

enum GamePhase {
    WaitingForPlayers { connected: HashSet<Id> },
    PendingMoves { moves: HashMap<Id, Move> },
    Done,
}

struct GameThread {}
impl GameThread {
    async fn game_thread(
        configuration: GameConfiguration,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        mut from_socket: Receiver<GameRequest>,
    ) {
        let mut state = GameState::new();
        loop {
            tokio::select! {
                request = from_socket.recv() => {
                    if let Some(request) = request {
                        state = Self::update(&configuration, state, request).await;
                    }
                    if matches!(state.phase, GamePhase::Done) {
                        break;
                    }
                }
                _ = shutdown_receiver.recv() => {
                    break;
                }
            }
        }
    }

    async fn update(
        configuration: &GameConfiguration,
        state: GameState,
        request: GameRequest,
    ) -> GameState {
        let player_id = request.player.id;
        match state.phase {
            GamePhase::WaitingForPlayers { ref connected } => {
                let mut connected = connected.clone();
                match request.request {
                    ClientRequest::JoinGame => {
                        connected.insert(player_id);
                    }
                    _ => {
                        warn!("Got non-JoinGame message in WaitingForPlayers phase");
                        return state;
                    }
                }
                if connected.len() == 2 {
                    // Players are ready, prompt for moves
                    debug!("All players connected, notifying.");
                    for player in configuration.players.values().into_iter() {
                        player.sender.send(ClientResponse::PendingMove).await;
                    }
                }
                return state.with_phase(GamePhase::WaitingForPlayers { connected });
            }
            GamePhase::PendingMoves { ref moves } => {
                let mut moves = moves.clone();
                let ClientRequest::Move { value } = request.request else {
                    warn!("Got non-Move message in PendingMoves phase");
                    return state;
                };
                if moves.contains_key(&player_id) {
                    warn!(
                        "Player {} submitted a move but already has a move present.",
                        player_id
                    );
                    return state;
                }

                // Apply player's move
                moves.insert(player_id, value);
                if moves.len() < 2 {
                    return state.with_phase(GamePhase::PendingMoves { moves });
                }

                // Evaluate game result
                let (player_1, player_2) = moves
                    .iter()
                    .collect_tuple()
                    .expect("Expected two player-move pairs");
                // Update state
                let (id_1, id_2) = (player_1.0, player_2.0);
                let state = match Self::get_winner(player_1, player_2) {
                    Some(winner) => {
                        let (mut w1, mut w2) = state.wins;
                        if &winner == id_1 {
                            w1 += 1;
                        } else if &winner == id_2 {
                            w2 += 1
                        } else {
                            todo!("This shouldn't compile");
                        }
                        GameState {
                            phase: GamePhase::PendingMoves {
                                moves: HashMap::new(),
                            },
                            wins: (w1, w2),
                            rounds_played: state.rounds_played + 1,
                        }
                    }
                    None => GameState {
                        phase: GamePhase::PendingMoves {
                            moves: HashMap::new(),
                        },
                        wins: state.wins,
                        rounds_played: state.rounds_played + 1,
                    },
                };
                return state;
            }
            GamePhase::Done => todo!(), // TODO: impl
        }
    }

    fn get_winner(player_1: (&Id, &Move), player_2: (&Id, &Move)) -> Option<Id> {
        let (id1, move1) = player_1;
        let (id2, move2) = player_2;
        match move1.beats(move2) {
            Some(player1_wins) => {
                if player1_wins {
                    Some(id1.clone())
                } else {
                    Some(id2.clone())
                }
            }
            None => None,
        }
    }
}
