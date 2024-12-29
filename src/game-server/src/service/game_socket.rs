use std::sync::Arc;

use crate::model::{
    external::{ClientRequest, ClientResponse},
    internal::GameRequest,
};
use axum::async_trait;
use common::websocket::{WebSocketState, WebsocketHandler};
use tokio::sync::Mutex;
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
