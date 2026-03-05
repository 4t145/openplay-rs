use crate::{AppState, ui::Hud};
use bevy::prelude::*;

#[derive(Component)]
pub struct GeneralHud; // Tag for cleanup

#[derive(Component)]
pub struct GeneralHudRoot;

#[derive(Component)]
pub struct GeneralHudTopBar;

#[derive(Component)]
pub struct GeneralHudTopBarLeftGroup;

#[derive(Component)]
pub struct GeneralHudTopBarRightGroup;

#[derive(Component)]
pub struct GeneralHudPlayerAvatar;

#[derive(Component)]
pub struct GeneralHudPlayerUsername;

pub fn setup_general_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let general_hud_entity = (
        GeneralHud,
        GeneralHudRoot,
        Hud,
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        children![(
            GeneralHudTopBar,
            Node {
                width: percent(100),
                height: Val::Px(50.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            children![
                (
                    GeneralHudTopBarLeftGroup,
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: UiRect::all(px(10.0)),
                        ..Default::default()
                    },
                    children![
                        (
                            GeneralHudPlayerAvatar,
                            ImageNode {
                                color: Color::WHITE,
                                ..Default::default()
                            }
                        ),
                        (
                            GeneralHudPlayerUsername,
                            Text::new("Player1"),
                            TextFont {
                                font: asset_server.load("fonts/FiraSans-Black.ttf"),
                                font_size: 20.0,
                                ..Default::default()
                            },
                            TextColor(Color::WHITE),
                        )
                    ]
                ),
                (
                    GeneralHudTopBarRightGroup,
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexEnd,
                        ..Default::default()
                    }
                )
            ]
        )],
    );
    commands.spawn(general_hud_entity);
}


pub struct GeneralHudPlugin;

impl Plugin for GeneralHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_general_hud);
    }
}