use std::pin::Pin;

use bytes::Bytes;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
pub mod game_action;
pub mod player_event;
pub mod room_action;
use crate::{
    message::{self, TypedData},
    room::{RoomEvent, Update},
    user::{game_action::GameActionData, room_action::RoomActionData},
};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(Bytes);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ActionSource {
    User(UserId),
    System,
}

impl From<UserId> for ActionSource {
    fn from(user_id: UserId) -> Self {
        ActionSource::User(user_id)
    }
}

impl From<&UserId> for ActionSource {
    fn from(user_id: &UserId) -> Self {
        ActionSource::User(user_id.clone())
    }
}

impl From<()> for ActionSource {
    fn from(_: ()) -> Self {
        ActionSource::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub source: ActionSource,
    #[serde(flatten)]
    pub data: ActionData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type", content = "data")]
pub enum ActionData {
    RoomAction(RoomActionData),
    GameAction(GameActionData),
}

impl ActionData {
    pub fn with_source(self, source: impl Into<ActionSource>) -> Action {
        Action { source: source.into(), data: self }   
    }
}

impl Action {
    pub fn source(&self) -> Option<&UserId> {
        match &self.source {
            ActionSource::User(user_id) => Some(user_id),
            ActionSource::System => None,
        }
    }
}

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
                formatter.write_str("a byte array representing a UserId")
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
pub enum UserAgentError {
    #[error("Failed to send update")]
    MessageHandlingFailed,
}

pub trait UserAgent: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync;
    fn send_update(&self, update: Update) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn receive_action(
        &self,
    ) -> impl Future<Output = Result<Option<ActionData>, Self::Error>> + Send;
    fn close(&self) -> impl Future<Output = ()> + Send;
}
type DynError = dyn std::error::Error + Send + Sync + 'static;

pub trait DynUserAgentTrait {
    fn send_update<'a>(&'a self, update: Update) -> BoxFuture<'a, Result<(), Box<DynError>>>;
    fn receive_action<'a>(&'a self) -> BoxFuture<'a, Result<Option<ActionData>, Box<DynError>>>;
    fn close<'a>(&'a self) -> BoxFuture<'a, ()>;
}

impl<T> DynUserAgentTrait for T
where
    T: UserAgent,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    fn send_update<'a>(&'a self, update: Update) -> BoxFuture<'a, Result<(), Box<DynError>>> {
        Box::pin(async move { self.send_update(update).await.map_err(Box::from) })
    }

    fn receive_action<'a>(&'a self) -> BoxFuture<'a, Result<Option<ActionData>, Box<DynError>>> {
        Box::pin(async move { self.receive_action().await.map_err(Box::from) })
    }

    fn close<'a>(&'a self) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.close().await })
    }
}

pub fn new_dyn_user_agent<T>(agent: T) -> DynUserAgent
where
    T: UserAgent,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    Box::new(agent)
}

pub type DynUserAgent = Box<dyn DynUserAgentTrait + Send + Sync>;
