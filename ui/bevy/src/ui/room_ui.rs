use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::HoverMap,
    prelude::*,
};
use bevy_ui_text_input::{SubmitText, TextInputMode, TextInputNode, TextInputPrompt};
use openplay_basic::{
    room::PositionMode,
    room::{RoomMessageSegment, RoomObserverView, RoomPlayerPosition, RoomUserPosition, SeatState},
    user::{
        room_action::{
            AddBot, KickOut, PositionChange, ReadyStateChange, RoomActionData, RoomManage,
        },
        ActionData,
    },
};

use crate::{
    data::CurrentUserInfo,
    network::{ConnectionRuntime, RoomSnapshot},
    state::MainState,
    ui::Ui,
};

const ROOT_PADDING_PX: f32 = 16.0;
const TOP_BAR_HEIGHT_PX: f32 = 50.0;
const SIDEBAR_WIDTH_PX: f32 = 420.0;
const SIDEBAR_COLLAPSED_HEIGHT_PX: f32 = 96.0;
const PLAYERS_SECTION_HEIGHT_PX: f32 = 160.0;
const OBSERVERS_SECTION_HEIGHT_PX: f32 = 140.0;
const MESSAGES_SECTION_HEIGHT_PX: f32 = 180.0;
const MESSAGES_SECTION_COLLAPSED_HEIGHT_PX: f32 = 32.0;
const PANEL_TITLE_FONT_SIZE: f32 = 18.0;
const LIST_ITEM_FONT_SIZE: f32 = 14.0;
const MESSAGE_INPUT_HEIGHT_PX: f32 = 34.0;
const FLOAT_BUTTON_WIDTH_PX: f32 = 160.0;
const FLOAT_BUTTON_HEIGHT_PX: f32 = 40.0;
const LINE_SCROLL_HEIGHT: f32 = 20.0;

const COLOR_ROOT_BG: Color = Color::srgba(0.03, 0.05, 0.08, 0.55);
const COLOR_PANEL_BG: Color = Color::srgba(0.07, 0.10, 0.14, 0.96);
const COLOR_LIST_BG: Color = Color::srgba(0.12, 0.14, 0.18, 0.95);
const COLOR_TAB_INACTIVE: Color = Color::srgb(0.15, 0.18, 0.22);
const COLOR_TEXT_PRIMARY: Color = Color::WHITE;
const COLOR_TEXT_SECONDARY: Color = Color::srgb(0.75, 0.78, 0.84);
const COLOR_READY_ON: Color = Color::srgb(0.16, 0.52, 0.26);
const COLOR_READY_OFF: Color = Color::srgb(0.50, 0.25, 0.18);
const COLOR_LEAVE: Color = Color::srgb(0.60, 0.20, 0.20);
const COLOR_AVATAR: Color = Color::srgb(0.36, 0.45, 0.61);
const COLOR_START_DISABLED: Color = Color::srgb(0.20, 0.24, 0.28);
const COLOR_START_ENABLED: Color = Color::srgb(0.18, 0.42, 0.25);

pub struct RoomUiPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomUiInputSystemSet;

#[derive(Component)]
pub struct RoomUi;

#[derive(Component)]
struct RoomUiSidebar;

#[derive(Component)]
struct RoomUiToggleSidebarButton;

#[derive(Component)]
struct RoomUiSidebarBody;

#[derive(Component)]
struct RoomUiPlayersPanel;

#[derive(Component)]
struct RoomUiObserversPanel;

#[derive(Component)]
struct RoomUiMessagesPanel;

#[derive(Component)]
struct RoomUiReadyButton;

#[derive(Component)]
struct RoomUiReadyButtonLabel;

#[derive(Component)]
struct RoomUiLeaveButton;

#[derive(Component)]
struct RoomUiStartGameButton;

#[derive(Component)]
struct RoomUiStartGameLabel;

#[derive(Component)]
struct RoomUiChatInput;

#[derive(Component)]
struct RoomUiScrollableList;

