use std::{net::Ipv6Addr, sync::Arc};

use axum::async_trait;
use common::{
    model::messages::SocketResponse,
    websocket::{Connection, WebSocketState, WebsocketHandler},
};
use tokio::sync::{mpsc::Sender, Mutex};

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

    async fn respond_to_request(
        connection: Connection<MatchmakingResponse>,
        request: ClientRequest,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> Option<ClientResponse> {
        match request {
            ClientRequest::JoinQueue => {
                // Tell matchmaking to add user to the queue
                let mm_request = MatchmakingRequest::JoinQueue(Player {
                    id: connection.user_id.clone(),
                    sender: connection.to_socket.sender.clone(),
                });
                mm_sender.send(mm_request).await.unwrap();
                None // We
            }
            ClientRequest::Ping => Some(ClientResponse::QueuePing { time_elapsed: 0u32 }),
            ClientRequest::GetServer => Some(ClientResponse::JoinServer {
                server_ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            }),
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
