use bevy::prelude::*;
use openplay_basic::{
    data::Data,
    message::{DataType, TypedData},
    user::{game_action::GameActionData, ActionData},
};
use openplay_doudizhu::{DouDizhuGame, Stage};
use openplay_poker::Card;

use crate::{
    data::CurrentUserInfo,
    game_components::poker::{PokerCard, PokerCardBundle, PokerTextureUpdateSet},
    games::{ActiveGame, GameKind, GameViewSnapshot},
    network::ConnectionRuntime,
    network::RoomSnapshot,
    state::MainState,
};

const PANEL_BG: Color = Color::srgba(0.05, 0.07, 0.1, 0.18);

pub struct DoudizhuScenePlugin;

#[derive(Component)]
struct DoudizhuSceneRoot;

#[derive(Component)]
struct DoudizhuStageText;

#[derive(Component)]
struct DoudizhuTurnText;

#[derive(Component)]
struct DoudizhuCardsRoot;

#[derive(Component)]
struct DoudizhuHandCard {
    _card: Card,
}

#[derive(Component)]
struct DoudizhuLastPlayCard;

#[derive(Component)]
struct BidButton {
    score: u8,
}

#[derive(Component)]
struct PassButton;

#[derive(Component)]
struct PlayLowestButton;

#[derive(Resource, Default, Clone)]
pub struct DoudizhuUiState {
    pub version: Option<u32>,
    pub rendered_version: Option<u32>,
    pub game: Option<DouDizhuGame>,
    pub stage: Option<Stage>,
    pub current_turn: Option<usize>,
    pub my_player_idx: Option<usize>,
    pub can_bid: bool,
    pub can_pass: bool,
    pub can_play: bool,
}

impl Plugin for DoudizhuScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DoudizhuUiState>()
            .add_systems(OnEnter(MainState::Game), setup_doudizhu_scene)
            .add_systems(OnExit(MainState::Game), cleanup_doudizhu_scene)
            .add_systems(
                Update,
                (
                    sync_doudizhu_state,
                    sync_doudizhu_permissions,
                    update_doudizhu_text,
                    sync_action_button_visuals,
                    doudizhu_action_input_system,
                    back_to_room_on_finished,
                )
                    .run_if(in_state(MainState::Game)),
            )
            .add_systems(
                Update,
                sync_doudizhu_hand_cards
                    .run_if(in_state(MainState::Game))
                    .before(PokerTextureUpdateSet),
            );
    }
}

