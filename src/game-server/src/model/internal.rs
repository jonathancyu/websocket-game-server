use common::{
    message::game_server::{ClientRequest, ClientResponse},
    model::messages::Id,
};
use tokio::sync::mpsc::Sender;

// Types
/// Handle used to send responses back to a connected player.
#[derive(Clone, Debug)]
pub struct PlayerHandle {
    pub id: Id,
    pub sender: Sender<ClientResponse>,
}

// Messages
/// A request from a player, tagged with the handle it arrived on.
#[derive(Debug, Clone)]
pub struct GameRequest {
    pub player: PlayerHandle,
    pub request: ClientRequest,
}
