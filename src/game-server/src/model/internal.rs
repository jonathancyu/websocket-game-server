use common::model::{
    game::{Move, Outcome},
    messages::Id,
};
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
pub struct RoundResultResponse {
    pub result: Outcome,
    pub other_move: Move,
}

// Messages
#[derive(Debug, Clone)]
pub struct GameRequest {
    pub player: PlayerHandle,
    pub request: ClientRequest,
}
