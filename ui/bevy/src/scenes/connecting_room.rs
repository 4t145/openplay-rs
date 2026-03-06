use bevy::prelude::*;

use crate::state::MainState;
pub struct ConnectingRoomScenesPlugin;

impl Plugin for ConnectingRoomScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MainState::ConnectingGameRoom), setup_connecting_room)
            .add_systems(OnExit(MainState::ConnectingGameRoom), cleanup_connecting_room)
            .add_systems(OnEnter(MainState::ConnectingGameRoom), skip_connecting_room)
            ;
    }
}

#[derive(Component)]
pub struct ConnectingRoomScene;

#[derive(Component)]
pub struct ConnectingRoomRoot;

pub fn setup_connecting_room() {
    // Setup logic for connecting room scene
}

pub fn cleanup_connecting_room() {
    // Cleanup logic for connecting room scene
}

fn skip_connecting_room(mut next_state: ResMut<NextState<MainState>>) {
    next_state.set(MainState::GameRoom);
}