use std::borrow::Cow;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::data::Data;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Codec(pub Cow<'static, str>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub codec: Codec,
    pub r#type: String,
    pub data: Data,
}