#[derive(Resource, Debug, Clone, Copy)]
struct RoomUiLocalState {
    is_sidebar_open: bool,
}

impl Default for RoomUiLocalState {
    fn default() -> Self {
        Self {
            is_sidebar_open: true,
        }
    }
}

#[derive(Component)]
struct RoomUiKickButton {
    target_user_id: String,
}

#[derive(Component)]
struct RoomUiSitButton {
    position: String,
}

#[derive(Component)]
struct RoomUiAddBotButton {
    position: String,
}

#[derive(Resource, Debug, Clone, Copy)]
struct RoomUiListEntities {
    players_list: Entity,
    observers_list: Entity,
    messages_list: Entity,
}

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct RoomUiScroll {
    entity: Entity,
    delta: Vec2,
}

#[derive(Debug, Clone, Copy)]
struct RoomUiPanelEntities {
    panel: Entity,
    list: Entity,
}

impl Plugin for RoomUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoomUiLocalState>()
            .add_systems(OnEnter(MainState::GameRoom), setup_room_ui)
            .add_systems(OnExit(MainState::GameRoom), cleanup_room_ui)
            .add_systems(
                Update,
                (
                    room_ui_toggle_sidebar_system,
                    room_ui_sync_from_snapshot,
                    room_ui_sync_ready_button_system,
                    room_ui_ready_button_system,
                    room_ui_sync_start_game_button_system,
                    room_ui_start_game_button_system,
                    room_ui_leave_button_system,
                    room_ui_sit_button_system,
                    room_ui_add_bot_button_system,
                    room_ui_submit_chat_system,
                    room_ui_kick_button_system,
                    send_scroll_events,
                )
                    .in_set(RoomUiInputSystemSet)
                    .run_if(in_state(MainState::GameRoom)),
            )
            .add_observer(on_scroll_handler);
    }
}

