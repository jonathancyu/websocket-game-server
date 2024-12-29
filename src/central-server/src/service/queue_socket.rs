use std::{net::Ipv6Addr, sync::Arc};

use axum::async_trait;
use common::websocket::{Connection, WebSocketState, WebsocketHandler};
use tokio::sync::{mpsc::Sender, Mutex};
use tracing::debug;

use crate::model::messages::{ClientRequest, ClientResponse, MatchmakingRequest, Player};

pub struct QueueSocket {
    state: Arc<Mutex<WebSocketState<ClientResponse>>>,
}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, MatchmakingRequest> for QueueSocket {
    fn get_state(&self) -> Arc<Mutex<WebSocketState<ClientResponse>>> {
        self.state.clone()
    }

    async fn respond_to_request(
        connection: Connection<ClientResponse>,
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
                debug!("Send mm {:?}", mm_request);
                mm_sender.send(mm_request).await.unwrap();
                None
            }
            ClientRequest::Ping => Some(ClientResponse::QueuePing { time_elapsed: 0u32 }),
            ClientRequest::GetServer => Some(ClientResponse::JoinServer {
                server_ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            }),
        }
    }

    fn drop_after_send(response: ClientResponse) -> bool {
        matches!(
            response,
            ClientResponse::MatchFound {
                game_id: _,
                server_address: _,
            } | ClientResponse::JoinServer { server_ip: _ }
        )
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
