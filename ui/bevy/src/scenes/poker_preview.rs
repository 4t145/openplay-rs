use crate::MainState;
use crate::game_components::poker::{PokerCard, PokerCardBundle};
use crate::state::OverlayState;
use crate::ui::Hud;
use bevy::{asset, prelude::*};
use openplay_poker::{Rank, Suit};

pub struct PokerPreviewPlugin;

impl Plugin for PokerPreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(OverlayState::Theme), setup_preview)
            .add_systems(OnExit(OverlayState::Theme), cleanup_preview)
            .add_systems(
                Update,
                input_handler.run_if(in_state(OverlayState::Theme)),
            )
            .add_systems(
                Update,
                close_button_handler.run_if(in_state(OverlayState::Theme)),
            );
    }
}

#[derive(Component)]
struct PokerPreviewScene; // Tag for cleanup

fn setup_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut asset_server: ResMut<AssetServer>,
) {
    // Lights
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            range: 100.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 10.0),
        PokerPreviewScene,
    ));

    let x_gap = 1.2;
    let y_gap = 1.6;
    let cols = 14;
    let rows = 4;

    let total_width = (cols as f32) * x_gap;
    let total_height = (rows as f32) * y_gap;
    let x_start = -(total_width / 2.0) + (x_gap / 2.0);
    let y_start = (total_height / 2.0) - (y_gap / 2.0);

    let suits = [Suit::Spades, Suit::Hearts, Suit::Clubs, Suit::Diamonds];
    let ranks = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    // Spawn 13x4 grid
    for (row_idx, suit) in suits.iter().enumerate() {
        for (col_idx, rank) in ranks.iter().enumerate() {
            let poker_card = PokerCard {
                card: openplay_poker::Card::new_natural(*suit, *rank),
                face_up: true,
            };

            let mut bundle = PokerCardBundle::new(poker_card, meshes.as_mut(), materials.as_mut());
            bundle.transform = Transform::from_translation(Vec3::new(
                x_start + col_idx as f32 * x_gap,
                y_start - row_idx as f32 * y_gap,
                0.0,
            ));

            commands.spawn((bundle, PokerPreviewScene));
        }

        // Extras column (14th column, index 13)
        let extra_card = match row_idx {
            0 => Some(openplay_poker::Card::RedJoker),
            1 => Some(openplay_poker::Card::BlackJoker),
            2 => Some(openplay_poker::Card::WildCard),
            3 => None, // Back
            _ => None,
        };

        if let Some(card) = extra_card {
            let poker_card = PokerCard {
                card,
                face_up: true,
            };
            let mut bundle = PokerCardBundle::new(poker_card, meshes.as_mut(), materials.as_mut());
            bundle.transform = Transform::from_translation(Vec3::new(
                x_start + 13.0 * x_gap,
                y_start - row_idx as f32 * y_gap,
                0.0,
            ));
            commands.spawn((bundle, PokerPreviewScene));
        } else if row_idx == 3 {
            // Back Card
            let poker_card = PokerCard {
                card: openplay_poker::Card::new_natural(Suit::Spades, Rank::Ace),
                face_up: false,
            };
            let mut bundle = PokerCardBundle::new(poker_card, meshes.as_mut(), materials.as_mut());
            bundle.transform = Transform::from_translation(Vec3::new(
                x_start + 13.0 * x_gap,
                y_start - row_idx as f32 * y_gap,
                0.0,
            ));
            commands.spawn((bundle, PokerPreviewScene));
        }
    }

    commands.spawn((
        Hud,
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::FlexEnd,
            ..default()
        },
        children![(
            PokerPreviewCloseButton,
            Button,
            Node {
                width: Val::Px(30.0),
                height: Val::Px(30.0),
                border: UiRect::all(px(5)),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BackgroundColor(Color::BLACK),
            children![(
                Text::new("X"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Black.ttf"),
                    font_size: 33.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            )]
        ),],
        PokerPreviewScene,
    ));
}

#[derive(Component)]
pub struct PokerPreviewCloseButton;
fn cleanup_preview(mut commands: Commands, query: Query<Entity, With<PokerPreviewScene>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn input_handler(mut next_state: ResMut<NextState<OverlayState>>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(OverlayState::None);
    }
}

fn close_button_handler(
    mut next_state: ResMut<NextState<OverlayState>>,
    mut interaction_query: Query<
        (&Interaction, &mut Button),
        (
            Changed<Interaction>,
            With<Button>,
            With<PokerPreviewCloseButton>,
        ),
    >,
) {
    for (interaction, mut button) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(OverlayState::None);
            button.set_changed(); // Mark button as changed to update its state
        }
    }
}
