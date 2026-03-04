use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use bevy::prelude::*;

use crate::game_components::poker::PokerCard;

const POKER_WIDTH: f32 = 64.0;
const POKER_HEIGHT: f32 = 64.0;
const ATLAS_COLUMNS: usize = 14;
const ATLAS_ROWS: usize = 4; // Based on file dimensions 909x259 (69*13+12=909, 64*4+3=259)

#[derive(Clone, PartialEq, Eq, Hash, Debug, Component)]
pub struct PokerThemeId(Arc<str>);


impl Default for PokerThemeId  {
    fn default() -> Self {
        Self(Arc::from("default"))
    }
}
impl PokerThemeId {
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }
    
}   

#[derive(Resource, Default)]
pub struct PokerThemeRegistry {
    pub themes: HashMap<PokerThemeId, PokerThemeStore>,
}

impl PokerThemeRegistry {
    pub fn register(&mut self, theme: PokerThemeStore) {
        self.themes.insert(theme.id().clone(), theme);
    }

    pub fn get(&self, id: &PokerThemeId) -> Option<&PokerThemeStore> {
        self.themes.get(id)
    }
}

#[derive(Resource, Clone)]
pub struct PokerThemeStore {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub theme: PokerThemeMeta,
}

#[derive(Clone)]
pub struct PokerThemeMeta {
    pub id: PokerThemeId,
    pub display_name: String,
    pub texture_path: PathBuf,
}

impl PokerThemeStore {
    pub fn id(&self) -> &PokerThemeId {
        &self.theme.id
    }
    pub fn poker_to_sprite(&self, poker: &PokerCard) -> Sprite {
        poker_to_sprite(self, poker)
    }
}
// # SYSTEMS

/// 初始化系统：加载默认皮肤资源
pub fn setup_default_theme(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture: Handle<Image> = asset_server.load("cardsLarge_tilemap.png");

    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(POKER_WIDTH as u32, POKER_HEIGHT as u32),
        ATLAS_COLUMNS as u32,
        ATLAS_ROWS as u32,
        Some(UVec2::new(1, 1)), // Padding between cards
        None,
    );
    let layout_handle = texture_atlas_layouts.add(layout);

    let default_theme = PokerThemeStore {
        texture,
        layout: layout_handle,
        theme: PokerThemeMeta {
            id: PokerThemeId::new("default"),
            display_name: "Default".to_string(),
            texture_path: PathBuf::from("cardsLarge_tilemap.png"),
        },
    };

    // 初始化注册表并添加默认主题
    let mut registry = PokerThemeRegistry::default();
    registry.register(default_theme.clone());

    // 插入全局资源
    commands.insert_resource(registry);
    commands.insert_resource(default_theme);
}

pub fn poker_to_sprite(theme: &PokerThemeStore, poker: &PokerCard) -> Sprite {
    let index = if !poker.face_up {
        27
    } else {
        match poker.card {
            openplay_poker::Card::NaturalCard(natural_card) => {
                let suit_idx = match natural_card.suit {
                    openplay_poker::Suit::Clubs => 0,
                    openplay_poker::Suit::Diamonds => 1,
                    openplay_poker::Suit::Hearts => 2,
                    openplay_poker::Suit::Spades => 3,
                };
                let rank_idx = match natural_card.rank {
                    openplay_poker::Rank::Ace => 0,
                    openplay_poker::Rank::Two => 1,
                    openplay_poker::Rank::Three => 2,
                    openplay_poker::Rank::Four => 3,
                    openplay_poker::Rank::Five => 4,
                    openplay_poker::Rank::Six => 5,
                    openplay_poker::Rank::Seven => 6,
                    openplay_poker::Rank::Eight => 7,
                    openplay_poker::Rank::Nine => 8,
                    openplay_poker::Rank::Ten => 9,
                    openplay_poker::Rank::Jack => 10,
                    openplay_poker::Rank::Queen => 11,
                    openplay_poker::Rank::King => 12,
                };
                suit_idx * ATLAS_COLUMNS + rank_idx
            }
            // Temporarily map special cards to King to avoid crash
            openplay_poker::Card::RedJoker => 41,
            openplay_poker::Card::BlackJoker => 55,
            openplay_poker::Card::WildCard => 13,
        }
    };

    // Safety clamp
    let max_idx = ATLAS_ROWS * ATLAS_COLUMNS - 1;
    let safe_index = index.min(max_idx);

    Sprite::from_atlas_image(
        theme.texture.clone(),
        TextureAtlas {
            layout: theme.layout.clone(),
            index: safe_index,
        },
    )
}
