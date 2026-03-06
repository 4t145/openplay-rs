mod data;
mod game_components;
mod global_config;
mod scenes;
mod state;
mod ui;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_ui_text_input::TextInputPlugin;

use crate::state::{MainState, OverlayState};

#[derive(AssetCollection, Resource)]
pub struct GlobalAssets {
    #[asset(texture_atlas_layout(
        tile_size_x = 64,
        tile_size_y = 64,
        columns = 14,
        rows = 4,
        padding_x = 1,
        padding_y = 1
    ))]
    pub default_poker_texture_atlas_layout: Handle<TextureAtlasLayout>,
    #[asset(path = "cardsLarge_tilemap.png")]
    pub default_poker_texture_atlas: Handle<Image>,
    #[asset(path = "icons/menu.png")]
    pub icon_menu: Handle<Image>,
    #[asset(path = "icons/dices.png")]
    pub icon_dice: Handle<Image>,
    #[asset(path = "icons/settings.png")]
    pub icon_settings: Handle<Image>,
}

#[derive(Component)]
struct Card;

#[derive(Component, Default)]
struct CardTilt {
    target_rotation: Quat,
}

#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(TextInputPlugin)
        .init_state::<MainState>()
        .init_state::<OverlayState>()
        .add_loading_state(
            LoadingState::new(MainState::GlobalAssetsLoading)
                .continue_to_state(MainState::Lobby)
                .load_collection::<GlobalAssets>(),
        )
        .add_plugins(scenes::ScenesPlugin)
        .add_plugins(game_components::GameComponentsPlugin)
        .add_plugins(ui::UiPlugin)
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, global_config::theme_manager::setup_default_theme)
        .run();
}

fn setup_camera(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // 摄像机：向后移动一段距离，看向原点
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
    ));
}
