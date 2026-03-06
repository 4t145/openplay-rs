use bevy::prelude::*;

use crate::state::MainState;
pub struct GameRoomScenesPlugin;

impl Plugin for GameRoomScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MainState::GameRoom), setup_game_room)
            .add_systems(OnExit(MainState::GameRoom), cleanup_game_room);
    }
}

pub fn setup_game_room() {
    // Setup logic for game room scene
}

pub fn cleanup_game_room() {
    // Cleanup logic for game room scene
}