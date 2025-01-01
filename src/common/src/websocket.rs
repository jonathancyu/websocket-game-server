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
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    model::messages::{Id, SocketRequest, SocketResponse},
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
        // TODO: handshake, then resolve connection from state
        let _connection_id: Option<Id> = None;
        let user_id = Id(Uuid::new_v4());

        // Lookup user's Connection by user_id
        let connection = {
            // Acquire state mutex in limited scope so we don't deadlock for the lifetime of the
            // connection
            state
                .lock()
                .await
                .user_handles
                .entry(user_id.clone())
                .or_insert_with(|| {
                    Connection::new(user_id.clone(), Channel::from(mpsc::channel(100)))
                })
                .clone()
        };

        debug!("Listening to {:?}", user_id.clone());
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
                        close_socket |= Self::drop_after_send(response.message);
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
                        close_socket |= Self::drop_after_send(response.message);
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

        debug!("Got message {:?}", &message);
        let response =
            Self::respond_to_request(connection.clone(), request.request, mm_sender).await;

        Ok(response.map(|response| SocketResponse {
            user_id: connection.user_id,
            message: response,
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
            Ok(message) => Some(SocketResponse {
                user_id: connection.user_id,
                message,
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
        connection: Connection<ExternalRS>,
        request: ExternalRQ,
        mm_sender: Sender<InternalRQ>,
    ) -> Option<ExternalRS> {
        None
    }
}

pub mod test {
    use std::{fmt::Debug, time::Duration};

    use futures_util::{SinkExt, StreamExt};
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tokio::time::timeout;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    use crate::model::messages::{SocketRequest, SocketResponse};

    #[derive(Serialize, Deserialize)]
    #[serde(tag = "type")]
    enum Event<RQ, RS>
    where
        RS: Serialize,
    {
        Send { request: SocketRequest<RQ> },
        Receive { response: SocketResponse<RS> },
    }

    #[derive(Serialize, Deserialize)]
    pub struct TestCase<RQ, RS>
    where
        RS: Serialize,
    {
        sequence: Vec<Event<RQ, RS>>,
    }

    impl<RQ, RS> TestCase<RQ, RS>
    where
        RQ: Serialize,
        RS: Serialize + for<'de> Deserialize<'de> + Debug + PartialEq,
    {
        async fn run(&self, socket_address: String) {
            let (ws_stream, _) = connect_async(socket_address).await.unwrap();
            let (mut write, mut read) = ws_stream.split();
            let timeout_len = Duration::from_millis(500);
            for event in self.sequence.iter() {
                match event {
                    Event::Send { request } => {
                        let body: String = json!(request).to_string();
                        timeout(timeout_len, write.send(Message::text(body)))
                            .await
                            .expect("Timeout sending message")
                            .expect("Failed to send message");
                    }
                    Event::Receive { response: expected } => {
                        let body = timeout(timeout_len, read.next())
                            .await
                            .expect("Timeout reading response")
                            .expect("No message found")
                            .expect("Failed to read message");
                        let response: SocketResponse<RS> = serde_json::from_str(
                            body.to_text().expect("Failed to convert response to text"),
                        )
                        .expect("Failed to deserialize response");
                        assert_eq!(expected, &response);
                    }
                }
            }
        }
    }
}
