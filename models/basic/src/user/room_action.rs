use serde::{Deserialize, Serialize};

use crate::{
    message::TypedData,
    room::{Chat, RoomPlayerPosition, RoomUserPosition},
    user::UserId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "data_type", content = "data")]
pub enum RoomActionData {
    Join(JoinRoom),
    Chat(Chat),
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
pub struct AddBot {
    pub position: RoomPlayerPosition,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomManage {
    KickOut(KickOut),
    SetGameConfig(TypedData),
    SetRoomConfig,
    AddBot(AddBot),
    StartGame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyStateChange {
    pub is_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRoom {
    pub nickname: String,
}
