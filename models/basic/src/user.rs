use base64::prelude::*;
use futures_util::future::BoxFuture;
use rand::Rng;
use serde::{Deserialize, Serialize};
pub mod game_action;
pub mod player_event;
pub mod room_action;
use crate::{
    room::Update,
    user::{game_action::GameActionData, room_action::RoomActionData},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ActionSource {
    User(UserId),
    System,
}
#[derive( Clone, PartialEq, Eq, Hash)]
pub struct UserId([u8; ed25519_dalek::PUBLIC_KEY_LENGTH]);

impl std::fmt::Debug for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let encoded = BASE64_STANDARD.encode(self.0);
        write!(f, "UserId({})", encoded)
    }
}

/// UserId 解析错误
#[derive(Debug, thiserror::Error)]
pub enum UserIdParseError {
    #[error("base64 decode failed: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("invalid length: expected {expected} bytes, got {got} bytes")]
    InvalidLength { expected: usize, got: usize },
}

impl UserId {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let mut bytes = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
        rng.fill_bytes(&mut bytes);
        UserId(bytes)
    }

    /// 直接从固定长度字节数组构造，供测试或内部使用
    pub fn from_bytes(bytes: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH]) -> Self {
        UserId(bytes)
    }

    /// 返回内部字节的引用
    pub fn as_bytes(&self) -> &[u8; ed25519_dalek::PUBLIC_KEY_LENGTH] {
        &self.0
    }
}

impl TryFrom<&str> for UserId {
    type Error = UserIdParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let decoded = BASE64_STANDARD.decode(s)?;
        if decoded.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
            return Err(UserIdParseError::InvalidLength {
                expected: ed25519_dalek::PUBLIC_KEY_LENGTH,
                got: decoded.len(),
            });
        }
        let mut bytes = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
        bytes.copy_from_slice(&decoded);
        Ok(UserId(bytes))
    }
}

impl TryFrom<String> for UserId {
    type Error = UserIdParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        UserId::try_from(s.as_str())
    }
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
        Action {
            source: source.into(),
            data: self,
        }
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
        let encoded = BASE64_STANDARD.encode(self.0);
        write!(f, "{}", encoded)
    }
}

impl Serialize for UserId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            // For human-readable formats, encode as base64 string
            let encoded = BASE64_STANDARD.encode(self.0);
            serializer.serialize_str(&encoded)
        } else {
            // For binary formats, serialize as bytes
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for UserId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            // For human-readable formats, expect a base64 string
            let s = String::deserialize(deserializer)?;
            let decoded = BASE64_STANDARD
                .decode(s.as_bytes())
                .map_err(serde::de::Error::custom)?;
            if decoded.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
                return Err(serde::de::Error::custom(format!(
                    "Invalid length for UserId: expected {}, got {}",
                    ed25519_dalek::PUBLIC_KEY_LENGTH,
                    decoded.len()
                )));
            }
            let mut bytes = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
            bytes.copy_from_slice(&decoded);
            Ok(UserId(bytes))
        } else {
            // For binary formats, deserialize as bytes
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            if bytes.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
                return Err(serde::de::Error::custom(format!(
                    "Invalid length for UserId: expected {}, got {}",
                    ed25519_dalek::PUBLIC_KEY_LENGTH,
                    bytes.len()
                )));
            }
            let mut array = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
            array.copy_from_slice(&bytes);
            Ok(UserId(array))
        }
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