fn setup_room_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut room_ui_state: ResMut<RoomUiLocalState>,
) {
    room_ui_state.is_sidebar_open = true;

    let font = asset_server.load("fonts/FiraSans-Black.ttf");

    let root_entity = commands
        .spawn((
            RoomUi,
            Ui,
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::End,
                padding: UiRect {
                    left: px(ROOT_PADDING_PX),
                    right: px(ROOT_PADDING_PX),
                    top: px(ROOT_PADDING_PX + TOP_BAR_HEIGHT_PX),
                    bottom: px(ROOT_PADDING_PX),
                },
                ..default()
            },
            BackgroundColor(COLOR_ROOT_BG),
        ))
        .id();

    let sidebar_entity = commands
        .spawn((
            RoomUiSidebar,
            Node {
                width: px(SIDEBAR_WIDTH_PX),
                height: percent(100),
                padding: UiRect::all(px(10.0)),
                row_gap: px(8.0),
                flex_direction: FlexDirection::Column,
                border_radius: BorderRadius::ZERO,
                ..default()
            },
            BackgroundColor(COLOR_PANEL_BG),
        ))
        .id();

    let sidebar_body_entity = commands
        .spawn((
            RoomUiSidebarBody,
            Node {
                width: percent(100),
                flex_grow: 1.0,
                row_gap: px(10.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();

    let mut panel_container_entity: Option<Entity> = None;

    commands.entity(sidebar_entity).with_children(|parent| {
        parent
            .spawn(Node {
                width: percent(100),
                height: px(34.0),
                column_gap: px(8.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|bar| {
                bar.spawn(title_text_bundle("Room Sidebar", font.clone()));
                bar.spawn((
                    Button,
                    RoomUiToggleSidebarButton,
                    Node {
                        width: px(94.0),
                        height: px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::ZERO,
                        ..default()
                    },
                    BackgroundColor(COLOR_TAB_INACTIVE),
                ))
                .with_children(|toggle_button| {
                    toggle_button.spawn(normal_text_bundle("Toggle".to_string(), font.clone()));
                });
            });

        panel_container_entity = Some(sidebar_body_entity);
    });

    let panel_container = panel_container_entity.expect("room ui panel container should exist");

    let players_panel = spawn_scroll_panel(
        &mut commands,
        RoomUiPlayersPanel,
        "Players",
        font.clone(),
        true,
        PLAYERS_SECTION_HEIGHT_PX,
    );

    let observers_panel = spawn_scroll_panel(
        &mut commands,
        RoomUiObserversPanel,
        "Observers",
        font.clone(),
        true,
        OBSERVERS_SECTION_HEIGHT_PX,
    );

    let messages_panel = spawn_scroll_panel(
        &mut commands,
        RoomUiMessagesPanel,
        "Messages",
        font.clone(),
        true,
        MESSAGES_SECTION_HEIGHT_PX,
    );

    commands.entity(panel_container).add_children(&[
        players_panel.panel,
        observers_panel.panel,
        messages_panel.panel,
    ]);

    let chat_input_entity = commands
        .spawn((
            RoomUiChatInput,
            TextInputNode {
                clear_on_submit: false,
                mode: TextInputMode::SingleLine,
                ..default()
            },
            TextInputPrompt::new("Type message and press Enter (placeholder)"),
            TextFont {
                font: font.clone(),
                font_size: LIST_ITEM_FONT_SIZE,
                ..default()
            },
            Node {
                width: percent(100),
                height: px(MESSAGE_INPUT_HEIGHT_PX),
                padding: UiRect::horizontal(px(8.0)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::ZERO,
                ..default()
            },
            BackgroundColor(COLOR_LIST_BG),
        ))
        .id();

    commands
        .entity(panel_container)
        .add_children(&[chat_input_entity]);

    let float_button_group = commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            right: px(ROOT_PADDING_PX),
            bottom: px(ROOT_PADDING_PX),
            column_gap: px(12.0),
            ..default()
        },))
        .id();

    commands.entity(float_button_group).with_children(|parent| {
        parent
            .spawn((
                Button,
                RoomUiReadyButton,
                Node {
                    width: px(FLOAT_BUTTON_WIDTH_PX),
                    height: px(FLOAT_BUTTON_HEIGHT_PX),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::ZERO,
                    ..default()
                },
                BackgroundColor(COLOR_READY_OFF),
            ))
            .with_children(|button| {
                button.spawn((
                    RoomUiReadyButtonLabel,
                    Text::new("Ready"),
                    TextFont {
                        font: font.clone(),
                        font_size: LIST_ITEM_FONT_SIZE,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_SECONDARY),
                ));
            });

        parent
            .spawn((
                Button,
                RoomUiStartGameButton,
                Node {
                    width: px(FLOAT_BUTTON_WIDTH_PX),
                    height: px(FLOAT_BUTTON_HEIGHT_PX),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::ZERO,
                    ..default()
                },
                BackgroundColor(COLOR_START_DISABLED),
            ))
            .with_children(|button| {
                button.spawn((
                    RoomUiStartGameLabel,
                    Text::new("Start Game"),
                    TextFont {
                        font: font.clone(),
                        font_size: LIST_ITEM_FONT_SIZE,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_SECONDARY),
                ));
            });

        parent
            .spawn((
                Button,
                RoomUiLeaveButton,
                Node {
                    width: px(FLOAT_BUTTON_WIDTH_PX),
                    height: px(FLOAT_BUTTON_HEIGHT_PX),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::ZERO,
                    ..default()
                },
                BackgroundColor(COLOR_LEAVE),
            ))
            .with_children(|button| {
                button.spawn(normal_text_bundle("Leave Room".to_string(), font.clone()));
            });
    });

    commands
        .entity(root_entity)
        .add_children(&[sidebar_entity, float_button_group]);

    commands
        .entity(sidebar_entity)
        .add_children(&[sidebar_body_entity]);

    commands.insert_resource(RoomUiListEntities {
        players_list: players_panel.list,
        observers_list: observers_panel.list,
        messages_list: messages_panel.list,
    });
}

fn title_text_bundle(text: &str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font,
            font_size: PANEL_TITLE_FONT_SIZE,
            ..default()
        },
        TextColor(COLOR_TEXT_PRIMARY),
    )
}

fn normal_text_bundle(text: impl Into<String>, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font,
            font_size: LIST_ITEM_FONT_SIZE,
            ..default()
        },
        TextColor(COLOR_TEXT_SECONDARY),
    )
}

