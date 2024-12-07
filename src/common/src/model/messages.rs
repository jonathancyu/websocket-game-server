use serde::{Deserialize, Serialize};
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

// Websocket messages
#[derive(Deserialize)]
pub struct SocketRequest<T> {
    pub user_id: Option<UserId>,
    pub request: T,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub struct SocketResponse<T>
where
    T: Serialize,
{
    pub user_id: UserId, // TODO: in here?
    pub message: T,
}
