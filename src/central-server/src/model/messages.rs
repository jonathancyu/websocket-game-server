use std::net::Ipv6Addr;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct UserId(pub Uuid);

impl<'de> Deserialize<'de> for UserId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let uuid = Uuid::parse_str(&s).map_err(serde::de::Error::custom)?;
        Ok(UserId(uuid))
    }
}
impl Serialize for UserId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

// Message types used for inter-thread communication
#[derive(Debug, Clone)]
pub struct MatchDetails {}

#[derive(Debug, Clone)]
pub enum MatchmakingRequest {
    JoinQueue(QueuedPlayer),
    LeaveQueue(UserId),
    Disconnected(UserId),
}

#[derive(Debug, Clone)]
pub enum MatchmakingResponse {
    QueueJoined,
    MatchFound(MatchDetails),
}

// Message types for the matchmaking thread
#[derive(Debug, Clone)]
pub struct QueuedPlayer {
    pub id: UserId,
    pub sender: Sender<MatchmakingResponse>,
}

// Websocket messages
#[derive(Deserialize)]
pub struct SocketRequest {
    pub id: Option<UserId>,
    pub request: ClientRequest,
}

#[derive(Deserialize)]
pub struct SocketResponse {
    pub id: Option<UserId>, // TODO: in here?
    pub message: ClientRequest,
}

// API messages

#[derive(Deserialize)]
pub enum ClientRequest {
    // Add user to queue
    JoinQueue,
    // Ensure queue is still alive
    Ping,
    // User was disconnected from the match, and needs the server address again
    GetServer,
}

#[derive(Serialize)]
pub enum ClientResponse {
    // Connected
    Connected { user_id: UserId },
    // Ack user joining queue
    JoinedQueue,
    // Constant ping to let user know still connected
    QueuePing { time_elapsed: u32 },
    // Notify user to connect to server at given IP
    JoinServer { server_ip: Ipv6Addr },
    // Error occurred
    Error { message: String },
}
