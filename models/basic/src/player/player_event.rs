use serde::{Deserialize, Serialize};

use crate::{
    ban::Ban, player::PlayerId, room::{GameMessage, PlayerChat, PlayerReady, RoomPlayerPosition}
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
    pub player: PlayerId,
    pub reason: Option<String>,
    pub ban: Option<Ban>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BecomePlayer {
    pub position: RoomPlayerPosition,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BecomeObserver {
    pub view: ObserverView
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag="kind", content="data")]
pub enum ObserverView {
    Position(RoomPlayerPosition),
    Player(PlayerId),
    #[default]
    Neutral
}