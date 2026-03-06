use bevy::prelude::*;
use openplay_basic::user::User;

#[derive(Resource)]
pub struct CurrentUserInfo {
    pub user: Option<User>,
}