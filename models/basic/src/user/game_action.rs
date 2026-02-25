use serde::{Deserialize, Serialize};

use crate::{message::TypedData, user::UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameActionSource {
    User(UserId),
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAction {
    pub message: TypedData,
    pub source: GameActionSource,
    pub ref_version: u32,
}
