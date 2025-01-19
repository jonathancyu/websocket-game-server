use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        broadcast,
        mpsc::{self, Sender},
    },
    time,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    },
};
use tracing::{debug, error, info, warn};

use crate::{
    model::messages::{Id, OpenSocketRequest, SocketRequest, SocketResponse},
    utility::Channel,
};

#[derive(Clone)]
pub struct Connection<RS>
where
    RS: Clone,
{
    pub user_id: Id,
    pub to_socket: Channel<RS>,
}
impl<RS> Connection<RS>
where
    RS: Clone,
{
    pub fn new(user_id: Id, to_socket: Channel<RS>) -> Self {
        Connection { user_id, to_socket }
    }
}

#[async_trait]
pub trait WebsocketHandler<ExternalRQ, ExternalRS, InternalRQ>
where
    Self: 'static,
    ExternalRQ: for<'de> Deserialize<'de> + Send + 'static,
    ExternalRS: Clone + Send + Serialize + 'static,
    InternalRQ: Clone + Send + 'static,
{
    async fn listen(
        &mut self,
        address: String,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        mm_sender: Sender<InternalRQ>,
    ) {
        let ws_listener = TcpListener::bind(address.clone())
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to bind to {}: {}", address, e);
            });
        info!("Initialized ws listener: {}", address);
        loop {
            tokio::select! {
                result = ws_listener.accept() => {
                    match result {
                        Err(e) => {
                            error!("Failed to accept connection from {} with error: {}", address, e);
                        }
                        Ok((stream, address)) => {
                            tokio::spawn(Self::connection_thread(
                                stream,
                                address,
                                mm_sender.clone(),
                            ));
                        }
                    }
                },
                _ = shutdown_receiver.recv() => {
                    break;
                }
            };
        }
        info!("Exited ws listener");
    }

    // Thread to handle connection lifetime
    async fn connection_thread(
        stream: TcpStream,
        address: SocketAddr,
        mm_sender: Sender<InternalRQ>,
    ) {
        info!("New ws connection: {}", address);

        let stream = accept_async(stream).await.unwrap();
        let (mut ws_sender, mut ws_receiver) = stream.split();
        // The first message sent is always the user's id:
        // TODO: extract to fn, add shutdown listener
        let mut user_id = None;
        while user_id.is_none() {
            match ws_receiver.next().await {
                None => {
                    warn!("Connection closed before receiving user ID");
                    return;
                }
                Some(Err(e)) => {
                    warn!("Error receiving message: {}", e);
                    continue;
                }
                Some(Ok(msg)) => {
                    if let Ok(msg_str) = msg.to_text() {
                        match serde_json::from_str::<OpenSocketRequest>(msg_str) {
                            Ok(request) => {
                                user_id = Some(request.user_id);
                            }
                            Err(error) => {
                                warn!("Failed to parse identification message: {:?}", error);
                            }
                        }
                    } else {
                        warn!("Received non-text message");
                    }
                }
            }
        }

        let user_id = user_id.expect("UserID can't be null at this point, right..?");

        // Lookup user's Connection by user_id
        let (to_user_sender, mut to_user_receiver) = mpsc::channel::<ExternalRS>(100);

        debug!("Listening to {:?}", user_id);
        let mut interval = time::interval(Duration::from_millis(50));
        loop {
            let mut close_socket = false;
            tokio::select! {
                // Poll connection for any push messages
                _ = interval.tick() => {
                    let response = Self::handle_internal_message(user_id, &mut to_user_receiver).await;
                    if let Some(response) = response {
                        let response_body = serde_json::to_string(&response).expect("Could not serialize response.");
                        ws_sender.send(Message::Text(response_body)).await.unwrap();

                        // Drop connection according to criteria
                        close_socket |= Self::drop_after_send(response.body);
                    }
                }

                // Otherwise, handle incoming messages
                msg = ws_receiver.next() => {
                    debug!("msg: {:?}", msg);
                    let Some(msg) = msg else {
                        debug!("WS received empty message, TODO"); // TODO: what to do
                        break;
                    };
                    let msg = msg.expect("Couldn't unwrap message");

                    let result = Self::handle_external_message(
                        user_id,
                        msg,
                        to_user_sender.clone(),
                        mm_sender.clone()
                    ).await;

                    let Ok(response) = result else {
                        break
                    };

                    if let Some(response) = response {
                        let response_body = serde_json::to_string(&response).expect("Could not serialize response.");
                        ws_sender.send(Message::Text(response_body)).await.unwrap();

                        // Drop connection according to criteria
                        close_socket |= Self::drop_after_send(response.body);
                    }
                }
            };
            if close_socket {
                ws_sender
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Normal,
                        reason: "Decided to close after sending the previous message".into(),
                    })))
                    .await
                    .expect("Failed to close socket");
            }
        }
    }
    // Read message from connection, return immediate response
    // TODO: do we need the sender here if we're not responding immediately?
    async fn handle_external_message(
        user_id: Id,
        message: Message,
        to_user: Sender<ExternalRS>,
        to_internal: Sender<InternalRQ>,
    ) -> Result<Option<SocketResponse<ExternalRS>>, &'static str> {
        if !message.is_text() {
            return Err("Got non-text message :(");
        }

        // Deserialize request
        let body = message.to_text().unwrap();
        let request: SocketRequest<ExternalRQ> =
            serde_json::from_str(body).expect("Could not deserialize request.");

        let response = Self::respond_to_request(user_id, request.body, to_user, to_internal).await;

        Ok(response.map(|body| SocketResponse { user_id, body }))
    }

    // Read internal message to potentially forward to the user.
    async fn handle_internal_message(
        user_id: Id,
        receiver: &mut mpsc::Receiver<ExternalRS>,
    ) -> Option<SocketResponse<ExternalRS>> {
        // If message was sent, forward to user
        match receiver.try_recv() {
            Ok(body) => Some(SocketResponse { user_id, body }),
            Err(_) => None,
        }
    }

    // Criterion to drop connection. By default, always keep the connection alive.
    fn drop_after_send(_response: ExternalRS) -> bool {
        false
    }

    // Logic to handle a client's request
    async fn respond_to_request(
        _user_id: Id,
        _request: ExternalRQ,
        _to_user: Sender<ExternalRS>,
        _to_internal: Sender<InternalRQ>,
    ) -> Option<ExternalRS> {
        None
    }
}