fn room_observer_view_label(view: &RoomObserverView) -> String {
    match view {
        RoomObserverView::Neutral => "Neutral".to_string(),
        RoomObserverView::Position(position) => format!("Seat {}", position),
        RoomObserverView::Player(_) => "Following Player".to_string(),
    }
}

fn spawn_scroll_panel<Tag: Component>(
    commands: &mut Commands,
    tag: Tag,
    title: &str,
    font: Handle<Font>,
    is_active: bool,
    section_height_px: f32,
) -> RoomUiPanelEntities {
    let panel_entity = commands
        .spawn((
            tag,
            Node {
                width: percent(100),
                height: px(section_height_px),
                flex_grow: 0.0,
                display: if is_active {
                    Display::Flex
                } else {
                    Display::None
                },
                row_gap: px(8.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();

    let title_entity = commands.spawn(title_text_bundle(title, font.clone())).id();
    let list_entity = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                padding: UiRect::all(px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                overflow: Overflow::scroll_y(),
                border_radius: BorderRadius::ZERO,
                ..default()
            },
            RoomUiScrollableList,
            ScrollPosition::default(),
            BackgroundColor(COLOR_LIST_BG),
        ))
        .id();

    commands
        .entity(panel_entity)
        .add_children(&[title_entity, list_entity]);

    RoomUiPanelEntities {
        panel: panel_entity,
        list: list_entity,
    }
}

fn spawn_user_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    left_tag: String,
    username: String,
    status: String,
    kick_target_user_id: Option<String>,
) {
    parent
        .spawn(Node {
            width: percent(100),
            min_height: px(42.0),
            padding: UiRect::axes(px(8.0), px(4.0)),
            display: Display::Grid,
            grid_template_columns: vec![
                GridTrack::px(10.0),
                GridTrack::px(20.0),
                GridTrack::flex(1.0),
                GridTrack::px(90.0),
                GridTrack::px(96.0),
            ],
            column_gap: px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(18.0),
                    height: px(18.0),
                    border_radius: BorderRadius::ZERO,
                    ..default()
                },
                BackgroundColor(COLOR_AVATAR),
            ));
            row.spawn(normal_text_bundle(left_tag, font.clone()));
            row.spawn(normal_text_bundle(username, font.clone()));
            row.spawn(normal_text_bundle(status, font.clone()));
            if let Some(target_user_id) = kick_target_user_id {
                row.spawn((
                    Button,
                    RoomUiKickButton { target_user_id },
                    Node {
                        width: px(96.0),
                        height: px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::ZERO,
                        ..default()
                    },
                    BackgroundColor(COLOR_LEAVE),
                ))
                .with_children(|button| {
                    button.spawn(normal_text_bundle("Kick".to_string(), font));
                });
            } else {
                row.spawn(normal_text_bundle(String::new(), font));
            }
        });
}

