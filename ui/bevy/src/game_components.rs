use bevy::app::{App, Plugin};

pub mod poker;

pub struct GameComponentsPlugin;

impl Plugin for GameComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(poker::PokerPlugin);
    }
}