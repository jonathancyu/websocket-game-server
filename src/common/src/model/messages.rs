use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// TODO: shouldn't be in the messages file
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Id(pub Uuid);

impl Id {
    pub fn new() -> Self {
        Id(Uuid::new_v4())
    }
}
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let uuid = Uuid::parse_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Id(uuid))
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

// Websocket messages
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenSocketRequest {
    pub user_id: Id,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SocketRequest<T> {
    pub user_id: Option<Id>, // TODO: remove optional
    pub body: T,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct SocketResponse<T>
where
    T: Serialize,
{
    pub user_id: Id, // TODO: in here?
    pub body: T,
}

// Matchmaking <-> Game server interface
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CreateGameRequest {
    pub players: Vec<Id>, // TODO: into tuple
    pub games_to_win: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CreateGameResponse {
    pub game_id: Id,
}

#[derive(Serialize, Deserialize)]
pub struct GetGameRequest {
    pub game_id: Id,
}

#[derive(Serialize, Deserialize)]
pub struct GetGameResponse {
    pub game_id: Id,
    pub players: (Id, Id),
}
