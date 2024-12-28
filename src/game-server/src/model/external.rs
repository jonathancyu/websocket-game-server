use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::internal::{Move, RoundResult};

#[derive(Serialize, Debug, Clone)]
enum Result {
    Win,
    Loss,
    Draw,
}

// Client types
#[derive(Deserialize)]
pub enum ClientRequest {
    JoinGame,
    Move(Move),
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub enum ClientResponse {
    RoundResult(RoundResult),
    MathResult(RoundResult),
}

// Matchmaking request/response
#[derive(Deserialize)]
pub struct MatchmakingRequest {
    game_id: Uuid,
}
