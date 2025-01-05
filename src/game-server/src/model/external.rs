use serde::{Deserialize, Serialize};

use super::internal::{Move, Result, RoundResult};

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
    RoundResult(RoundResult),
    MatchResult { result: Result, wins: u8, total: u8 },
}
