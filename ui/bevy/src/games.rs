use bevy::prelude::*;
use openplay_basic::message::TypedData;

pub mod doudizhu;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKind {
    Doudizhu,
}

impl GameKind {
    pub fn from_app_id(app_id: &str) -> Option<Self> {
        match app_id {
            "doudizhu" => Some(Self::Doudizhu),
            _ => None,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct ActiveGame {
    pub kind: Option<GameKind>,
}

#[derive(Resource, Default, Clone)]
pub struct GameViewSnapshot {
    pub latest: Option<LatestGameView>,
}

#[derive(Debug, Clone)]
pub struct LatestGameView {
    pub version: u32,
    pub data: TypedData,
}

pub struct GamesPlugin;

impl Plugin for GamesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveGame>()
            .init_resource::<GameViewSnapshot>()
            .add_plugins(doudizhu::DoudizhuScenePlugin);
    }
}
