use crate::state::MainState;
use bevy::prelude::*;
use bevy::window::Window;
use bevy_ui_text_input::{TextInputMode, TextInputNode, TextInputPrompt};
pub struct LobbyScenesPlugin;

impl Plugin for LobbyScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MainState::Lobby), setup_lobby)
            .add_systems(OnExit(MainState::Lobby), cleanup_lobby)
            .add_systems(
                Update,
                handle_enter_room_button.run_if(in_state(MainState::Lobby)),
            );
    }
}

#[derive(Component)]
pub struct LobbyScene;

#[derive(Component)]
pub struct LobbyRoot;

#[derive(Component)]
pub struct LobbyEnterRoomButton;

#[derive(Component)]
pub struct LobbyExitButton;

#[derive(Component)]

pub struct UiRoomNavigator;

#[derive(Component)]

pub struct UiRoomNavigatorInput;

#[derive(Component)]

pub struct UiRoomNavigatorEnterButton;

pub fn ui_room_navigator(asset_server: Res<AssetServer>) -> impl Bundle {
    (
        UiRoomNavigator,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        children![
            (
                UiRoomNavigatorInput,
                TextInputNode {
                    clear_on_submit: false,
                    mode: TextInputMode::SingleLine,
                    ..Default::default()
                },
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Black.ttf"),
                    font_size: 20.0,
                    ..default()
                },
                TextInputPrompt::new("Code or host addr"),
                Node {
                    height: Val::Px(30.0),
                    flex_grow: 1.0,
                    margin: UiRect::axes(px(5.0), px(2.5)),
                    ..Default::default()
                },
                BackgroundColor(bevy::color::palettes::css::BLACK.into()),
            ),
            (
                UiRoomNavigatorEnterButton,
                Button,
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(30.0),
                    margin: UiRect::axes(px(5.0), px(2.5)),
                    ..Default::default()
                },
                children![(
                    Text::new("Enter"),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Black.ttf"),
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(bevy::color::palettes::css::WHITE.into()),
                    TextShadow::default(),
                )]
            ),
        ],
    )
}

pub fn handle_enter_room_button(
    mut next_state: ResMut<NextState<MainState>>,
    mut interaction_query: Query<
        (&Interaction, &mut Button),
        (
            Changed<Interaction>,
            With<Button>,
            With<UiRoomNavigatorEnterButton>,
        ),
    >,
) {
    for (interaction, mut button) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(MainState::ConnectingGameRoom);
            button.set_changed(); // Mark button as changed to update its state
        }
    }
}

pub fn setup_lobby(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut windows: Query<&mut Window>,
) {
    let mut main_window = windows.single_mut().expect("single window");
    main_window.ime_enabled = true;

    let bundle = (
        LobbyRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        children![ui_room_navigator(asset_server)],
    );
    commands.spawn((bundle, LobbyScene));
}

pub fn cleanup_lobby(
    mut commands: Commands,
    query: Query<Entity, With<LobbyScene>>,
    mut windows: Query<&mut Window>,
) {
    let mut main_window = windows.single_mut().expect("single window");
    main_window.ime_enabled = false;

    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
