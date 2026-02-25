use serde::{Deserialize, Serialize};

use crate::{
    ban::Ban,
    room::{GameMessage, PlayerChat, PlayerReady, RoomPlayerPosition},
    user::UserId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PlayerEvent {
    PlayerChat(PlayerChat),
    PlayerReady(PlayerReady),
    KickOut(KickOut),
    GameMessage(GameMessage),
    HeartBeat,
    StartGame,
    Leave,
    BecomePlayer(BecomePlayer),
    BecomeObserver(BecomeObserver),
    Reconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickOut {
    pub player: UserId,
    pub reason: Option<String>,
    pub ban: Option<Ban>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BecomePlayer {
    pub position: RoomPlayerPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BecomeObserver {
    pub view: RoomObserverView,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Hash, PartialEq, Eq)]
#[serde(tag = "kind", content = "data")]
pub enum RoomObserverView {
    Position(RoomPlayerPosition),
    Player(UserId),
    #[default]
    Neutral,
}
