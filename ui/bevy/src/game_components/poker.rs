use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use fontdue::{Font, FontSettings};
use openplay_poker::{Card, Rank, Suit};
use std::sync::OnceLock;

static FONT: OnceLock<Font> = OnceLock::new();

fn get_font() -> &'static Font {
    FONT.get_or_init(|| {
        let font_data = include_bytes!("../../assets/fonts/FiraSans-Black.ttf") as &[u8];
        Font::from_bytes(font_data, FontSettings::default()).unwrap()
    })
}

#[derive(Debug, Component)]
pub struct PokerCard {
    pub card: openplay_poker::Card,
    pub face_up: bool,
}

#[derive(Component, Default)]
pub struct CardTilt {
    pub target_rotation: Quat,
}

#[derive(Component)]
pub enum PokerCardTexture {
    Atlas {
        texture: Handle<Image>,
        atlas_layout: Handle<TextureAtlasLayout>,
    },
    Default {},
}

impl Default for PokerCardTexture {
    fn default() -> Self {
        Self::Default {}
    }
}

#[derive(Bundle)]
pub struct PokerCardBundle {
    pub tilt: CardTilt,
    pub card: PokerCard,
    pub texture: PokerCardTexture,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl PokerCardBundle {
    pub fn new(card: openplay_poker::Card, face_up: bool, meshes: &mut Assets<Mesh>) -> Self {
        let mesh = meshes.add(Rectangle::new(CARD_WIDTH as f32 / 100.0, CARD_HEIGHT as f32 / 100.0));
        let material = MeshMaterial3d(Handle::default());
        
        Self {
            tilt: CardTilt::default(),
            card: PokerCard { card, face_up },
            texture: PokerCardTexture::Default {},
            mesh: Mesh3d(mesh),
            material,
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}

pub const CARD_WIDTH: u32 = 80;
pub const CARD_HEIGHT: u32 = 112;

pub fn update_poker_card_texture(
    mut query: Query<(&PokerCard, &PokerCardTexture, &mut MeshMaterial3d<StandardMaterial>), Changed<PokerCardTexture>>,
    mut cache: Local<Vec<(openplay_poker::Card, Handle<StandardMaterial>)>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (poker_card, texture, mut material) in query.iter_mut() {
        if let PokerCardTexture::Default {} = texture {
            // Find in fast cache instead of HashMap to bypass non-Hash Card limitation
            let mat_handle = if let Some((_, handle)) = cache.iter().find(|(c, _)| c == &poker_card.card) {
                handle.clone()
            } else {
                let image = generate_poker_texture(&poker_card.card);
                let image_handle = images.add(image);
                let standard_material = StandardMaterial {
                    base_color_texture: Some(image_handle),
                    unlit: true, // Make cards look flat / good without lighting for now, optionally
                    ..default()
                };
                let handle = materials.add(standard_material);
                cache.push((poker_card.card, handle.clone()));
                handle
            };
            
            material.0 = mat_handle;
        }
    }
}

pub struct PokerPlugin;

impl Plugin for PokerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_poker_card_texture);
    }
}

/// 将文字渲染到像素数组中
fn render_text_to_buffer(
    text: &str,
    font_size: f32,
    x_offset: i32,
    y_offset: i32,
    color: (u8, u8, u8),
    buffer: &mut [u8],
    width: i32,
    height: i32,
    invert: bool,
) {
    let font = get_font();
    let mut current_x = x_offset;

    for c in text.chars() {
        let (metrics, bitmap) = font.rasterize(c, font_size);

        let char_y_offset = y_offset
            + (font_size as i32 - metrics.bounds.height as i32 - metrics.bounds.ymin as i32);

        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let pixel_alpha = bitmap[y * metrics.width + x] as f32 / 255.0;

                if pixel_alpha > 0.0 {
                    let mut py = char_y_offset + y as i32;
                    let mut px = current_x + x as i32;

                    // 中心对称翻转坐标
                    if invert {
                        px = width - 1 - px;
                        py = height - 1 - py;
                    }

                    if px >= 0 && px < width && py >= 0 && py < height {
                        let i = ((py * width + px) * 4) as usize;

                        // 预乘Alpha混合
                        let inv_alpha = 1.0 - pixel_alpha;
                        buffer[i] =
                            (color.0 as f32 * pixel_alpha + buffer[i] as f32 * inv_alpha) as u8;
                        buffer[i + 1] =
                            (color.1 as f32 * pixel_alpha + buffer[i + 1] as f32 * inv_alpha) as u8;
                        buffer[i + 2] =
                            (color.2 as f32 * pixel_alpha + buffer[i + 2] as f32 * inv_alpha) as u8;
                        // Alpha channel
                        buffer[i + 3] = (255.0 * pixel_alpha + buffer[i + 3] as f32 * inv_alpha)
                            .min(255.0) as u8;
                    }
                }
            }
        }
        current_x += metrics.advance_width as i32;
    }
}

