use crate::{GlobalAssets, MainState, data::CurrentUserInfo, state::OverlayState, ui::Ui};
use bevy::prelude::*;
/*

[Avatar] <username>                                [AssetManage] [Settings] [Menu]


*/
#[derive(Component)]
pub struct GeneralUi; // Tag for cleanup

#[derive(Component)]
pub struct GeneralUiRoot;

#[derive(Component)]
pub struct GeneralUiTopBar;

#[derive(Component)]
pub struct GeneralUiTopBarLeftGroup;

#[derive(Component)]
pub struct GeneralUiTopBarRightGroup;

#[derive(Component)]
pub struct GeneralUiPlayerAvatar;

#[derive(Component)]
pub struct GeneralUiPlayerUsername;

#[derive(Component)]
pub struct GeneralUiThemeButton;
#[derive(Component)]
pub struct GeneralUiSettingsButton;
#[derive(Component)]
pub struct GeneralUiMenuButton;
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
        GeneralUi,
        GeneralUiRoot,
        Ui,
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
            GeneralUiTopBar,
            Node {
                width: percent(100),
                height: Val::Px(50.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            children![
                (
                    GeneralUiTopBarLeftGroup,
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: UiRect::all(px(10.0)),
                        ..Default::default()
                    },
                    children![
                        (
                            GeneralUiPlayerAvatar,
                            ImageNode {
                                color: Color::WHITE,
                                ..Default::default()
                            }
                        ),
                        (
                            GeneralUiPlayerUsername,
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
                    GeneralUiTopBarRightGroup,
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexEnd,
                        ..Default::default()
                    },
                    children![
                        functional_button(GeneralUiThemeButton, global_assets.icon_dice.clone()),
                        functional_button(
                            GeneralUiSettingsButton,
                            global_assets.icon_settings.clone()
                        ),
                        functional_button(GeneralUiMenuButton, global_assets.icon_menu.clone()),
                    ]
                )
            ]
        )],
    );
    commands.spawn(general_hud_entity);
}
fn clean_up_general_hud(mut commands: Commands, query: Query<Entity, With<GeneralUi>>) {
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
    interaction: Query<(&Interaction, &Button, &GeneralUiThemeButton)>,
) {
    for (interaction, _, _) in interaction.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(OverlayState::Theme);
        }
    }
}

fn general_hud_menu_button_handler(
    mut next_state: ResMut<NextState<OverlayState>>,
    interaction: Query<(&Interaction, &Button, &GeneralUiMenuButton)>,
) {
    for (interaction, _, _) in interaction.iter() {
        if *interaction == Interaction::Pressed {
            info!("Menu button pressed");
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeneralUiInputSystemSet;
pub struct GeneralUiPlugin;

impl Plugin for GeneralUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentUserInfo { user: None });
        app.add_systems(
            Update,
            (
                general_hud_theme_button_handler,
                general_hud_menu_button_handler,
                general_hud_input_handler,
            )
                .in_set(GeneralUiInputSystemSet)
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
