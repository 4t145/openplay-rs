mod room_event;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Dtu,
    game::{IntervalId, TimerId},
    user::{User, UserId, player_event::RoomObserverView},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub info: RoomInfo,
    pub state: RoomState,
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
pub use room_event::*;
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct RoomPlayerPosition(String);

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

pub struct RoomContext {}

impl RoomContext {
    pub fn request_timer() -> TimerId {
        todo!()
    }

    pub fn request_interval() -> IntervalId {
        todo!()
    }

    pub fn get_room_info(&self) -> RoomInfo {
        todo!()
    }
}
