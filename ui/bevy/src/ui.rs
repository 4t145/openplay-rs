use bevy::prelude::*;

pub mod button;
pub mod font;
pub mod user_manage;
pub mod room_ui;


#[derive(Debug, bevy::prelude::Component)]
pub struct Ui;

pub mod general_ui;

pub struct UiPlugin;

pub fn setup_hud(mut commands: Commands) {
    
}

pub fn cleanup_hud(mut commands: Commands, query: Query<Entity, With<Ui>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// press F1 to toggle hud visibility
pub fn toggle_hud_visibility(
    mut query: Query<&mut Visibility, With<Ui>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::F1) {
        info!("Toggling HUD visibility");
        for mut visibility in query.iter_mut() {
            *visibility = match *visibility {
                Visibility::Visible => Visibility::Hidden,
                Visibility::Hidden => Visibility::Visible,
                Visibility::Inherited => Visibility::Inherited,
            }
        }
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup,  setup_hud);
        app.add_systems(Update, toggle_hud_visibility);
        app.add_plugins((
            general_ui::GeneralUiPlugin,
            button::UiButtonPlugin,
            room_ui::RoomUiPlugin,
        ));
    }
}
