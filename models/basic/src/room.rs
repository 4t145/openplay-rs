mod room_update;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Dtu, game::{GameMeta, GameViewUpdate, PositionSpec}, user::{User, UserId}
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
    pub fn new(info: RoomInfo) -> Self {
        Room {
            info,
            state: RoomState::empty(),
        }
    }

    pub fn new_with_position_spec(info: RoomInfo, position_spec: &PositionSpec) -> Self {
        Room {
            info,
            state: RoomState::empty_with_position_spec(position_spec),
        }
    }
    pub fn remove_player(&mut self, player_id: &UserId) -> Option<User> {
        if let Some(observer_state) = self.state.observers.remove(player_id) {
            return Some(observer_state.player);
        } else if let Some(position) = self.state.find_player_position(player_id) {
            if let Some(seat) = self.state.positions.seats.get_mut(&position) {
                if let SeatState::Occupied(player_state) =
                    std::mem::replace(seat, SeatState::Vacant)
                {
                    return Some(player_state.player);
                }
            }
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
    pub game_meta: GameMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_config: Option<crate::message::TypedData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomState {
    pub positions: PositionState,
    pub observers: HashMap<UserId, RoomObserverState>,
    pub phase: RoomPhase,
}

impl RoomState {
    pub fn empty() -> Self {
        RoomState {
            positions: PositionState::empty(),
            observers: HashMap::new(),
            phase: RoomPhase {
                kind: RoomPhaseKind::Waiting,
                since: chrono::Utc::now(),
            },
        }
    }

    pub fn empty_with_position_spec(position_spec: &PositionSpec) -> Self {
        RoomState {
            positions: PositionState::from_spec(position_spec),
            observers: HashMap::new(),
            phase: RoomPhase {
                kind: RoomPhaseKind::Waiting,
                since: chrono::Utc::now(),
            },
        }
    }

    pub fn player_count(&self) -> usize {
        self.positions.occupied_count()
    }

    pub fn find_player_position(&self, user_id: &UserId) -> Option<RoomPlayerPosition> {
        self.positions
            .seats
            .iter()
            .find_map(|(position, seat)| match seat {
                SeatState::Occupied(player_state) if &player_state.player.id == user_id => {
                    Some(position.clone())
                }
                _ => None,
            })
    }

    pub fn get_player_state(&self, position: &RoomPlayerPosition) -> Option<&RoomPlayerState> {
        self.positions
            .seats
            .get(position)
            .and_then(|seat| match seat {
                SeatState::Occupied(player_state) => Some(player_state),
                SeatState::Vacant => None,
            })
    }

    pub fn get_player_state_mut(
        &mut self,
        position: &RoomPlayerPosition,
    ) -> Option<&mut RoomPlayerState> {
        self.positions
            .seats
            .get_mut(position)
            .and_then(|seat| match seat {
                SeatState::Occupied(player_state) => Some(player_state),
                SeatState::Vacant => None,
            })
    }

    pub fn set_player_state(
        &mut self,
        position: RoomPlayerPosition,
        player_state: RoomPlayerState,
    ) {
        self.positions
            .seats
            .insert(position, SeatState::Occupied(player_state));
    }

    pub fn clear_position(&mut self, position: &RoomPlayerPosition) -> Option<RoomPlayerState> {
        let seat = self.positions.seats.get_mut(position)?;
        match std::mem::replace(seat, SeatState::Vacant) {
            SeatState::Occupied(player_state) => Some(player_state),
            SeatState::Vacant => None,
        }
    }

    pub fn iter_players(&self) -> impl Iterator<Item = (&RoomPlayerPosition, &RoomPlayerState)> {
        self.positions
            .seats
            .iter()
            .filter_map(|(position, seat)| match seat {
                SeatState::Occupied(player_state) => Some((position, player_state)),
                SeatState::Vacant => None,
            })
    }

    pub fn iter_players_mut(
        &mut self,
    ) -> impl Iterator<Item = (&RoomPlayerPosition, &mut RoomPlayerState)> {
        self.positions
            .seats
            .iter_mut()
            .filter_map(|(position, seat)| match seat {
                SeatState::Occupied(player_state) => Some((position, player_state)),
                SeatState::Vacant => None,
            })
    }

    pub fn all_players_ready(&self) -> bool {
        self.iter_players().all(|(_, player)| player.id_ready)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionState {
    pub mode: PositionMode,
    pub seats: HashMap<RoomPlayerPosition, SeatState>,
}

impl PositionState {
    pub fn empty() -> Self {
        Self {
            mode: PositionMode::Fixed,
            seats: HashMap::new(),
        }
    }

    pub fn from_spec(position_spec: &PositionSpec) -> Self {
        match position_spec {
            PositionSpec::Fixed { positions } => {
                let seats = positions
                    .iter()
                    .cloned()
                    .map(|position| (position, SeatState::Vacant))
                    .collect();
                Self {
                    mode: PositionMode::Fixed,
                    seats,
                }
            }
            PositionSpec::Flexible {
                min_players,
                max_players,
                default_players,
            } => {
                let player_count = (*default_players).clamp(*min_players, *max_players);
                let seats = (1..=player_count)
                    .map(|index| {
                        (
                            RoomPlayerPosition::from(index.to_string()),
                            SeatState::Vacant,
                        )
                    })
                    .collect();
                Self {
                    mode: PositionMode::Flexible {
                        min_players: *min_players,
                        max_players: *max_players,
                    },
                    seats,
                }
            }
        }
    }

    pub fn occupied_count(&self) -> usize {
        self.seats
            .values()
            .filter(|seat| matches!(seat, SeatState::Occupied(_)))
            .count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum PositionMode {
    Fixed,
    Flexible {
        min_players: usize,
        max_players: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeatState {
    Vacant,
    Occupied(RoomPlayerState),
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
        let mut seats: Vec<_> = self.room.state.iter_players().collect();
        seats.sort_by_key(|(pos, _)| pos.as_str().to_string());
        seats.into_iter().map(|(_, ps)| ps.player.clone()).collect()
    }
}
