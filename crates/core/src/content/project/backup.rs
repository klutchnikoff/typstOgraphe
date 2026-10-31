//! Portable recovery context: a compressed format-11 archive, cached and shared in memory.
use super::{Error, MAX_ARCHIVE_BYTES, Project};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct Backup(Arc<str>);
impl Backup {
    pub fn capture(project: &Project) -> Result<Self, Error> {
        Ok(Self(STANDARD.encode(project.to_bytes()?).into()))
    }
    pub fn restore(&self) -> Result<Project, Error> {
        let bytes = STANDARD
            .decode(self.0.as_bytes())
            .map_err(crate::errors::CoreError::external)?;
        Project::from_bytes(bytes)
    }
}
impl std::fmt::Debug for Backup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backup")
            .field("encoded_bytes", &self.0.len())
            .finish()
    }
}
impl Serialize for Backup {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for Backup {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let encoded = String::deserialize(d)?;
        if encoded.len() as u64 > MAX_ARCHIVE_BYTES.div_ceil(3) * 4 {
            return Err(serde::de::Error::custom(
                "project backup exceeds archive budget",
            ));
        }
        Ok(Self(encoded.into()))
    }
}
