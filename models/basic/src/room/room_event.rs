use serde::{Deserialize, Serialize};

use crate::{
    game::UpdateGameState,
    message::TypedData,
    player::player_event::{BecomeObserver, ObserverView},
    room::RoomPlayerPosition,
};

pub struct ServerToClientMessage {
    pub room_id: String,
    pub event: RoomEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RoomEvent {
    PlayerChat(PlayerChat),
    PlayerJoin(PlayerJoin),
    PlayerBecomeObserver(PlayerBecomeObserver),
    PlayerBecomePlayer(PlayerBecomePlayer),
    PlayerLeave(PlayerLeave),
    PlayerKickedOut(PlayerKickedOut),
    PlayerDisconnected(PlayerDisconnected),
    PlayerReconnected(PlayerReconnected),
    PlayerReady(PlayerReady),
    // GameMessage(GameMessage),
    UpdateGameState(UpdateGameState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerChat {
    pub player_id: crate::player::PlayerId,
    pub message: Vec<RoomMessageSegment>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomMessageSegment {
    Text(String),
    Emote(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerJoin {
    pub player_id: crate::player::PlayerId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerLeave {
    pub player_id: crate::player::PlayerId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerKickedOut {
    pub player_id: crate::player::PlayerId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDisconnected {
    pub player_id: crate::player::PlayerId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerReconnected {
    pub player_id: crate::player::PlayerId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerReady {
    pub player_id: crate::player::PlayerId,
    pub is_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMessage {
    pub player_id: crate::player::PlayerId,
    pub message: TypedData,
}
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct PlayerBecomeObserver {
    pub player_id: crate::player::PlayerId,
    pub view: ObserverView,
}
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct PlayerBecomePlayer {
    pub player_id: crate::player::PlayerId,
    pub position: RoomPlayerPosition,
}
