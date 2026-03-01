use base64::prelude::*;
use bytes::Bytes;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Data(pub Bytes);

impl<T> From<T> for Data
where
    T: Into<Bytes>,
{
    fn from(value: T) -> Self {
        Data(value.into())
    }
}

impl serde::Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            // For human-readable formats, encode as base64 string
            let encoded = BASE64_STANDARD.encode(&self.0);
            serializer.serialize_str(&encoded)
        } else {
            // For binary formats, serialize as bytes
            serializer.serialize_bytes(&self.0)
        }
    }
}   

impl<'de> serde::Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            // For human-readable formats, expect a base64 string
            let s = String::deserialize(deserializer)?;
            let decoded = BASE64_STANDARD
                .decode(s.as_bytes())
                .map_err(serde::de::Error::custom)?;
            Ok(Data(Bytes::from(decoded)))
        } else {
            // For binary formats, deserialize as bytes
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            Ok(Data(Bytes::from(bytes)))
        }
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

