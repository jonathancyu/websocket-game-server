use serde::{Deserialize, Serialize};
use uuid::Uuid;

// TODO: shouldn't be in the messages file
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Id(pub Uuid);

impl Id {
    pub fn new() -> Self {
        Id(Uuid::new_v4())
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
#[derive(Deserialize)]
pub struct SocketRequest<T> {
    pub user_id: Option<Id>,
    pub request: T,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub struct SocketResponse<T>
where
    T: Serialize,
{
    pub user_id: Id, // TODO: in here?
    pub message: T,
}

// Matchmaking <-> Game server interface
#[derive(Serialize, Deserialize)]
pub struct CreateGameRequest {
    pub game_id: Id,
    pub players: Vec<Id>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateGameResponse {
    pub game_id: Id,
}
