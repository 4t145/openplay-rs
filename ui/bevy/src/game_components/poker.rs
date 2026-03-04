use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    window::PrimaryWindow,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use openplay_poker::{Card, Rank, Suit};
use std::sync::OnceLock;

use crate::global_config::theme_manager::{PokerThemeId, PokerThemeStore, PokerThemeRegistry};

#[derive(Debug, Component)]
pub struct PokerCard {
    pub card: openplay_poker::Card,
    pub face_up: bool,
}

#[derive(Component, Default)]
pub struct CardTilt {
    pub target_rotation: Quat,
}

#[derive(Bundle)]
pub struct PokerCardBundle {
    pub tilt: CardTilt,
    pub card: PokerCard,
    pub theme: PokerThemeId,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}
const CARD_SCALE: f32 = 1.2; 
const VISUAL_ASPECT_RATIO: f32 = 0.714; // 63.5mm / 88.9mm
impl PokerCardBundle {
    pub fn new(card: PokerCard, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        // 创建独立网格以支持自定义UV
        // 使用 CARD_SCALE 缩放以匹配视觉大小
        let width = 1.0 * CARD_SCALE;
        let height = 1.0 * CARD_SCALE;
        let mesh = meshes.add(Rectangle::new(width, height));
        
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: false, 
            perceptual_roughness: 0.8,
            reflectance: 0.2,
            ..default()
        });
        
        Self {
            tilt: CardTilt::default(),
            card,
            theme: PokerThemeId::default(),
            mesh: Mesh3d(mesh),
            material: MeshMaterial3d(material),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}

pub struct PokerPlugin;

impl Plugin for PokerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_poker_card_texture,
            interact_poker_card,
            animate_poker_card_tilt,
        ));
    }
}

pub fn update_poker_card_texture(
    q_cards: Query<
        (
            Entity,
            &PokerCard,
            &PokerThemeId,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Or<(Changed<PokerCard>, Changed<PokerThemeId>)>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    theme_registry: Res<PokerThemeRegistry>,
    texture_atlases: Res<Assets<TextureAtlasLayout>>,
) {
    for (_entity, card, theme_id, mesh_handle, mat_handle) in q_cards.iter() {
        if let Some(theme) = theme_registry.get(theme_id) {
            // 获取主题生成的Sprite信息
            let sprite = theme.poker_to_sprite(card);
            
            // 1. 更新材质贴图
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                // Bevy 0.14 Sprite 使用 image 字段
                mat.base_color_texture = Some(theme.texture.clone());
                mat.alpha_mode = AlphaMode::Blend;
                mat.unlit = false; // 接受光照
                mat.perceptual_roughness = 0.8; // 纸张质感
                mat.reflectance = 0.2;
            }

            // 2. 根据图集更新网格UV
            if let Some(atlas) = &sprite.texture_atlas {
                 if let Some(layout) = texture_atlases.get(&atlas.layout) {
                     if atlas.index < layout.textures.len() {
                        let rect = layout.textures[atlas.index];
                        let size = layout.size.as_vec2();
                        
                        // 计算UV (归一化 0..1)
                        // Bevy纹理原点为左上角
                        let min = rect.min.as_vec2() / size;
                        let max = rect.max.as_vec2() / size;

                        // Bevy Rectangle Mesh 顶点顺序通常为:
                        // Top-Left, Top-Right, Bottom-Right, Bottom-Left
                        // 如果发现贴图左右镜像了，需要交换左右顶点的 U 坐标 (min.x <-> max.x)
                        let uvs = vec![
                            [max.x, min.y], // Top-Left 
                            [min.x, min.y], // Top-Right 
                            [min.x, max.y], // Bottom-Right 
                            [max.x, max.y], // Bottom-Left 
                        ];

                        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                             mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                        }
                     }
                }
            }
        }
    }
}

pub fn interact_poker_card(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut card_query: Query<(&GlobalTransform, &mut CardTilt), With<PokerCard>>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    let Ok(window) = window_query.single() else { return };

    if let Some(cursor_position) = window.cursor_position() {
        // 计算从相机到鼠标的射线
        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
             for (card_transform, mut tilt) in card_query.iter_mut() {
                // 射线-平面求交
                // 平面法线为 +Z (0,0,1) 在局部空间，但卡片会旋转
                // 简化：假设卡片主要朝上
                let plane_normal = card_transform.up().any_orthonormal_vector();
                let plane_origin = card_transform.translation();

                let denom = ray.direction.dot(plane_normal);
                if denom.abs() > 1e-6 {
                    let t = (plane_origin - ray.origin).dot(plane_normal) / denom;
                    if t >= 0.0 {
                        let intersection_point = ray.origin + ray.direction * t;
                        // 将交点转换到卡片局部空间
                        let local_point = card_transform.affine().inverse().transform_point3(intersection_point);
                        
                        // 高度是完整的 (Mesh是正方形，假设像素画占满了垂直方向)
                        let hit_height = 1.0 * CARD_SCALE;
                        // 宽度只取中间的 71.4%
                        let hit_width = hit_height * VISUAL_ASPECT_RATIO;
                        
                        let half_width = hit_width / 2.0; 
                        let half_height = hit_height / 2.0;

                        if local_point.x.abs() < half_width && local_point.y.abs() < half_height {
                             // 应用倾斜效果 (类似 Balatro/小丑牌)
                             let max_angle = 0.5; // 最大倾斜角度 (弧度)
                             
                             // 计算基于偏移的倾斜
                             // 按下上方 -> 绕X轴负旋转
                             // 按下右方 -> 绕Y轴正旋转
                             let rot_x = -(local_point.y / half_height) * max_angle;
                             let rot_y = (local_point.x / half_width) * max_angle;
                             
                             tilt.target_rotation = Quat::from_euler(EulerRot::XYZ, rot_x, rot_y, 0.0);
                        } else {
                             tilt.target_rotation = Quat::IDENTITY;
                        }
                    }
                }
             }
        }
    } else {
        // 无鼠标时复位
        for (_, mut tilt) in card_query.iter_mut() {
            tilt.target_rotation = Quat::IDENTITY;
        }
    }
}

pub fn animate_poker_card_tilt(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &CardTilt), With<PokerCard>>,
) {
    for (mut transform, tilt) in query.iter_mut() {
        // 平滑插值动画
        transform.rotation = transform.rotation.slerp(tilt.target_rotation, time.delta_secs() * 15.0);
    }
}