// 过程化绘制花色形状
fn draw_suit_pixel(suit: Suit, u: f32, v: f32) -> bool {
    match suit {
        Suit::Hearts => {
            let u = u * 1.2;
            let v = v * 1.2 - 0.2; // 稍微上移
            let a = u * u + v * v - 1.0;
            a * a * a + u * u * v * v * v <= 0.0
        }
        Suit::Spades => {
            let u_h = u * 1.2;
            let v_h = -v * 1.2 - 0.1; // 倒置的心形
            let a = u_h * u_h + v_h * v_h - 1.0;
            let is_heart = a * a * a + u_h * u_h * v_h * v_h * v_h <= 0.0;
            // 底部的小手柄
            let is_stem = v > 0.0 && v < 0.9 && u.abs() < 0.05 + v * 0.2;
            is_heart || is_stem
        }
        Suit::Diamonds => u.abs() * 1.5 + v.abs() * 1.2 <= 1.0,
        Suit::Clubs => {
            let c_top = u * u + (v + 0.3) * (v + 0.3) <= 0.16;
            let c_left = (u + 0.35) * (u + 0.35) + (v + 0.1) * (v + 0.1) <= 0.16;
            let c_right = (u - 0.35) * (u - 0.35) + (v + 0.1) * (v + 0.1) <= 0.16;
            let c_center = u * u + (v + 0.1) * (v + 0.1) <= 0.1;
            let is_stem = v > 0.1 && v < 0.9 && u.abs() < 0.05 + v * 0.2;
            c_top || c_left || c_right || c_center || is_stem
        }
    }
}

fn draw_suit_to_buffer(
    suit: Suit,
    cx: f32,
    cy: f32,
    size: f32,
    color: (u8, u8, u8),
    alpha: f32,
    buffer: &mut [u8],
    width: i32,
    height: i32,
    invert: bool,
) {
    let half_size = (size * 1.5) as i32; // 遍历边界
    for dy in -half_size..=half_size {
        for dx in -half_size..=half_size {
            // 通过稍微子采样进行抗锯齿
            let mut coverage = 0.0;
            for sub_y in [-0.25, 0.25].iter() {
                for sub_x in [-0.25, 0.25].iter() {
                    let u = (dx as f32 + sub_x) / size;
                    let v = (dy as f32 + sub_y) / size;
                    if draw_suit_pixel(suit, u, v) {
                        coverage += 0.25;
                    }
                }
            }

            if coverage > 0.0 {
                let mut px = (cx + dx as f32).round() as i32;
                let mut py = (cy + dy as f32).round() as i32;

                if invert {
                    px = width - 1 - px;
                    py = height - 1 - py;
                }

                if px >= 0 && px < width && py >= 0 && py < height {
                    let i = ((py * width + px) * 4) as usize;
                    let pixel_alpha = coverage * alpha;
                    let inv_alpha = 1.0 - pixel_alpha;

                    buffer[i] = (color.0 as f32 * pixel_alpha + buffer[i] as f32 * inv_alpha) as u8;
                    buffer[i + 1] =
                        (color.1 as f32 * pixel_alpha + buffer[i + 1] as f32 * inv_alpha) as u8;
                    buffer[i + 2] =
                        (color.2 as f32 * pixel_alpha + buffer[i + 2] as f32 * inv_alpha) as u8;
                    buffer[i + 3] = (255.0 * pixel_alpha + buffer[i + 3] as f32 * inv_alpha)
                        .min(255.0) as u8;
                }
            }
        }
    }
}

