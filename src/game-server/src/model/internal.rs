use common::{
    message::game_server::{ClientRequest, ClientResponse},
    model::{
        game::{Move, Outcome},
        messages::Id,
    },
};
use tokio::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

// Types
#[derive(Clone, Debug)]
pub struct PlayerHandle {
    pub id: Id,
    pub sender: Sender<ClientResponse>,
}

// Messages
#[derive(Debug, Clone)]
pub struct GameRequest {
    pub player: PlayerHandle,
    pub request: ClientRequest,
}
