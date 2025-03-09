use std::net::Ipv6Addr;

use axum::async_trait;
use common::{model::messages::Id, websocket::WebsocketHandler};
use tokio::sync::mpsc::Sender;
use tracing::debug;

use crate::model::messages::{ClientRequest, ClientResponse, MatchmakingRequest, Player};

pub struct QueueSocket {}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, MatchmakingRequest> for QueueSocket {
    async fn respond_to_request(
        user_id: Id,
        request: ClientRequest,
        to_user_sender: Sender<ClientResponse>,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> Option<ClientResponse> {
        match request {
            ClientRequest::JoinQueue => {
                // Tell matchmaking to add user to the queue
                let mm_request = MatchmakingRequest::JoinQueue(Player {
                    id: user_id,
                    sender: to_user_sender.clone(),
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
        Self {}
    }
}

impl Default for QueueSocket {
    fn default() -> Self {
        Self::new()
    }
}
