use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::player::PlayerId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub id: String,
    pub owner: PlayerId,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomState {
    pub players: HashMap<PlayerId, RoomPlayerState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPlayerState {
    pub id_ready: bool,
    pub is_connected: bool,
    pub player: crate::player::Player,
}