fn setup_doudizhu_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Black.ttf");

    commands.spawn((
        DoudizhuSceneRoot,
        PointLight {
            intensity: 2_200_000.0,
            shadows_enabled: true,
            range: 120.0,
            ..default()
        },
        Transform::from_xyz(0.0, 8.0, 12.0),
    ));

    commands.spawn((
        DoudizhuSceneRoot,
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(PANEL_BG),
        children![
            (
                Node {
                    width: px(520.0),
                    height: px(220.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Text::new("DouDiZhu"),
                        TextFont {
                            font: font.clone(),
                            font_size: 30.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ),
                    (
                        DoudizhuStageText,
                        Text::new("Stage: waiting game view"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ),
                    (
                        DoudizhuTurnText,
                        Text::new("Turn: -"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    )
                ],
            ),
            (
                Node {
                    width: px(520.0),
                    height: px(48.0),
                    column_gap: px(8.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    spawn_action_button(font.clone(), "Bid 0", BidButton { score: 0 }),
                    spawn_action_button(font.clone(), "Bid 1", BidButton { score: 1 }),
                    spawn_action_button(font.clone(), "Bid 2", BidButton { score: 2 }),
                    spawn_action_button(font.clone(), "Bid 3", BidButton { score: 3 }),
                    spawn_action_button(font.clone(), "Pass", PassButton),
                    spawn_action_button(font, "Play Lowest", PlayLowestButton),
                ]
            )
        ],
    ));

    commands.spawn((
        DoudizhuCardsRoot,
        DoudizhuSceneRoot,
        Transform::from_xyz(0.0, -4.2, 1.0),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));
}

fn spawn_action_button<Tag: Component>(font: Handle<Font>, label: &str, tag: Tag) -> impl Bundle {
    (
        Button,
        tag,
        Node {
            width: px(80.0),
            height: px(30.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.18, 0.24, 0.34)),
        children![(
            Text::new(label),
            TextFont {
                font,
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

fn cleanup_doudizhu_scene(mut commands: Commands, query: Query<Entity, With<DoudizhuSceneRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn sync_doudizhu_state(
    active_game: Res<ActiveGame>,
    game_view_snapshot: Res<GameViewSnapshot>,
    mut ui_state: ResMut<DoudizhuUiState>,
) {
    if active_game.kind != Some(GameKind::Doudizhu) || !game_view_snapshot.is_changed() {
        return;
    }

    let Some(latest) = game_view_snapshot.latest.as_ref() else {
        return;
    };

    let typed_data = &latest.data;

    if typed_data.r#type.r#type != "doudizhu_state" {
        return;
    }

    let Ok(game_state) = serde_json::from_slice::<DouDizhuGame>(&typed_data.data) else {
        return;
    };

    ui_state.version = Some(latest.version);
    ui_state.game = Some(game_state.clone());
    ui_state.stage = Some(game_state.stage.clone());
    ui_state.current_turn = Some(game_state.current_turn);
}

fn sync_doudizhu_permissions(
    room_snapshot: Res<RoomSnapshot>,
    current_user_info: Res<CurrentUserInfo>,
    mut ui_state: ResMut<DoudizhuUiState>,
) {
    let Some(game) = ui_state.game.as_ref() else {
        let mut changed = false;
        if ui_state.my_player_idx.is_some() {
            ui_state.my_player_idx = None;
            changed = true;
        }
        if ui_state.can_bid {
            ui_state.can_bid = false;
            changed = true;
        }
        if ui_state.can_pass {
            ui_state.can_pass = false;
            changed = true;
        }
        if ui_state.can_play {
            ui_state.can_play = false;
            changed = true;
        }
        let _ = changed;
        return;
    };
    let game = game.clone();

    let Some(current_user) = current_user_info.user.as_ref() else {
        if ui_state.my_player_idx.is_some() {
            ui_state.my_player_idx = None;
        }
        if ui_state.can_bid {
            ui_state.can_bid = false;
        }
        if ui_state.can_pass {
            ui_state.can_pass = false;
        }
        if ui_state.can_play {
            ui_state.can_play = false;
        }
        return;
    };

    let my_player_idx = game
        .players
        .iter()
        .position(|player| player.player.id == current_user.id)
        .or_else(|| {
            room_snapshot
                .room
                .as_ref()
                .and_then(|room| room.state.find_player_position(&current_user.id))
                .and_then(|position| position.as_str().parse::<usize>().ok())
                .and_then(|seat| seat.checked_sub(1))
        });

    let is_my_turn = my_player_idx.is_some_and(|idx| idx == game.current_turn);
    let next_can_bid = matches!(game.stage, Stage::Bidding) && is_my_turn;
    let next_can_pass = matches!(game.stage, Stage::Playing) && is_my_turn;
    let next_can_play = matches!(game.stage, Stage::Playing)
        && is_my_turn
        && my_player_idx
            .and_then(|idx| game.players.get(idx))
            .is_some_and(|player| !player.hand.is_empty());

    if ui_state.my_player_idx != my_player_idx {
        ui_state.my_player_idx = my_player_idx;
    }
    if ui_state.can_bid != next_can_bid {
        ui_state.can_bid = next_can_bid;
    }
    if ui_state.can_pass != next_can_pass {
        ui_state.can_pass = next_can_pass;
    }
    if ui_state.can_play != next_can_play {
        ui_state.can_play = next_can_play;
    }
}

fn sync_doudizhu_hand_cards(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ui_state: ResMut<DoudizhuUiState>,
    cards_root_query: Query<Entity, With<DoudizhuCardsRoot>>,
    hand_query: Query<Entity, With<DoudizhuHandCard>>,
    last_play_query: Query<Entity, With<DoudizhuLastPlayCard>>,
) {
    let Some(version) = ui_state.version else {
        return;
    };

    if ui_state.rendered_version == Some(version) {
        return;
    }

    let Ok(cards_root) = cards_root_query.single() else {
        return;
    };

    for entity in hand_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in last_play_query.iter() {
        commands.entity(entity).despawn();
    }

    let Some(game) = ui_state.game.as_ref() else {
        return;
    };

    if let Some(my_idx) = ui_state.my_player_idx {
        if let Some(my_player) = game.players.get(my_idx) {
            let card_count = my_player.hand.len();
            let x_gap = 0.7;
            let x_start = -((card_count as f32 - 1.0) * x_gap / 2.0);
            for (idx, card) in my_player.hand.iter().enumerate() {
                let mut bundle = PokerCardBundle::new(
                    PokerCard {
                        card: *card,
                        face_up: true,
                    },
                    meshes.as_mut(),
                    materials.as_mut(),
                );
                bundle.transform = Transform::from_xyz(x_start + idx as f32 * x_gap, 0.0, 0.0);
                let card_entity = commands
                    .spawn((bundle, DoudizhuHandCard { _card: *card }))
                    .id();
                commands.entity(cards_root).add_child(card_entity);
            }
        }
    }

    if let Some(last_play) = game.last_play.as_ref() {
        let card_count = last_play.cards.len();
        let x_gap = 0.8;
        let x_start = -((card_count as f32 - 1.0) * x_gap / 2.0);
        for (idx, card) in last_play.cards.iter().enumerate() {
            let mut bundle = PokerCardBundle::new(
                PokerCard {
                    card: *card,
                    face_up: true,
                },
                meshes.as_mut(),
                materials.as_mut(),
            );
            bundle.transform = Transform::from_xyz(x_start + idx as f32 * x_gap, 2.6, 0.0);
            let card_entity = commands.spawn((bundle, DoudizhuLastPlayCard)).id();
            commands.entity(cards_root).add_child(card_entity);
        }
    }

    ui_state.rendered_version = Some(version);
}

fn send_doudizhu_action(
    runtime: &ConnectionRuntime,
    ui_state: &DoudizhuUiState,
    action: &openplay_doudizhu::Action,
) {
    let Some(ref_version) = ui_state.version else {
        return;
    };

    let Ok(payload) = serde_json::to_vec(action) else {
        return;
    };

    let typed_data = TypedData {
        r#type: DataType {
            app: openplay_doudizhu::get_app(),
            r#type: "action".to_string(),
        },
        codec: "json".to_string(),
        data: Data::from(payload),
    };

    runtime.send_action(ActionData::GameAction(GameActionData {
        message: typed_data,
        ref_version,
    }));
}

fn doudizhu_action_input_system(
    runtime: Res<ConnectionRuntime>,
    ui_state: Res<DoudizhuUiState>,
    bid_query: Query<(&Interaction, &BidButton), (Changed<Interaction>, With<BidButton>)>,
    pass_query: Query<&Interaction, (Changed<Interaction>, With<PassButton>)>,
    play_query: Query<&Interaction, (Changed<Interaction>, With<PlayLowestButton>)>,
) {
    let Some(game) = ui_state.game.as_ref() else {
        return;
    };

    for (interaction, bid) in bid_query.iter() {
        if *interaction == Interaction::Pressed && ui_state.can_bid {
            send_doudizhu_action(
                &runtime,
                &ui_state,
                &openplay_doudizhu::Action::Bid { score: bid.score },
            );
        }
    }

    for interaction in pass_query.iter() {
        if *interaction == Interaction::Pressed && ui_state.can_pass {
            send_doudizhu_action(&runtime, &ui_state, &openplay_doudizhu::Action::Pass);
        }
    }

    for interaction in play_query.iter() {
        if *interaction == Interaction::Pressed && ui_state.can_play {
            let Some(current_player) = game.players.get(game.current_turn) else {
                continue;
            };
            let Some(card) = current_player.hand.last() else {
                continue;
            };
            send_doudizhu_action(
                &runtime,
                &ui_state,
                &openplay_doudizhu::Action::Play { cards: vec![*card] },
            );
        }
    }
}

fn sync_action_button_visuals(
    ui_state: Res<DoudizhuUiState>,
    mut button_queries: ParamSet<(
        Query<&mut BackgroundColor, With<BidButton>>,
        Query<&mut BackgroundColor, With<PassButton>>,
        Query<&mut BackgroundColor, With<PlayLowestButton>>,
    )>,
) {
    if !ui_state.is_changed() {
        return;
    }

    let enabled = Color::srgb(0.18, 0.40, 0.26);
    let disabled = Color::srgb(0.25, 0.26, 0.28);

    for mut bg in button_queries.p0().iter_mut() {
        *bg = BackgroundColor(if ui_state.can_bid { enabled } else { disabled });
    }
    for mut bg in button_queries.p1().iter_mut() {
        *bg = BackgroundColor(if ui_state.can_pass { enabled } else { disabled });
    }
    for mut bg in button_queries.p2().iter_mut() {
        *bg = BackgroundColor(if ui_state.can_play { enabled } else { disabled });
    }
}

fn update_doudizhu_text(
    ui_state: Res<DoudizhuUiState>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<DoudizhuStageText>>,
        Query<&mut Text, With<DoudizhuTurnText>>,
    )>,
) {
    if !ui_state.is_changed() {
        return;
    }

    let stage_text = match ui_state.stage.as_ref() {
        Some(stage) => format!("Stage: {:?}", stage),
        None => "Stage: waiting game view".to_string(),
    };
    let turn_text = match ui_state.current_turn {
        Some(turn) => format!("Turn: {}", turn),
        None => "Turn: -".to_string(),
    };

    for mut text in text_queries.p0().iter_mut() {
        text.0 = stage_text.clone();
    }
    for mut text in text_queries.p1().iter_mut() {
        text.0 = turn_text.clone();
    }
}

fn back_to_room_on_finished(
    active_game: Res<ActiveGame>,
    ui_state: Res<DoudizhuUiState>,
    mut next_state: ResMut<NextState<MainState>>,
) {
    if active_game.kind != Some(GameKind::Doudizhu) {
        return;
    }

    if matches!(ui_state.stage, Some(Stage::Finished)) {
        next_state.set(MainState::GameRoom);
    }
}
