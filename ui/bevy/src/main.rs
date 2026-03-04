pub mod game_components;
pub mod ui;

use bevy::prelude::*;
use bevy::remote::RemotePlugin;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::window::PrimaryWindow;
use openplay_poker::{Rank, Suit};

use crate::game_components::poker::{PokerCardBundle, update_poker_card_texture};

const CARD_WIDTH: f32 = 64.0;
const CARD_HEIGHT: f32 = 64.0;

#[derive(Component)]
struct Card;

#[derive(Component, Default)]
struct CardTilt {
    target_rotation: Quat,
}

fn main() {
    App::new()
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Update, update_poker_card_texture)
        .add_systems(Startup, setup)
        .add_systems(Update, (interact_card, animate_card))
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // 摄像机：向后移动一段距离，看向原点
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 由于我们在 poker.rs 里定义的网格大小是 0.8 x 1.12 米 (宽高除以了100)
    // 我们在此调整在 3D 空间中的排布间距
    let cw = 0.8;
    let ch = 1.12;
    let gap = 0.2;

    let start_x = -((13.0 * (cw + gap)) / 2.0);
    let start_y = (4.0 * (ch + gap)) / 2.0;

    for row in 0..4 {
        for col in 0..13 {
            let suit = match row {
                0 => Suit::Clubs,
                1 => Suit::Diamonds,
                2 => Suit::Hearts,
                3 => Suit::Spades,
                _ => unreachable!(),
            };
            let rank = match col {
                0 => Rank::Ace,
                1 => Rank::Two,
                2 => Rank::Three,
                3 => Rank::Four,
                4 => Rank::Five,
                5 => Rank::Six,
                6 => Rank::Seven,
                7 => Rank::Eight,
                8 => Rank::Nine,
                9 => Rank::Ten,
                10 => Rank::Jack,
                11 => Rank::Queen,
                12 => Rank::King,
                _ => unreachable!(),
            };
            let model_card = openplay_poker::Card::new_natural(suit, rank);

            let mut card_bundle = PokerCardBundle::new(model_card, true, &mut meshes);
            
            let x = start_x + (col as f32 * (cw + gap)) + (cw / 2.0);
            let y = start_y - (row as f32 * (ch + gap)) - (ch / 2.0);
            
            // 放置在 z = 0 平面上
            card_bundle.transform = Transform::from_xyz(x, y, 0.0);
            commands.spawn(card_bundle);
        }
    }
}

// fn setup(
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,
//     mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
// ) {
//     commands.spawn(Camera2d);

//     let texture = asset_server.load("cardsLarge_tilemap.png");
//     let layout = TextureAtlasLayout::from_grid(
//         UVec2::new(CARD_WIDTH as u32, CARD_HEIGHT as u32),
//         13,
//         4,
//         None,
//         None,
//     );
//     let texture_atlas_layout = texture_atlas_layouts.add(layout);

//     let start_x = -((13.0 * CARD_WIDTH) / 2.0);
//     let start_y = (4.0 * CARD_HEIGHT) / 2.0;

//     for row in 0..4 {
//         for col in 0..13 {
//             let index = (row * 13 + col) as usize;
//             let x = start_x + (col as f32 * (CARD_WIDTH + 5.0));
//             let y = start_y - (row as f32 * (CARD_HEIGHT + 5.0));

//             commands.spawn((
//                 Sprite::from_atlas_image(
//                     texture.clone(),
//                     TextureAtlas {
//                         layout: texture_atlas_layout.clone(),
//                         index,
//                     },
//                 ),
//                 Transform::from_translation(Vec3::new(x, y, 0.0)),
//                 Card,
//                 CardTilt::default(),
//             ));
//         }
//     }
// }

fn interact_card(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut card_query: Query<(&GlobalTransform, &mut CardTilt), With<Card>>,
) {
    let (camera, camera_transform) = camera_query.single().expect("should have a single camera");
    let window = window_query.single().expect("should have a primary window");

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
    {
        for (card_transform, mut tilt) in card_query.iter_mut() {
            let card_pos = card_transform.translation().truncate();
            let half_size = Vec2::new(CARD_WIDTH / 2.0, CARD_HEIGHT / 2.0);
            let diff = world_position - card_pos;

            if diff.x.abs() < half_size.x && diff.y.abs() < half_size.y {
                let max_angle = 0.7;

                let rot_x = -(diff.y / half_size.y) * max_angle;
                let rot_y = (diff.x / half_size.x) * max_angle;

                tilt.target_rotation = Quat::from_euler(EulerRot::XYZ, rot_x, rot_y, 0.0);
            } else {
                tilt.target_rotation = Quat::IDENTITY;
            }
        }
    }
}

fn animate_card(time: Res<Time>, mut query: Query<(&mut Transform, &CardTilt), With<Card>>) {
    for (mut transform, tilt) in query.iter_mut() {
        transform.rotation = transform
            .rotation
            .slerp(tilt.target_rotation, time.delta_secs() * 20.0);
    }
}
