use common::model::messages::Id;
use tokio::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

// Types
#[derive(Debug, Clone)]
pub struct Player {
    pub id: Id,
    pub sender: Sender<GameRequest>,
}

#[derive(Serialize, Debug, Clone)]
pub enum Result {
    Win,
    Loss,
    Draw,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Move {
    Rock,
    Paper,
    Scissors,
}

#[derive(Serialize, Debug, Clone)]
pub struct RoundResult {
    pub result: Result,
    pub other_move: Move,
}

// Messages
#[derive(Debug, Clone)]
pub enum GameRequest {
    Connect(Player),
    Move { player: Id, value: Move },
    Disconnect(Player), // TODO: impl
}
