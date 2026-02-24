use std::ops::{Deref, DerefMut};

use bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Data(pub Bytes);

impl serde::Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <&[u8]>::deserialize(deserializer)?;
        Ok(Data(Bytes::copy_from_slice(bytes)))
    }
}

impl Deref for Data {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Data {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Bytes> for Data {
    fn from(bytes: Bytes) -> Self {
        Data(bytes)
    }
}
