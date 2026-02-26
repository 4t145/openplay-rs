use serde::{Deserialize, Serialize};

use crate::{ban::Ban, room::RoomPlayerPosition, user::UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickOut {
    pub player: UserId,
    pub reason: Option<String>,
    pub ban: Option<Ban>,
}

