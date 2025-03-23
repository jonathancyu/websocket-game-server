use std::net::Ipv6Addr;

use common::model::messages::Id;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

// Message types used for inter-thread communication
#[derive(Debug, Clone)]
pub enum MatchmakingRequest {
    JoinQueue(Player),
    LeaveQueue(Id),
}

// Message types for the matchmaking thread
#[derive(Debug, Clone)]
pub struct Player {
    pub id: Id,
    pub sender: Sender<ClientResponse>,
}

// API messages

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    // Add user to queue
    JoinQueue,
    // Ensure queue is still alive
    Ping,
    // User was disconnected from the match, and needs the server address again
    GetServer,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum ClientResponse {
    // User actually joined queue
    JoinedQueue,
    // Constant ping to let user know still connected
    QueuePing { time_elapsed: u32 },
    // Notify user to connect to server at given IP
    MatchFound { game_id: Id, server_address: String },
    // Notify user to connect to server at given IP
    // TODO: implement querying for server when one goes down
    JoinServer { server_ip: Ipv6Addr },
}
