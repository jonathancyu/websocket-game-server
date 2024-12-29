use std::sync::Arc;

use crate::model::{
    external::{ClientRequest, ClientResponse},
    internal::{GameRequest, GameResponse},
};
use axum::async_trait;
use common::{
    model::messages::SocketResponse,
    websocket::{Connection, WebSocketState, WebsocketHandler},
};
use tokio::sync::{mpsc::Sender, Mutex};
use tracing::warn;
pub struct GameHandler {
    state: Arc<Mutex<WebSocketState<GameResponse>>>,
}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, GameRequest, GameResponse> for GameHandler {
    fn get_state(&self) -> Arc<Mutex<WebSocketState<GameResponse>>> {
        self.state.clone()
    }

    async fn handle_internal_message(
        connection: Connection<GameResponse>,
    ) -> Option<SocketResponse<ClientResponse>> {
        // See if MM sent any messages
        let mut receiver = connection.to_socket.receiver.lock().await;
        // If message was sent, forward to user
        // TODO: we're just forwarding lol
        match receiver.try_recv() {
            Ok(message) => Some(SocketResponse {
                user_id: connection.user_id,
                message: match message {
                    GameResponse::Connected => ClientResponse::GameJoined,
                    GameResponse::PendingMove => ClientResponse::PendingMove,
                    GameResponse::RoundResult(round_result) => {
                        ClientResponse::RoundResult(round_result)
                    }
                    GameResponse::MatchResult {
                        result,
                        wins,
                        total,
                    } => ClientResponse::MatchResult {
                        result,
                        wins,
                        total,
                    },
                },
            }),
            Err(e) => {
                warn!("Game socket internal recv error: {:?}", e);
                None
            }
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
