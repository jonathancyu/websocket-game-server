use std::{
    collections::{HashMap, HashSet},
    panic::PanicHookInfo,
};

use common::model::messages::Id;
use itertools::Itertools;
use tokio::sync::{broadcast, mpsc::Receiver};
use tracing::{debug, warn};

use crate::model::{
    external::{ClientRequest, ClientResponse},
    internal::{GameRequest, Move, Player},
};

#[derive(Clone)]
pub struct GameConfiguration {
    pub players: (Id, Id),
    pub games_to_win: u8,
}

struct GameState {
    phase: GamePhase,
    configuration: GameConfiguration,
    wins: (u8, u8),
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
            wins: (0, 0),
            rounds_played: 0,
            players: HashMap::new(),
        }
    }

    pub fn with_phase(&mut self, phase: GamePhase) -> &mut Self {
        self.phase = phase;
        self
    }

    pub async fn update(&mut self, request: GameRequest) {
        let player_id = request.player.id.clone();
        match self.phase {
            GamePhase::WaitingForPlayers { ref connected } => {
                let mut connected = connected.clone();
                match request.request {
                    ClientRequest::JoinGame => {
                        connected.insert(player_id.clone());
                        self.players.insert(player_id, request.player);
                    }
                    _ => {
                        warn!("Got non-JoinGame message in WaitingForPlayers phase");
                        return;
                    }
                }
                if connected.len() == 2 {
                    // Players are ready, prompt for moves
                    debug!("All players connected, notifying.");
                    for player in self.players.values().into_iter() {
                        player.sender.send(ClientResponse::PendingMove).await;
                    }
                }
                self.phase = GamePhase::WaitingForPlayers { connected };
            }
            GamePhase::PendingMoves { ref moves } => {
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
                let (id_1, id_2) = (player_1.0, player_2.0);
                match Self::get_winner(player_1, player_2) {
                    Some(winner) => {
                        let (mut w1, mut w2) = self.wins;
                        if &winner == id_1 {
                            w1 += 1;
                        } else if &winner == id_2 {
                            w2 += 1
                        } else {
                            todo!("This shouldn't compile");
                        }
                        self.phase = GamePhase::PendingMoves {
                            moves: HashMap::new(),
                        };
                        self.wins = (w1, w2);
                        self.rounds_played += 1;
                    }
                    None => {
                        self.rounds_played += 1;
                    }
                };
                self.phase = GamePhase::PendingMoves {
                    moves: HashMap::new(),
                };
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