fn spawn_vacant_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    left_tag: String,
    position: RoomPlayerPosition,
    can_sit: bool,
    can_add_bot: bool,
) {
    parent
        .spawn(Node {
            width: percent(100),
            min_height: px(42.0),
            padding: UiRect::axes(px(8.0), px(4.0)),
            display: Display::Grid,
            grid_template_columns: vec![
                GridTrack::px(10.0),
                GridTrack::px(20.0),
                GridTrack::flex(1.0),
                GridTrack::px(90.0),
                GridTrack::px(200.0),
            ],
            column_gap: px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(18.0),
                    height: px(18.0),
                    border_radius: BorderRadius::ZERO,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.28, 0.30, 0.34)),
            ));
            row.spawn(normal_text_bundle(left_tag, font.clone()));
            row.spawn(normal_text_bundle("<empty>".to_string(), font.clone()));
            row.spawn(normal_text_bundle("Vacant".to_string(), font.clone()));
            row.spawn((Node {
                width: px(200.0),
                height: px(24.0),
                column_gap: px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },))
                .with_children(|actions| {
                    if can_sit {
                        actions
                            .spawn((
                                Button,
                                RoomUiSitButton {
                                    position: position.to_string(),
                                },
                                Node {
                                    width: px(96.0),
                                    height: px(24.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border_radius: BorderRadius::ZERO,
                                    ..default()
                                },
                                BackgroundColor(COLOR_READY_ON),
                            ))
                            .with_children(|button| {
                                button.spawn(normal_text_bundle("Sit".to_string(), font.clone()));
                            });
                    }

                    if can_add_bot {
                        actions
                            .spawn((
                                Button,
                                RoomUiAddBotButton {
                                    position: position.to_string(),
                                },
                                Node {
                                    width: px(96.0),
                                    height: px(24.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border_radius: BorderRadius::ZERO,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.18, 0.32, 0.58)),
                            ))
                            .with_children(|button| {
                                button
                                    .spawn(normal_text_bundle("AddBot".to_string(), font.clone()));
                            });
                    }

                    if !can_sit && !can_add_bot {
                        actions.spawn(normal_text_bundle(String::new(), font.clone()));
                    }
                });
        });
}

fn room_ui_sync_from_snapshot(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    room_ui_state: Res<RoomUiLocalState>,
    list_entities: Res<RoomUiListEntities>,
) {
    if !room_snapshot.is_changed() && !room_ui_state.is_changed() {
        return;
    }

    let Some(room) = room_snapshot.room.as_ref() else {
        return;
    };

    let font = asset_server.load("fonts/FiraSans-Black.ttf");
    let current_user_id = current_user_info.user.as_ref().map(|user| user.id.clone());
    let is_owner = current_user_id
        .as_ref()
        .is_some_and(|user_id| *user_id == room.info.owner);
    let current_player_position = current_user_id
        .as_ref()
        .and_then(|user_id| room.state.find_player_position(user_id));

    commands
        .entity(list_entities.players_list)
        .despawn_children();
    commands
        .entity(list_entities.observers_list)
        .despawn_children();
    commands
        .entity(list_entities.messages_list)
        .despawn_children();

    let mut seats: Vec<_> = room.state.positions.seats.iter().collect();
    seats.sort_by(|(left_position, _), (right_position, _)| {
        left_position.as_str().cmp(right_position.as_str())
    });

    commands
        .entity(list_entities.players_list)
        .with_children(|list| {
            for (position, seat_state) in seats {
                match seat_state {
                    SeatState::Occupied(player_state) => {
                        let status = format!(
                            "{} {}",
                            if player_state.id_ready {
                                "Ready"
                            } else {
                                "NotReady"
                            },
                            if player_state.is_connected {
                                "Online"
                            } else {
                                "Offline"
                            }
                        );
                        let can_kick = is_owner && player_state.player.id != room.info.owner;
                        let kick_target = if can_kick {
                            Some(player_state.player.id.to_string())
                        } else {
                            None
                        };
                        spawn_user_row(
                            list,
                            font.clone(),
                            format!("[P{}]", position),
                            player_state.player.nickname.clone(),
                            status,
                            kick_target,
                        );
                    }
                    SeatState::Vacant => {
                        let can_sit = current_player_position.is_none();
                        let can_add_bot = is_owner;
                        spawn_vacant_row(
                            list,
                            font.clone(),
                            format!("[P{}]", position),
                            position.clone(),
                            can_sit,
                            can_add_bot,
                        );
                    }
                }
            }
        });

    let mut observers: Vec<_> = room.state.observers.iter().collect();
    observers.sort_by(|(_, left_state), (_, right_state)| {
        left_state.player.nickname.cmp(&right_state.player.nickname)
    });
    commands
        .entity(list_entities.observers_list)
        .with_children(|list| {
            for (_user_id, observer_state) in observers {
                let status = if observer_state.is_connected {
                    "Watching Online"
                } else {
                    "Watching Offline"
                }
                .to_string();
                let can_kick = is_owner && observer_state.player.id != room.info.owner;
                let kick_target = if can_kick {
                    Some(observer_state.player.id.to_string())
                } else {
                    None
                };
                spawn_user_row(
                    list,
                    font.clone(),
                    format!("[O {}]", room_observer_view_label(&observer_state.view)),
                    observer_state.player.nickname.clone(),
                    status,
                    kick_target,
                );
            }
        });

    commands
        .entity(list_entities.messages_list)
        .with_children(|list| {
            if room_ui_state.is_sidebar_open {
                for message in room_snapshot.messages.iter().rev().take(60).rev() {
                    list.spawn(normal_text_bundle(message.clone(), font.clone()));
                }
            } else {
                let latest = room_snapshot
                    .messages
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "<no messages yet>".to_string());
                list.spawn(normal_text_bundle(latest, font.clone()));
            }
        });
}

