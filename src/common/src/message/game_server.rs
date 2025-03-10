use crate::model::game::{Move, Outcome};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RoundResultResponse {
    pub result: Outcome,
    pub other_move: Move,
}

// Client types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ClientRequest {
    JoinGame,
    Move { value: Move },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum ClientResponse {
    GameJoined,
    PendingMove,
    RoundResult(RoundResultResponse),
    MatchResult {
        result: Outcome,
        wins: u8,
        total: u8,
    },
}
