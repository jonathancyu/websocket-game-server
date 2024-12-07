use std::{net::Ipv6Addr, sync::Arc};

use axum::async_trait;
use common::{
    model::messages::{SocketRequest, SocketResponse, UserId},
    websocket::{Connection, WebSocketState, WebsocketHandler},
};
use tokio::sync::{mpsc::Sender, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tracing::debug;
use uuid::Uuid;

use crate::model::messages::{
    ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse, Player,
};

pub struct QueueSocket {
    state: Arc<Mutex<WebSocketState<MatchmakingResponse>>>,
}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse>
    for QueueSocket
{
    fn get_state(&self) -> Arc<Mutex<WebSocketState<MatchmakingResponse>>> {
        self.state.clone()
    }

    async fn handle_internal_message(
        connection: Connection<MatchmakingResponse>,
    ) -> Option<SocketResponse<ClientResponse>> {
        // See if MM sent any messages
        let message = connection.to_socket.receiver.lock().await.recv().await?;

        // If message was sent, forward to user
        Some(SocketResponse {
            user_id: connection.user_id,
            message: match message {
                MatchmakingResponse::QueueJoined => ClientResponse::JoinedQueue,
                MatchmakingResponse::MatchFound(game) => ClientResponse::MatchFound {
                    game_id: game.id,
                    server_address: game.server_address,
                },
            },
        })
    }

    async fn handle_external_message(
        connection: Connection<MatchmakingResponse>,
        message: Message,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> Result<SocketResponse<ClientResponse>, &'static str> {
        if !message.is_text() {
            return Err("Got non-text message :(");
        }

        // Deserialize request
        let body = message.to_text().unwrap();
        debug!(body);
        let request: SocketRequest<ClientRequest> =
            serde_json::from_str(body).expect("Could not deserialize request.");
        // If client provided user_id, use it. Otherwise give them a new one.
        let user_id = match request.user_id {
            Some(user_id) => user_id,
            None => UserId(Uuid::new_v4()),
        };

        debug!("Got message {:?}", &message);
        Ok(SocketResponse {
            user_id,
            message: QueueSocket::respond_to_request(connection, request.request, mm_sender)
                .await,
        })
    }

    async fn respond_to_request(
        connection: Connection<MatchmakingResponse>,
        request: ClientRequest,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> ClientResponse {
        match request {
            ClientRequest::JoinQueue => {
                let mm_request = MatchmakingRequest::JoinQueue(Player {
                    id: connection.user_id.clone(),
                    sender: connection.to_socket.sender.clone(),
                });
                mm_sender.send(mm_request).await.unwrap();
                ClientResponse::AckJoinQueue
            }
            ClientRequest::Ping => ClientResponse::QueuePing { time_elapsed: 0u32 },
            ClientRequest::GetServer => ClientResponse::JoinServer {
                server_ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            },
        }
    }
}

impl QueueSocket {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WebSocketState::new())),
        }
    }
}

impl Default for QueueSocket {
    fn default() -> Self {
        Self::new()
    }
}
