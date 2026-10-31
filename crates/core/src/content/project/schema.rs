use super::{Error, invalid_index};
use crate::document_types::DocumentType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const FORMAT_VERSION: u32 = 11;
pub const MAX_SNAPSHOTS: usize = 100;
pub const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_OBJECTS: usize = 16_384;
pub(super) const MAX_INDEX_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

/// An address of the original uncompressed bytes, never a filesystem path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ObjectId(String);
impl ObjectId {
    pub(super) fn of(bytes: &[u8]) -> Self {
        let hex: String = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Self(format!("sha256:{hex}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub(super) fn path(&self) -> String {
        format!("objects/sha256/{}", &self.0[7..])
    }
    pub(super) fn from_path(path: &str) -> Option<Self> {
        let hex = path.strip_prefix("objects/sha256/")?;
        Self::parse(&format!("sha256:{hex}"))
    }
    fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix("sha256:")?;
        (hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f')))
        .then(|| Self(value.into()))
    }
}
impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?)
            .ok_or_else(|| serde::de::Error::custom("invalid SHA-256 reference"))
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub format: String,
    pub version: u32,
    pub project_id: Uuid,
    pub current: ObjectId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotReason {
    Manual,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotDetails {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub creator: Option<String>,
}
impl SnapshotDetails {
    fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name.chars().count() <= 160
            && !self.name.chars().any(char::is_control)
            && self.tags.len() <= 32
            && self.tags.iter().all(|t| {
                !t.trim().is_empty() && t.chars().count() <= 64 && !t.chars().any(char::is_control)
            })
            && self.tags.iter().collect::<BTreeSet<_>>().len() == self.tags.len()
            && self.description.as_ref().is_none_or(|s| s.len() <= 16_384)
            && self
                .creator
                .as_ref()
                .is_none_or(|s| s.len() <= 512 && !s.chars().any(char::is_control))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRecord {
    pub id: Uuid,
    pub created_at: String,
    pub details: SnapshotDetails,
    pub reason: SnapshotReason,
    pub state: ObjectId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewReference {
    pub id: Uuid,
    pub review_id: Uuid,
    pub created_at: String,
    pub state: ObjectId,
    pub active: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VersionIndex {
    pub version: u32,
    pub snapshots: Vec<SnapshotRecord>,
    pub review_references: Vec<ReviewReference>,
}
impl VersionIndex {
    pub fn empty() -> Self {
        Self {
            version: 1,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        if self.version != 1 {
            return Err(crate::errors::StorageError::Unsupported.into());
        }
        if self.snapshots.len() > MAX_SNAPSHOTS {
            return Err(crate::errors::StorageError::SnapshotLimit {
                limit: MAX_SNAPSHOTS as u32,
            }
            .into());
        }
        let mut ids = BTreeSet::new();
        let mut reviews = BTreeSet::new();
        for record in &self.snapshots {
            if record.id.is_nil()
                || !ids.insert(record.id)
                || !utc(&record.created_at)
                || !record.details.valid()
            {
                return Err(invalid_index());
            }
        }
        for record in &self.review_references {
            if record.id.is_nil()
                || record.review_id.is_nil()
                || !ids.insert(record.id)
                || !reviews.insert(record.review_id)
                || !utc(&record.created_at)
            {
                return Err(invalid_index());
            }
        }
        if serde_json::to_vec(self)
            .map_err(crate::errors::CoreError::external)?
            .len() as u64
            > MAX_INDEX_BYTES
        {
            return Err(crate::errors::StorageError::EntryTooLarge {
                name: "versions.json".into(),
            }
            .into());
        }
        Ok(())
    }
    pub fn roots(&self) -> impl Iterator<Item = &ObjectId> {
        self.snapshots
            .iter()
            .map(|s| &s.state)
            .chain(self.review_references.iter().map(|r| &r.state))
    }
}
fn utc(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok_and(|d| d.offset().local_minus_utc() == 0)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StateDescriptor {
    pub version: u32,
    pub contract_version: u32,
    pub document_type: DocumentType,
    pub document: ObjectId,
    pub metadata: ObjectId,
    pub yjs: Checkpoint,
    pub model: Model,
    pub sources: Sources,
    pub resources: ObjectId,
    pub producer: Producer,
}
impl StateDescriptor {
    pub fn validate(&self) -> Result<(), Error> {
        if self.version != 1
            || self.contract_version != crate::content::contract::VERSION
            || self.yjs.encoding != "yjs-v1"
        {
            return Err(crate::errors::StorageError::Unsupported.into());
        }
        Ok(())
    }
    pub fn refs(&self) -> [&ObjectId; 7] {
        [
            &self.document,
            &self.metadata,
            &self.yjs.object,
            &self.model.source,
            &self.sources.main,
            &self.sources.body,
            &self.resources,
        ]
    }
}
macro_rules! record { ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct $name { $(pub $field: $ty),* }
}; }
record!(Checkpoint {
    encoding: String,
    object: ObjectId
});
record!(Model {
    id: String,
    source: ObjectId
});
record!(Sources {
    main: ObjectId,
    body: ObjectId
});
record!(Producer {
    application: String,
    generator: u32,
    typst: String
});
pub(super) type ResourceMap = BTreeMap<String, ObjectId>;

// A plain BTreeMap deserializer silently overwrites repeated logical paths.
#[derive(Deserialize)]
#[serde(transparent)]
pub(super) struct ResourceManifest(#[serde(deserialize_with = "unique_resources")] pub ResourceMap);
fn unique_resources<'de, D: serde::Deserializer<'de>>(d: D) -> Result<ResourceMap, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = ResourceMap;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("unique logical resource paths")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<ResourceMap, M::Error> {
            let mut resources = ResourceMap::new();
            while let Some((path, id)) = map.next_entry::<String, ObjectId>()? {
                if resources.len() >= MAX_OBJECTS || resources.insert(path, id).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate resource path or resource count limit",
                    ));
                }
            }
            Ok(resources)
        }
    }
    d.deserialize_map(Visitor)
}