fn room_ui_toggle_sidebar_system(
    mut room_ui_state: ResMut<RoomUiLocalState>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<RoomUiToggleSidebarButton>)>,
    mut room_ui_nodes: ParamSet<(
        Query<&mut Node, With<RoomUiSidebar>>,
        Query<&mut Node, With<RoomUiSidebarBody>>,
        Query<&mut Node, With<RoomUiPlayersPanel>>,
        Query<&mut Node, With<RoomUiObserversPanel>>,
        Query<&mut Node, With<RoomUiMessagesPanel>>,
        Query<&mut Node, With<RoomUiChatInput>>,
    )>,
) {
    for interaction in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        room_ui_state.is_sidebar_open = !room_ui_state.is_sidebar_open;
        for mut node in room_ui_nodes.p0().iter_mut() {
            if room_ui_state.is_sidebar_open {
                node.width = px(SIDEBAR_WIDTH_PX);
                node.height = percent(100);
            } else {
                node.width = px(SIDEBAR_WIDTH_PX);
                node.height = px(SIDEBAR_COLLAPSED_HEIGHT_PX);
            }
        }

        for mut body_node in room_ui_nodes.p1().iter_mut() {
            body_node.display = Display::Flex;
        }

        let collapsed = !room_ui_state.is_sidebar_open;
        for mut players_panel in room_ui_nodes.p2().iter_mut() {
            players_panel.display = if collapsed {
                Display::None
            } else {
                Display::Flex
            };
        }
        for mut observers_panel in room_ui_nodes.p3().iter_mut() {
            observers_panel.display = if collapsed {
                Display::None
            } else {
                Display::Flex
            };
        }
        for mut chat_input in room_ui_nodes.p5().iter_mut() {
            chat_input.display = if collapsed {
                Display::None
            } else {
                Display::Flex
            };
        }
        for mut messages_panel in room_ui_nodes.p4().iter_mut() {
            messages_panel.height = if collapsed {
                px(MESSAGES_SECTION_COLLAPSED_HEIGHT_PX)
            } else {
                px(MESSAGES_SECTION_HEIGHT_PX)
            };
        }
    }
}

fn room_ui_ready_button_system(
    runtime: Res<ConnectionRuntime>,
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<RoomUiReadyButton>)>,
) {
    for interaction in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let my_ready_state = current_user_info
            .user
            .as_ref()
            .and_then(|user| {
                room_snapshot.room.as_ref().and_then(|room| {
                    room.state
                        .iter_players()
                        .find(|(_, player_state)| player_state.player.id == user.id)
                        .map(|(_, player_state)| player_state.id_ready)
                })
            })
            .unwrap_or(false);

        let next_ready = !my_ready_state;
        runtime.send_action(ActionData::RoomAction(RoomActionData::ChangeReadyState(
            ReadyStateChange {
                is_ready: next_ready,
            },
        )));
    }
}

