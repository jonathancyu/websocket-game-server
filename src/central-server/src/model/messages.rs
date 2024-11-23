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
// Message types used for inter-thread communication
#[derive(Debug, Clone)]
pub struct MatchDetails {}

#[derive(Debug, Clone)]
pub enum MatchmakingRequest {
    JoinQueue(QueuedPlayer),
    LeaveQueue(String),
    Exit,
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

// API Request/response

#[derive(Deserialize)]
pub enum ClientRequest {
    // Add user to queue
    JoinQueue { user_id: UserId },
    // Ensure queue is still alive
    Ping,
    // User was disconnected from the match, and needs the server address again
    GetServer { user_id: UserId },
}

#[derive(Serialize)]
pub enum ClientResponse {
    // Ack user joining queue
    JoinedQueue,
    // Constant ping to let user know still connected
    QueuePing { time_elapsed: u32 },
    // Notify user to connect to server at given IP
    JoinServer { server_ip: Ipv6Addr },
    // Error occurred
    Error { message: String },
}
