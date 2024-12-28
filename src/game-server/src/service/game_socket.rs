use std::{net::Ipv6Addr, sync::Arc};

use crate::model::messages::{
    ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse, Player,
};
use axum::async_trait;
use common::{
    model::messages::SocketResponse,
    websocket::{Connection, WebSocketState, WebsocketHandler},
};
use tokio::sync::{mpsc::Sender, Mutex};
pub struct GameHandler {
    state: Arc<Mutex<WebSocketState<MatchmakingResponse>>>,
}

#[async_trait]
impl WebsocketHandler<ClientRequest, ClientResponse, MatchmakingRequest, MatchmakingResponse>
    for GameHandler
{
    fn get_state(&self) -> Arc<Mutex<WebSocketState<MatchmakingResponse>>> {
        self.state.clone()
    }

    async fn handle_internal_message(
        connection: Connection<MatchmakingResponse>,
    ) -> Option<SocketResponse<ClientResponse>> {
        todo!()
    }

    async fn respond_to_request(
        connection: Connection<MatchmakingResponse>,
        request: ClientRequest,
        mm_sender: Sender<MatchmakingRequest>,
    ) -> Option<ClientResponse> {
        todo!()
    }

    fn drop_after_send(response: ClientResponse) -> bool {
        todo!()
    }
}
