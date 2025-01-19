use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        broadcast,
        mpsc::{self, Sender},
        Mutex,
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
use tracing::{debug, error, field::debug, info, warn};
use uuid::Uuid;

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

pub struct WebSocketState<T: Clone> {
    user_handles: HashMap<Id, Connection<T>>,
}
impl<T> WebSocketState<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        WebSocketState {
            user_handles: HashMap::new(),
        }
    }
}

impl<T> Default for WebSocketState<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
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
    fn get_state(&self) -> Arc<Mutex<WebSocketState<ExternalRS>>>;

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
                                self.get_state(),
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
        state: Arc<Mutex<WebSocketState<ExternalRS>>>,
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
        // TODO: how to make this guaranteed at compile time?
        let user_id = user_id.expect("UserID can't be null at this point, right..?");

        // Lookup user's Connection by user_id
        let connection = {
            // Acquire state mutex in limited scope so we don't deadlock for the lifetime of the
            // connection
            state
                .lock()
                .await
                .user_handles
                .entry(user_id)
                .or_insert_with(|| Connection::new(user_id, Channel::from(mpsc::channel(100))))
                .clone()
        };

        debug!("Listening to {:?}", user_id);
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            let mut close_socket = false;
            tokio::select! {
                // Poll connection for any push messages
                _ = interval.tick() => {
                    let response = Self::handle_internal_message(connection.clone()).await;
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
                        connection.clone(),
                        msg,
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
        connection: Connection<ExternalRS>,
        message: Message,
        mm_sender: Sender<InternalRQ>,
    ) -> Result<Option<SocketResponse<ExternalRS>>, &'static str> {
        if !message.is_text() {
            return Err("Got non-text message :(");
        }

        // Deserialize request
        let body = message.to_text().unwrap();
        let request: SocketRequest<ExternalRQ> =
            serde_json::from_str(body).expect("Could not deserialize request.");

        let response = Self::respond_to_request(connection.clone(), request.body, mm_sender).await;

        Ok(response.map(|body| SocketResponse {
            user_id: connection.user_id,
            body,
        }))
    }

    // Read internal message to potentially forward to the user.
    async fn handle_internal_message(
        connection: Connection<ExternalRS>,
    ) -> Option<SocketResponse<ExternalRS>> {
        // See if MM sent any messages
        let mut receiver = connection.to_socket.receiver.lock().await;

        // If message was sent, forward to user
        match receiver.try_recv() {
            Ok(body) => Some(SocketResponse {
                user_id: connection.user_id,
                body,
            }),
            Err(_) => None,
        }
    }

    // Criterion to drop connection. By default, always keep the connection alive.
    fn drop_after_send(_response: ExternalRS) -> bool {
        false
    }

    // Logic to handle a client's request
    async fn respond_to_request(
        _connection: Connection<ExternalRS>,
        _request: ExternalRQ,
        _internal_sender: Sender<InternalRQ>,
    ) -> Option<ExternalRS> {
        None
    }
}
