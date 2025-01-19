use std::{collections::HashMap, fmt::Debug, time::Duration};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt, TryStreamExt,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, warn};

use crate::model::messages::{Id, OpenSocketRequest, SocketRequest, SocketResponse};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Event<RQ, RS, RestRq, RestRs>
where
    RS: Serialize,
{
    SocketOpen {
        name: String,
        user_id: Id,
    },
    SocketSend {
        name: String,
        request: SocketRequest<RQ>,
    },
    SocketReceive {
        name: String,
        response: SocketResponse<RS>,
    },
    Post {
        name: String,
        endpoint: String,
        request: RestRq,
        response_code: u16,
        response: Option<RestRs>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct TestCase<RQ, RS, RestRq, RestRs>
where
    RS: Serialize,
{
    sequence: Vec<Event<RQ, RS, RestRq, RestRs>>,
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

// TODO: how to pass N generics?
impl<RQ, RS, RestRq, RestRs> TestCase<RQ, RS, RestRq, RestRs>
where
    RQ: Serialize,
    RS: Serialize + for<'de> Deserialize<'de> + Debug + PartialEq,
    RestRq: Serialize + Debug,
    RestRs: Serialize + for<'de> Deserialize<'de> + Debug + PartialEq,
{
    pub async fn run(&self, address_lookup: HashMap<String, ServerAddress>) {
        let timeout_len = Duration::from_millis(1500);
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

        // TODO: remove duplicate code (fr fr)
        for event in self.sequence.iter() {
            match event {
                Event::SocketOpen { name, user_id } => {
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

                    let body: String = json!(OpenSocketRequest { user_id: *user_id }).to_string();
                    timeout(timeout_len, write.send(Message::text(body)))
                        .await
                        .expect("Timeout sending message")
                        .expect("Failed to send message");
                }
                Event::SocketSend { name, request } => {
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
                Event::Post {
                    name,
                    endpoint,
                    request,
                    response_code,
                    response: expected_response,
                } => {
                    let handle = server_handles.get(name).expect("REST API not found");
                    let ServerHandle::RestApi(address) = handle else {
                        panic!("Expected REST API at {:?}, found websocket", name);
                    };

                    let url = address.to_owned() + endpoint;
                    debug!("POSTing {:?} to {:?}", request, url);
                    let response = Client::new()
                        .post(address.to_owned() + endpoint)
                        .json(&request)
                        .send()
                        .await
                        .expect("Request failed");
                    debug!("Status: {:?}", response.status());
                    assert_eq!(*response_code, response.status().as_u16());
                    match response.json::<RestRs>().await {
                        Ok(body) => {
                            debug!("Got response: {:?}", body)
                        }
                        Err(e) => {
                            debug!("Failed to deserialize response: {:?}", e);
                        }
                    }
                }
            }
        }
    }
}
