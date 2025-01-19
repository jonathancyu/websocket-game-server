
use crate::model::{
    external::{ClientRequest, ClientResponse},
    internal::{GameRequest, PlayerHandle},
};
use async_trait::async_trait;
use common::{
    model::messages::Id,
    websocket::WebsocketHandler,
};
use tokio::sync::mpsc::Sender;
pub struct GameSocket {}
impl GameSocket {
    pub fn new() -> Self {
        Self {}
    }
}
impl Default for GameSocket {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, GameRequest> for GameSocket {
    async fn respond_to_request(
        user_id: Id,
        request: ClientRequest,
        to_user_sender: Sender<ClientResponse>,
        internal_sender: Sender<GameRequest>,
    ) -> Option<ClientResponse> {
        // Resolve player object and route to game manager
        let request = GameRequest {
            player: PlayerHandle {
                id: user_id,
                sender: to_user_sender.clone(),
            },
            request,
        };
        internal_sender
            .send(request)
            .await
            .expect("Failed to send internal message");
        None
    }

    fn drop_after_send(response: ClientResponse) -> bool {
        matches!(
            response,
            ClientResponse::MatchResult {
                result: _,
                wins: _,
                total: _,
            }
        )
    }
}
