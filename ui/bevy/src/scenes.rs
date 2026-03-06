use bevy::app::{App, Plugin};

pub mod connecting_room;
pub mod lobby;
pub mod poker_preview;
pub mod game_room;
pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            poker_preview::PokerPreviewPlugin,
            lobby::LobbyScenesPlugin,
            connecting_room::ConnectingRoomScenesPlugin,
            game_room::GameRoomScenesPlugin,
        ));
    }
}
