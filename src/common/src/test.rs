use std::{collections::HashMap, fmt::Debug, time::Duration};

use futures_util::{
    stream::{self, SplitSink, SplitStream},
    SinkExt, StreamExt, TryStreamExt,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::model::messages::{SocketRequest, SocketResponse};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Event<RQ, RS>
where
    RS: Serialize,
{
    SocketSend {
        name: String,
        request: SocketRequest<RQ>,
    },
    SocketReceive {
        name: String,
        response: SocketResponse<RS>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct TestCase<RQ, RS>
where
    RS: Serialize,
{
    sequence: Vec<Event<RQ, RS>>,
}

pub enum ServerAddress {
    WebSocket(String),
    RestApi(String),
}
type SocketWriteHandle = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type SocketReadHandle = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
enum ServerHandle {
    WebSocket {
        read: SocketReadHandle,
        write: SocketWriteHandle,
    },
    RestApi(String),
}

impl<RQ, RS> TestCase<RQ, RS>
where
    RQ: Serialize,
    RS: Serialize + for<'de> Deserialize<'de> + Debug + PartialEq,
{
    pub async fn run(&self, address_lookup: HashMap<String, ServerAddress>) {
        let timeout_len = Duration::from_millis(500);
        let mut server_handles = HashMap::new();
        for (id, address) in address_lookup {
            server_handles.insert(
                id,
                match address {
                    ServerAddress::WebSocket(address) => {
                        let (ws_stream, _) = connect_async(address)
                            .await
                            .expect("Failed to establish socket");
                        let (write, read) = ws_stream.split();
                        ServerHandle::WebSocket { write, read }
                    }
                    ServerAddress::RestApi(address) => ServerHandle::RestApi(address),
                },
            );
        }

        for event in self.sequence.iter() {
            match event {
                Event::SocketSend { name, request } => {
                    // TODO: how to dedupe?
                    let handle = server_handles.get_mut(name).expect("Send socket not found");
                    assert!(matches!(
                        handle,
                        ServerHandle::WebSocket { write: _, read: _ }
                    ));
                    let ServerHandle::WebSocket {
                        ref mut write,
                        ref mut read,
                    } = handle
                    else {
                        panic!("Expected WebSocket handle at {:}", name);
                    };

                    // TODO: UNLESS it's a ping message
                    if let Ok(msg) = timeout(timeout_len, read.try_next()).await {
                        panic!("Expected no incoming message, got {:?}", msg);
                    }

                    let body: String = json!(request).to_string();
                    timeout(timeout_len, write.send(Message::text(body)))
                        .await
                        .expect("Timeout sending message")
                        .expect("Failed to send message");
                }
                Event::SocketReceive {
                    name,
                    response: expected,
                } => {
                    let handle = server_handles.get_mut(name).expect("Send socket not found");
                    assert!(matches!(
                        handle,
                        ServerHandle::WebSocket { write: _, read: _ }
                    ));
                    let ServerHandle::WebSocket {
                        write: _,
                        ref mut read,
                    } = handle
                    else {
                        panic!("Expected WebSocket handle at {:}", name);
                    };
                    let body = timeout(timeout_len, read.next())
                        .await
                        .unwrap_or_else(|e| {
                            panic!("Timeout (error {:?}) waiting for {:?}", e, expected)
                        })
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
