use common::model::messages::Id;
use serde::{Deserialize, Serialize};

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
pub struct CreateGameRequest {
    game_id: Id,
    players: Vec<Id>,
}

#[derive(Serialize)]
pub struct CreateGameResponse {
    game_id: Id,
}