fn room_ui_sync_ready_button_system(
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    mut ready_button_bg_query: Query<&mut BackgroundColor, With<RoomUiReadyButton>>,
    mut ready_text_query: Query<&mut Text, With<RoomUiReadyButtonLabel>>,
) {
    if !room_snapshot.is_changed() && !current_user_info.is_changed() {
        return;
    }

    let my_ready_state = current_user_info
        .user
        .as_ref()
        .and_then(|user| {
            room_snapshot.room.as_ref().and_then(|room| {
                room.state
                    .iter_players()
                    .find(|(_, player_state)| player_state.player.id == user.id)
                    .map(|(_, player_state)| player_state.id_ready)
            })
        })
        .unwrap_or(false);

    for mut bg in ready_button_bg_query.iter_mut() {
        *bg = BackgroundColor(if my_ready_state {
            COLOR_READY_ON
        } else {
            COLOR_READY_OFF
        });
    }

    for mut text in ready_text_query.iter_mut() {
        text.0 = if my_ready_state {
            "Not Ready".to_string()
        } else {
            "Ready".to_string()
        };
    }
}

fn room_ui_sit_button_system(
    runtime: Res<ConnectionRuntime>,
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    interaction_query: Query<
        (&Interaction, &RoomUiSitButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    let Some(current_user) = current_user_info.user.as_ref() else {
        return;
    };
    let Some(room) = room_snapshot.room.as_ref() else {
        return;
    };

    if room.state.find_player_position(&current_user.id).is_some() {
        return;
    }

    for (interaction, sit_button) in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let target_position = RoomPlayerPosition::from(sit_button.position.as_str());
        runtime.send_action(ActionData::RoomAction(RoomActionData::PositionChange(
            PositionChange {
                from: RoomUserPosition::Observer(RoomObserverView::default()),
                to: RoomUserPosition::Player(target_position),
            },
        )));
    }
}

fn room_ui_add_bot_button_system(
    runtime: Res<ConnectionRuntime>,
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    interaction_query: Query<
        (&Interaction, &RoomUiAddBotButton),
        (Changed<Interaction>, With<RoomUiAddBotButton>),
    >,
) {
    let Some(current_user) = current_user_info.user.as_ref() else {
        return;
    };
    let Some(room) = room_snapshot.room.as_ref() else {
        return;
    };

    if room.info.owner != current_user.id {
        return;
    }

    for (interaction, add_bot_button) in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        runtime.send_action(ActionData::RoomAction(RoomActionData::RoomManage(
            RoomManage::AddBot(AddBot {
                position: RoomPlayerPosition::from(add_bot_button.position.as_str()),
                name: None,
            }),
        )));
    }
}

fn room_ui_leave_button_system(
    runtime: Res<ConnectionRuntime>,
    mut next_state: ResMut<NextState<MainState>>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<RoomUiLeaveButton>)>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            runtime.send_action(ActionData::RoomAction(RoomActionData::Leave));
            next_state.set(MainState::Lobby);
        }
    }
}

fn can_start_game(
    room: &openplay_basic::room::Room,
    current_user: &openplay_basic::user::User,
) -> bool {
    if room.info.owner != current_user.id {
        return false;
    }

    if !matches!(
        room.state.phase.kind,
        openplay_basic::room::RoomPhaseKind::Waiting
    ) {
        return false;
    }

    let occupied = room.state.player_count();
    let seat_ok = match room.state.positions.mode {
        PositionMode::Fixed => occupied == room.state.positions.seats.len(),
        PositionMode::Flexible {
            min_players,
            max_players,
        } => occupied >= min_players && occupied <= max_players,
    };

    seat_ok && room.state.all_players_ready()
}

fn room_ui_sync_start_game_button_system(
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    mut button_bg_query: Query<&mut BackgroundColor, With<RoomUiStartGameButton>>,
    mut label_query: Query<&mut Text, With<RoomUiStartGameLabel>>,
) {
    if !room_snapshot.is_changed() && !current_user_info.is_changed() {
        return;
    }

    let enabled = match (room_snapshot.room.as_ref(), current_user_info.user.as_ref()) {
        (Some(room), Some(user)) => can_start_game(room, user),
        _ => false,
    };

    for mut bg in button_bg_query.iter_mut() {
        *bg = BackgroundColor(if enabled {
            COLOR_START_ENABLED
        } else {
            COLOR_START_DISABLED
        });
    }

    for mut text in label_query.iter_mut() {
        text.0 = if enabled {
            "Start Game".to_string()
        } else {
            "Start Game (Disabled)".to_string()
        };
    }
}

