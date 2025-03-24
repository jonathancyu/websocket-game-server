use common::{
    message::game_server::{ClientRequest, ClientResponse},
    model::messages::Id,
};
use tokio::sync::mpsc::Sender;


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
