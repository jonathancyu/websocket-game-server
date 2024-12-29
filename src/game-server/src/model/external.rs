use serde::{Deserialize, Serialize};

use super::internal::{Move, Result, RoundResult};

// Client types
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    JoinGame,
    Move(Move),
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub enum ClientResponse {
    GameJoined,
    PendingMove,
    RoundResult(RoundResult),
    MatchResult { result: Result, wins: u8, total: u8 },
}
