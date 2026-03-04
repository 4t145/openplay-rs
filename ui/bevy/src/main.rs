pub mod game_components;
pub mod global_config;
pub mod ui;
use bevy::prelude::*;
use bevy::remote::RemotePlugin;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::window::PrimaryWindow;
use openplay_poker::{Rank, Suit};

use crate::{game_components::poker::PokerPlugin, ui::{button::UiButtonPlugin, poker_preview::PokerPreviewPlugin}};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Game,
    PokerPreview,
}

#[derive(Component)]
struct Card;

#[derive(Component, Default)]
struct CardTilt {
    target_rotation: Quat,
}

#[derive(Component)]
struct GameEntity;

fn main() {
    App::new()
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<AppState>()
        .add_plugins(PokerPreviewPlugin)
        .add_plugins(PokerPlugin)
        .add_plugins(UiButtonPlugin)
        .add_systems(Startup, global_config::theme_manager::setup_default_theme)
        .add_systems(OnEnter(AppState::Game), setup_game)
        .add_systems(OnExit(AppState::Game), cleanup_game)
        .add_systems(Update, game_input_handler.run_if(in_state(AppState::Game)))
        .run();
}

fn game_input_handler(
    mut next_state: ResMut<NextState<AppState>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyT) {
        next_state.set(AppState::PokerPreview);
    }
}

fn cleanup_game(mut commands: Commands, query: Query<Entity, With<GameEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn setup_game(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // 摄像机：向后移动一段距离，看向原点
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        GameEntity,
    ));

    // UI Hint
    commands.spawn((
        Text::new("Press 'T' to switch to Poker Theme Preview (2D)"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        GameEntity,
    ));
}
