use crate::{GlobalAssets, MainState, data::CurrentUserInfo, state::OverlayState, ui::Hud};
use bevy::prelude::*;
/*

[Avatar] <username>                                [AssetManage] [Settings] [Menu]


*/
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

#[derive(Component)]
pub struct GeneralHudThemeButton;
#[derive(Component)]
pub struct GeneralHudSettingsButton;
#[derive(Component)]
pub struct GeneralHudMenuButton;
pub fn button_icon(image: Handle<Image>, color: Color) -> impl Bundle {
    (
        ImageNode {
            image,
            image_mode: NodeImageMode::Stretch,
            color,
            ..Default::default()
        },
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
    )
}

pub fn functional_button<Tag: Component>(tag: Tag, icon: Handle<Image>) -> impl Bundle {
    (
        Button,
        tag,
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            height: Val::Px(30.0),
            width: Val::Px(30.0),
            margin: UiRect::horizontal(px(5.0)),
            ..Default::default()
        },
        children![button_icon(icon, Color::WHITE),],
    )
}
pub fn setup_general_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    current_user_info: Res<CurrentUserInfo>,
    global_assets: Res<GlobalAssets>,
) {
    let current_user_name = current_user_info
        .user
        .as_ref()
        .map(|user| user.nickname.clone())
        .unwrap_or_else(|| "<anon>".to_string());
    let general_hud_entity = (
        GeneralHud,
        GeneralHudRoot,
        Hud,
        Visibility::Visible,
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
                            Text::new(current_user_name),
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
                    },
                    children![
                        functional_button(GeneralHudThemeButton, global_assets.icon_dice.clone()),
                        functional_button(
                            GeneralHudSettingsButton,
                            global_assets.icon_settings.clone()
                        ),
                        functional_button(GeneralHudMenuButton, global_assets.icon_menu.clone()),
                    ]
                )
            ]
        )],
    );
    commands.spawn(general_hud_entity);
}
fn clean_up_general_hud(mut commands: Commands, query: Query<Entity, With<GeneralHud>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
fn general_hud_input_handler(
    mut next_state: ResMut<NextState<MainState>>,
    input: Res<ButtonInput<KeyCode>>,
) {
}

fn general_hud_theme_button_handler(
    mut next_state: ResMut<NextState<OverlayState>>,
    interaction: Query<(&Interaction, &Button, &GeneralHudThemeButton)>,
) {
    for (interaction, _, _) in interaction.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(OverlayState::Theme);
        }
    }
}

fn general_hud_menu_button_handler(
    mut next_state: ResMut<NextState<OverlayState>>,
    interaction: Query<(&Interaction, &Button, &GeneralHudMenuButton)>,
) {
    for (interaction, _, _) in interaction.iter() {
        if *interaction == Interaction::Pressed {
            info!("Menu button pressed");
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeneralHudInputSystemSet;
pub struct GeneralHudPlugin;

impl Plugin for GeneralHudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentUserInfo { user: None });
        app.add_systems(
            Update,
            (
                general_hud_theme_button_handler,
                general_hud_menu_button_handler,
                general_hud_input_handler,
            )
                .in_set(GeneralHudInputSystemSet)
                .run_if(in_state(OverlayState::None)),
        );
        app.add_systems(
            OnEnter(OverlayState::None),
            setup_general_hud.run_if(not(in_state(MainState::GlobalAssetsLoading))),
        )
        .add_systems(OnExit(MainState::GlobalAssetsLoading), setup_general_hud)
        .add_systems(OnExit(OverlayState::None), clean_up_general_hud);
    }
}