fn room_ui_start_game_button_system(
    runtime: Res<ConnectionRuntime>,
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<RoomUiStartGameButton>)>,
) {
    let enabled = match (room_snapshot.room.as_ref(), current_user_info.user.as_ref()) {
        (Some(room), Some(user)) => can_start_game(room, user),
        _ => false,
    };

    if !enabled {
        return;
    }

    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            runtime.send_action(ActionData::RoomAction(RoomActionData::RoomManage(
                RoomManage::StartGame,
            )));
        }
    }
}

fn room_ui_submit_chat_system(
    runtime: Res<ConnectionRuntime>,
    mut submit_reader: MessageReader<SubmitText>,
    chat_query: Query<Entity, With<RoomUiChatInput>>,
) {
    for submit in submit_reader.read() {
        if chat_query.get(submit.entity).is_err() {
            continue;
        }

        let message = submit.text.trim();
        if message.is_empty() {
            continue;
        }

        runtime.send_action(ActionData::RoomAction(RoomActionData::Chat(
            openplay_basic::room::Chat {
                message: vec![RoomMessageSegment::Text(message.to_string())],
            },
        )));
    }
}

fn room_ui_kick_button_system(
    runtime: Res<ConnectionRuntime>,
    interaction_query: Query<
        (&Interaction, &RoomUiKickButton),
        (Changed<Interaction>, With<RoomUiKickButton>),
    >,
) {
    for (interaction, kick_button) in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Ok(target_user_id) =
            openplay_basic::user::UserId::try_from(kick_button.target_user_id.as_str())
        else {
            continue;
        };

        runtime.send_action(ActionData::RoomAction(RoomActionData::RoomManage(
            RoomManage::KickOut(KickOut {
                player: target_user_id,
                reason: None,
                ban: None,
            }),
        )));
    }
}

fn send_scroll_events(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    scrollable_lists: Query<Entity, With<RoomUiScrollableList>>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if mouse_wheel.unit == MouseScrollUnit::Line {
            delta *= LINE_SCROLL_HEIGHT;
        }

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }

        let mut has_hovered_target = false;
        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                has_hovered_target = true;
                commands.trigger(RoomUiScroll { entity, delta });
            }
        }

        if !has_hovered_target {
            for entity in scrollable_lists.iter() {
                commands.trigger(RoomUiScroll { entity, delta });
            }
        }
    }
}

fn on_scroll_handler(
    mut scroll: On<RoomUiScroll>,
    mut scrollable_nodes: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let Ok((mut scroll_position, node, computed_node)) = scrollable_nodes.get_mut(scroll.entity)
    else {
        return;
    };

    let max_offset = (computed_node.content_size() - computed_node.size())
        * computed_node.inverse_scale_factor();

    let delta = &mut scroll.delta;

    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0.0 {
        let max_reached = if delta.x > 0.0 {
            scroll_position.x >= max_offset.x
        } else {
            scroll_position.x <= 0.0
        };

        if !max_reached {
            scroll_position.x = (scroll_position.x + delta.x).clamp(0.0, max_offset.x.max(0.0));
            delta.x = 0.0;
        }
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0.0 {
        let max_reached = if delta.y > 0.0 {
            scroll_position.y >= max_offset.y
        } else {
            scroll_position.y <= 0.0
        };

        if !max_reached {
            scroll_position.y = (scroll_position.y + delta.y).clamp(0.0, max_offset.y.max(0.0));
            delta.y = 0.0;
        }
    }

    if *delta == Vec2::ZERO {
        scroll.propagate(false);
    }
}

pub fn cleanup_room_ui(mut commands: Commands, query: Query<Entity, With<RoomUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_children();
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<RoomUiListEntities>();
}
