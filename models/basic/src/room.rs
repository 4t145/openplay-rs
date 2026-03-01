mod room_update;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Dtu,
    game::GameViewUpdate,
    user::{User, UserId},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub info: RoomInfo,
    pub state: RoomState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Update {
    Room(Box<RoomUpdate>),
    GameView(GameViewUpdate),
}

impl Room {
    pub fn new(mut info: RoomInfo, owner: User) -> Self {
        info.owner = owner.id.clone();
        Room {
            info,
            state: RoomState::from_owner(owner),
        }
    }
    pub fn remove_player(&mut self, player_id: &UserId) -> Option<User> {
        if let Some(observer_state) = self.state.observers.remove(player_id) {
            return Some(observer_state.player);
        } else if let Some(position) = self
            .state
            .players
            .iter()
            .find(|(_, state)| &state.player.id == player_id)
            .map(|(p, _)| p.clone())
        {
            let player_state = self.state.players.remove(&position);
            return player_state.map(|p| p.player);
        }
        None
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub id: String,
    pub owner: UserId,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_config: Option<crate::message::TypedData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomState {
    pub players: HashMap<RoomPlayerPosition, RoomPlayerState>,
    pub observers: HashMap<UserId, RoomObserverState>,
    pub phase: RoomPhase,
}

impl RoomState {
    pub fn from_owner(player: User) -> Self {
        let players = HashMap::new();
        let mut observers = HashMap::new();
        observers.insert(
            player.id.clone(),
            RoomObserverState {
                is_connected: true,
                view: RoomObserverView::default(),
                player: player.clone(),
            },
        );
        RoomState {
            players,
            observers,
            phase: RoomPhase {
                kind: RoomPhaseKind::Waiting,
                since: chrono::Utc::now(),
            },
        }
    }
    pub fn player_count(&self) -> usize {
        self.players.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPlayerState {
    pub id_ready: bool,
    pub is_connected: bool,
    pub player: crate::user::User,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomObserverState {
    pub is_connected: bool,
    pub view: RoomObserverView,
    pub player: crate::user::User,
}
pub use room_update::*;
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct RoomPlayerPosition(String);

impl RoomPlayerPosition {
    /// Access the inner position string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RoomPlayerPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for RoomPlayerPosition {
    fn from(s: String) -> Self {
        RoomPlayerPosition(s)
    }
}

impl From<&str> for RoomPlayerPosition {
    fn from(s: &str) -> Self {
        RoomPlayerPosition(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum RoomPhaseKind {
    Waiting,
    Gaming,
}
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct RoomPhase {
    pub kind: RoomPhaseKind,
    pub since: Dtu,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum RoomUserPosition {
    Player(RoomPlayerPosition),
    Observer(RoomObserverView),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Hash, PartialEq, Eq)]
#[serde(tag = "kind", content = "data")]
pub enum RoomObserverView {
    Position(RoomPlayerPosition),
    Player(UserId),
    #[default]
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Hash, PartialEq, Eq)]
#[serde(tag = "kind", content = "data")]
pub enum RoomView {
    Position(RoomPlayerPosition),
    #[default]
    Neutral,
}

pub struct RoomContext {
    pub room: Room,
}

impl RoomContext {
    pub fn new(room: Room) -> Self {
        Self { room }
    }

    pub fn get_room_info(&self) -> &RoomInfo {
        &self.room.info
    }

    pub fn get_room_state(&self) -> &RoomState {
        &self.room.state
    }

    /// Get the list of players ordered by seat position (0, 1, 2, ...).
    pub fn get_ordered_players(&self) -> Vec<User> {
        let mut seats: Vec<_> = self.room.state.players.iter().collect();
        seats.sort_by_key(|(pos, _)| pos.as_str().to_string());
        seats.into_iter().map(|(_, ps)| ps.player.clone()).collect()
    }
}
