use serde::{Deserialize, Serialize};

use crate::{
    message::TypedData,
    room::{RoomChatMessageContent, RoomUserPosition}, user::UserId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomActionSource {
    User(UserId),
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)] 
pub struct RoomAction {
    pub source: RoomActionSource,
    pub kind: RoomActionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomActionKind {
    Chat(RoomChatMessageContent),
    ChangeReadyState(ReadyStateChange),
    PositionChange(PositionChange),
    RoomManage(RoomManage),
    Reconnect,
    Leave,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionChange {
    pub from: RoomUserPosition,
    pub to: RoomUserPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickOut {
    pub player: crate::user::UserId,
    pub reason: Option<String>,
    pub ban: Option<crate::ban::Ban>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomManage {
    KickOut(KickOut),
    SetGameConfig(TypedData),
    SetRoomConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyStateChange {
    pub is_ready: bool,
}
