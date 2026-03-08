use bevy::prelude::*;

use crate::{network::ReconnectState, state::MainState};
pub struct ConnectingRoomScenesPlugin;

impl Plugin for ConnectingRoomScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MainState::ConnectingGameRoom), setup_connecting_room)
            .add_systems(OnExit(MainState::ConnectingGameRoom), cleanup_connecting_room)
            .add_systems(OnEnter(MainState::ReconnectingGameRoom), setup_reconnecting_room)
            .add_systems(OnExit(MainState::ReconnectingGameRoom), cleanup_reconnecting_room)
            .add_systems(
                Update,
                update_reconnecting_text.run_if(in_state(MainState::ReconnectingGameRoom)),
            );
    }
}

#[derive(Component)]
pub struct ConnectingRoomScene;

#[derive(Component)]
pub struct ConnectingRoomRoot;

#[derive(Component)]
pub struct ReconnectingRoomScene;

#[derive(Component)]
pub struct ReconnectingStatusText;

pub fn setup_connecting_room(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        ConnectingRoomScene,
        ConnectingRoomRoot,
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![ (
            Text::new("Connecting to room server..."),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Black.ttf"),
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ) ],
    ));
}

pub fn cleanup_connecting_room(mut commands: Commands, query: Query<Entity, With<ConnectingRoomScene>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn setup_reconnecting_room(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    reconnect: Res<ReconnectState>,
) {
    let status = reconnect
        .last_error
        .as_ref()
        .map(|error| format!("last error: {}", error))
        .unwrap_or_else(|| "trying to reconnect...".to_string());

    commands.spawn((
        ReconnectingRoomScene,
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: px(8.0),
            ..default()
        },
        children![
            (
                Text::new("Connection lost, reconnecting..."),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Black.ttf"),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
            (
                ReconnectingStatusText,
                Text::new(status),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Black.ttf"),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.9, 0.9, 0.9)),
            )
        ],
    ));
}

pub fn cleanup_reconnecting_room(
    mut commands: Commands,
    query: Query<Entity, With<ReconnectingRoomScene>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn update_reconnecting_text(
    reconnect: Res<ReconnectState>,
    mut text_query: Query<&mut Text, With<ReconnectingStatusText>>,
) {
    if !reconnect.is_changed() {
        return;
    }

    let status = reconnect
        .last_error
        .as_ref()
        .map(|error| {
            format!(
                "attempt {}/10, last error: {}",
                reconnect.attempts,
                error
            )
        })
        .unwrap_or_else(|| format!("attempt {}/10", reconnect.attempts));

    for mut text in text_query.iter_mut() {
        text.0 = status.clone();
    }
}
