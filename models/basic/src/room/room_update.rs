use serde::{Deserialize, Serialize};

use crate::{
    room::{Room, RoomState},
    user::{
        room_action::{PositionChange, ReadyStateChange, RoomManage},
        UserId,
    },
};

pub struct ServerToClientMessage {
    pub room_id: String,
    pub event: RoomEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActionEvent<D = ()> {
    pub user_id: UserId,
    pub data: D,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RoomEvent {
    UserChat(UserActionEvent<Chat>),
    UserJoin(UserActionEvent),
    UserChangePosition(UserActionEvent<PositionChange>),
    UserLeave(UserActionEvent),
    UserKickedOut(UserActionEvent),
    UserDisconnected(UserActionEvent),
    UserReconnected(UserActionEvent),
    UserReady(UserActionEvent<ReadyStateChange>),
    RoomManage(UserActionEvent<RoomManage>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomUpdate {
    pub room: Room,
    pub events: Vec<RoomEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub message: Vec<RoomMessageSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomMessageSegment {
    Text(String),
    Emote(String),
}
