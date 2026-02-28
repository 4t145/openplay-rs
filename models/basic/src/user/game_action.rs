use serde::{Deserialize, Serialize};

use crate::message::TypedData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameActionData {
    pub message: TypedData,
    pub ref_version: u32,
}
