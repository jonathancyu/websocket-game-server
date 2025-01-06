use common::model::messages::Id;
use tokio::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

use super::external::{ClientRequest, ClientResponse};

// Types
#[derive(Clone, Debug)]
pub struct PlayerHandle {
    pub id: Id,
    pub sender: Sender<ClientResponse>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Result {
    Win,
    Loss,
    Draw,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Move {
    Rock,
    Paper,
    Scissors,
}
impl Move {
    pub fn beats(&self, other: &Move) -> Option<bool> {
        if self == other {
            None
        } else {
            Some(matches!(
                (self, other),
                (Move::Rock, Move::Scissors)
                    | (Move::Scissors, Move::Paper)
                    | (Move::Paper, Move::Rock)
            ))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RoundResult {
    pub result: Result,
    pub other_move: Move,
}

// Messages
#[derive(Debug, Clone)]
pub struct GameRequest {
    pub player: PlayerHandle,
    pub request: ClientRequest,
}
