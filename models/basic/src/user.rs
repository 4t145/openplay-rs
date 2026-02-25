use std::pin::Pin;

use bytes::Bytes;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
pub mod player_event;
pub mod room_action;
pub mod game_action;
use crate::{
    message::{self, TypedData},
    user::player_event::PlayerEvent,
    room::RoomEvent,
};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(Bytes);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use base64::prelude::*;
        let encoded = BASE64_STANDARD.encode(&self.0);
        write!(f, "{}", encoded)
    }
}

impl From<Bytes> for UserId {
    fn from(bytes: Bytes) -> Self {
        UserId(bytes)
    }
}

impl Serialize for UserId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for UserId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PlayerIdVisitor;

        impl<'de> serde::de::Visitor<'de> for PlayerIdVisitor {
            type Value = UserId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte array representing a PlayerId")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UserId(Bytes::copy_from_slice(v)))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bytes = Vec::new();
                while let Some(byte) = seq.next_element()? {
                    bytes.push(byte);
                }
                Ok(UserId(Bytes::from(bytes)))
            }
        }

        deserializer.deserialize_bytes(PlayerIdVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct User {
    pub nickname: String,
    pub id: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub is_bot: bool,
}

impl User {
    pub fn new_robot(nickname: String, id: UserId) -> Self {
        User {
            nickname,
            id,
            avatar_url: None,
            is_bot: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlayerAgentError {
    #[error("Failed to send room event")]
    MessageHandlingFailed,
}

pub trait PlayerAgent: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync;
    fn send_room_event(
        &self,
        event: RoomEvent,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn receive_player_event(
        &self,
    ) -> impl Future<Output = Result<Option<PlayerEvent>, Self::Error>> + Send;
    fn close(&self) -> impl Future<Output = ()> + Send;
}
type DynError = dyn std::error::Error + Send + Sync + 'static;

pub trait DynPlayerAgentTrait {
    fn send_room_event<'a>(&'a self, event: RoomEvent) -> BoxFuture<'a, Result<(), Box<DynError>>>;
    fn receive_player_event<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Option<PlayerEvent>, Box<DynError>>>;
    fn close<'a>(&'a self) -> BoxFuture<'a, ()>;
}

impl<T> DynPlayerAgentTrait for T
where
    T: PlayerAgent,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    fn send_room_event<'a>(&'a self, event: RoomEvent) -> BoxFuture<'a, Result<(), Box<DynError>>> {
        Box::pin(async move { self.send_room_event(event).await.map_err(Box::from) })
    }

    fn receive_player_event<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Option<PlayerEvent>, Box<DynError>>> {
        Box::pin(async move { self.receive_player_event().await.map_err(Box::from) })
    }

    fn close<'a>(&'a self) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.close().await })
    }
}

pub fn new_dyn_player_agent<T>(agent: T) -> DynPlayerAgent
where
    T: PlayerAgent,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    Box::new(agent)
}

pub type DynPlayerAgent = Box<dyn DynPlayerAgentTrait + Send + Sync>;
