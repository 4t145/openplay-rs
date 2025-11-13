use bytes::Bytes;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerId(Bytes);

impl From<Bytes> for PlayerId {
    fn from(bytes: Bytes) -> Self {
        PlayerId(bytes)
    }
}

impl Serialize for PlayerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for PlayerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PlayerIdVisitor;

        impl<'de> serde::de::Visitor<'de> for PlayerIdVisitor {
            type Value = PlayerId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte array representing a PlayerId")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(PlayerId(Bytes::copy_from_slice(v)))
            }
        }

        deserializer.deserialize_bytes(PlayerIdVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Player {
    pub nickname: String,
    pub id: PlayerId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub is_bot: bool,
}

