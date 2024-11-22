use std::{fmt::Display, net::Ipv6Addr};

use serde::{de, Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Hash, Eq, PartialEq)]
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

#[derive(Hash, Eq, PartialEq)]
pub struct QueueMessage {
    pub user_id: UserId,
}

// API Request/response

#[derive(Deserialize)]
pub enum Request {
    // Add user to queue
    JoinQueue { user_id: UserId },
    // User was disconnected from the match, and needs the server address again
    GetServer { user_id: UserId },
}

#[derive(Serialize)]
pub enum Response {
    // Ack user joining queue
    JoinedQueue,
    // Constant ping to let user know still connected
    QueuePing { time_elapsed: u32 },
    // Notify user to connect to server at given IP
    JoinServer { server_ip: Ipv6Addr },
    // Error occurred
    Error { message: String },
}