/// 为扑克牌生成基于像素的极简纹理
pub fn generate_poker_texture(card: &Card) -> Image {
    let width = CARD_WIDTH;
    let height = CARD_HEIGHT;

    // Rgba8UnormSrgb format expects 4 bytes per pixel (R, G, B, A)
    let mut data = vec![255; (width * height * 4) as usize]; // Initialize with white (mostly, alpha is also 255)

    let color = get_card_color(card);
    let r = (color.to_srgba().red * 255.0) as u8;
    let g = (color.to_srgba().green * 255.0) as u8;
    let b = (color.to_srgba().blue * 255.0) as u8;

    // 简单绘制一个边框
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;

            // 简单的边框检测
            let is_border = x == 0 || y == 0 || x == width - 1 || y == height - 1;

            if is_border {
                data[i] = 128; // R
                data[i + 1] = 128; // G
                data[i + 2] = 128; // B
                data[i + 3] = 255; // A
            } else {
                // 内部填充白色
                data[i] = 255;
                data[i + 1] = 255;
                data[i + 2] = 255;
                data[i + 3] = 255;

                // 作为一个极其简化的粗略的中心花色标记
                // 我们在中间画一个小方块代替花色
                // if x > width / 3 && x < width * 2 / 3 && y > height / 3 && y < height * 2 / 3 {
                //      data[i] = r;
                //      data[i + 1] = g;
                //      data[i + 2] = b;
                //      data[i + 3] = 50; // 半透明
                // }
            }
        }
    }

    let rank_str = match card {
        Card::NaturalCard(nc) => match nc.rank {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        }
        .to_string(),
        Card::RedJoker => "JOK".to_string(),
        Card::BlackJoker => "JOK".to_string(),
        Card::WildCard => "?".to_string(),
    };

    let font_size = if rank_str.len() > 2 { 14.0 } else { 20.0 };

    // 渲染左上角点数
    render_text_to_buffer(
        &rank_str, font_size, 5, 5, (r, g, b), &mut data, width as i32, height as i32, false,
    );
    // 渲染右下角点数 (倒置)
    render_text_to_buffer(
        &rank_str, font_size, 5, 5, (r, g, b), &mut data, width as i32, height as i32, true,
    );

    if let Card::NaturalCard(nc) = card {
        // 左上角花色
        draw_suit_to_buffer(nc.suit, 12.0, 32.0, 7.0, (r, g, b), 1.0, &mut data, width as i32, height as i32, false);
        // 右下角花色
        draw_suit_to_buffer(nc.suit, 12.0, 32.0, 7.0, (r, g, b), 1.0, &mut data, width as i32, height as i32, true);
        // 中心大花色
        draw_suit_to_buffer(nc.suit, width as f32 / 2.0, height as f32 / 2.0, 20.0, (r, g, b), 0.2, &mut data, width as i32, height as i32, false);
    } else {
        // Joker 的中心星星标记
        let star = "*";
        render_text_to_buffer(
            star, 20.0, 5, 25, (r, g, b), &mut data, width as i32, height as i32, false,
        );
        render_text_to_buffer(
            star, 20.0, 5, 25, (r, g, b), &mut data, width as i32, height as i32, true,
        );
        render_text_to_buffer(
            star, 80.0, 15, 30, (r, g, b), &mut data, width as i32, height as i32, false,
        );
    }

    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// 获取卡牌颜色
fn get_card_color(card: &Card) -> Color {
    match card {
        Card::NaturalCard(nc) => match nc.suit {
            Suit::Hearts | Suit::Diamonds => Color::srgb(0.8, 0.0, 0.0), // 深红
            Suit::Clubs | Suit::Spades => Color::BLACK,
        },
        Card::RedJoker => Color::srgb(0.8, 0.0, 0.0),
        Card::BlackJoker => Color::BLACK,
        Card::WildCard => Color::srgb(0.5, 0.0, 0.5), // 紫色
    }
}


