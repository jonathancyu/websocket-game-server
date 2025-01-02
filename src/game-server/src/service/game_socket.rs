use std::sync::Arc;

use crate::model::{
    external::{ClientRequest, ClientResponse},
    internal::{GameRequest, Player},
};
use axum::async_trait;
use common::websocket::{Connection, WebSocketState, WebsocketHandler};
use tokio::sync::{mpsc::Sender, Mutex};
pub struct GameSocket {
    state: Arc<Mutex<WebSocketState<ClientResponse>>>,
}
impl GameSocket {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WebSocketState::new())),
        }
    }
}
impl Default for GameSocket {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, GameRequest> for GameSocket {
    fn get_state(&self) -> Arc<Mutex<WebSocketState<ClientResponse>>> {
        self.state.clone()
    }

    async fn respond_to_request(
        connection: Connection<ClientResponse>,
        request: ClientRequest,
        internal_sender: Sender<GameRequest>,
    ) -> Option<ClientResponse> {
        match request {
            ClientRequest::JoinGame => {
                internal_sender
                    .send(GameRequest::Connect(Player {
                        id: connection.user_id.clone(),
                        sender: connection.to_socket.sender.clone(),
                    }))
                    .await
                    .expect("Failed to send internal message");
                None
            }
            ClientRequest::Move { r#move: _ } => None,
        }
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
