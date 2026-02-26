use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::data::Data;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Codec(pub Cow<'static, str>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedData {
    pub r#type: DataType,
    pub codec: String,
    pub data: Data,
}

impl std::fmt::Display for TypedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.r#type, self.codec)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    pub id: String,
    pub revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataType {
    pub app: App,
    pub r#type: String,
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}/{}", self.app.id, self.app.revision, self.r#type)
    }
}
