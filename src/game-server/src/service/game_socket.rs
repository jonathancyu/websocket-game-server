use std::sync::Arc;

use crate::model::{
    external::{ClientRequest, ClientResponse},
    internal::GameRequest,
};
use axum::async_trait;
use common::websocket::{WebSocketState, WebsocketHandler};
use tokio::sync::Mutex;
pub struct GameHandler {
    state: Arc<Mutex<WebSocketState<ClientResponse>>>,
}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, GameRequest> for GameHandler {
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
