use std::{collections::HashMap, fmt::Debug, fs, time::Duration};

use futures_util::{
    stream::{SplitSink, SplitStream},
    FutureExt, SinkExt, StreamExt, TryStreamExt,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{http::response, Message},
    MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, info};
use uuid::Uuid;

use crate::model::messages::{Id, OpenSocketRequest};

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
        request: RQ,
    },
    SocketReceive {
        name: String,
        response: RS,
        replace_uuids: Option<bool>,
    },
    Post {
        name: String,
        endpoint: String,
        request: RestRq,
        response_code: u16,
        response: Option<RestRs>,
        replace_uuids: Option<bool>,
    },
    Comment {
        text: String,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct DummyType {}

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

fn substitute_ids(text: &str, id: Id) -> String {
    // UUID regex pattern: 8-4-4-4-12 hex digits
    let uuid_regex =
        regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
            .expect("Failed to compile UUID regex");

    return uuid_regex.replace_all(text, id.to_string()).to_string();
}

// TODO: how to pass N generics?
impl<RQ, RS, RestRq, RestRs> TestCase<RQ, RS, RestRq, RestRs>
where
    RQ: Serialize + for<'de> Deserialize<'de>,
    RS: Serialize + for<'de> Deserialize<'de> + Debug + PartialEq,
    RestRq: Serialize + for<'de> Deserialize<'de> + Debug,
    RestRs: Serialize + for<'de> Deserialize<'de> + Debug + PartialEq,
{
    fn compare_with_uuid_replacement<T>(response_text: &str, expected: &T) -> T
    where
        T: Serialize + for<'de> Deserialize<'de> + Debug + PartialEq,
    {
        let id = Id::new();
        let replaced_response_text = substitute_ids(response_text, id);
        let response: T =
            serde_json::from_str(&replaced_response_text).expect("Failed to deserialize response");

        // Replace ids in expected
        let expected_json =
            serde_json::to_string(expected).expect("Failed to serialize expected response");
        let expected_json = substitute_ids(&expected_json, id);
        let expected: T = serde_json::from_str(&expected_json)
            .expect("Failed to deserialize replaced expected response");

        assert_eq!(expected, response);
        response
    }

    pub fn load(file_path: String, replacements: Vec<(impl ToString, impl ToString)>) -> Self {
        let mut text = fs::read_to_string(file_path).expect("Unable to read file");
        for (from, to) in replacements {
            let from = &format!("${{{}}}", from.to_string());
            text = text.replace(from, &to.to_string());
        }
        let test_case: Self = serde_json::from_str(&text).expect("Could not parse test case");
        test_case
    }

    pub async fn run(&self, address_lookup: HashMap<String, ServerAddress>) {
        let timeout_len = Duration::from_millis(250);
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
                    let body: String = json!(OpenSocketRequest { user_id: *user_id }).to_string();
                    let handle = server_handles.get_mut(name).expect("Send socket not found");
                    Self::socket_send(timeout_len, handle, body).await;
                }
                Event::SocketSend { name, request } => {
                    let body: String = json!(request).to_string();
                    let handle = server_handles.get_mut(name).expect("Send socket not found");
                    Self::socket_send(timeout_len, handle, body).await;
                }
                Event::SocketReceive {
                    name,
                    response: expected,
                    replace_uuids,
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
                    let response_text = body
                        .to_text()
                        .expect("Failed to convert response to text")
                        .to_string();

                    // If replace_uuids is true, replace all UUIDs with a fixed value
                    if replace_uuids.unwrap_or(false) {
                        Self::compare_with_uuid_replacement::<RS>(&response_text, expected);
                    } else {
                        let response: RS = serde_json::from_str(&response_text)
                            .expect("Failed to deserialize response");
                        assert_eq!(expected, &response);
                    }
                }
                Event::Post {
                    name,
                    endpoint,
                    request,
                    response_code,
                    response: expected_response,
                    replace_uuids,
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
                    debug!("expected_response: {:?}", expected_response);

                    if let Some(expected) = expected_response {
                        debug!("Comparing response with expected");
                        let response_text =
                            response.text().await.expect("Failed to get response text");
                        if replace_uuids.unwrap_or(false) {
                            info!("Replacing UUIDs in response ALKJSD");
                            Self::compare_with_uuid_replacement::<RestRs>(&response_text, expected);
                        } else {
                            let response = serde_json::from_str::<RestRs>(&response_text)
                                .expect("Failed to parse response");
                            assert_eq!(expected, &response);
                        }
                    }
                }
                Event::Comment { text } => {
                    info!("Comment: {:}", text)
                }
            }
        }
    }

    async fn socket_send(timeout_len: Duration, handle: &mut ServerHandle, body: impl ToString) {
        let ServerHandle::WebSocket {
            ref mut write,
            ref mut read,
        } = handle
        else {
            panic!("Expected WebSocket handle");
        };

        // TODO: UNLESS it's a ping message
        if let Some(msg) = read.try_next().now_or_never() {
            panic!("Expected no incoming message, got {:?}", msg);
        }

        timeout(timeout_len, write.send(Message::text(body.to_string())))
            .await
            .expect("Timeout sending message")
            .expect("Failed to send message");
    }
}
