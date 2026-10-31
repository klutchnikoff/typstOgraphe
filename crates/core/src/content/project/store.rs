//! The source file stays open across atomic replacements; old roots remain readable.
use super::{Error, schema::*};
use crate::{
    assets::MAX_DOCUMENT_BYTES,
    errors::{CoreError, StorageError},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
};
use zip::ZipArchive;

type Source = Arc<Mutex<ZipArchive<File>>>;
#[derive(Clone, Default)]
pub(super) struct Store {
    source: Option<Source>,
    pending: BTreeMap<ObjectId, Arc<[u8]>>,
    sizes: BTreeMap<ObjectId, u64>,
}
pub(super) fn json<T: DeserializeOwned>(bytes: &[u8], name: &str) -> Result<T, Error> {
    serde_json::from_slice(bytes).map_err(|e| {
        StorageError::EntryUnreadable {
            name: name.into(),
            error: e.to_string(),
        }
        .into()
    })
}
pub(super) fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(value).map_err(|e| CoreError::external(e).into())
}
fn read_entry(zip: &mut ZipArchive<File>, name: &str, limit: u64) -> Result<Vec<u8>, Error> {
    let entry = zip
        .by_name(name)
        .map_err(|e| StorageError::EntryUnreadable {
            name: name.into(),
            error: e.to_string(),
        })?;
    let size = entry.size();
    if size > limit {
        return Err(StorageError::EntryTooLarge { name: name.into() }.into());
    }
    let mut bytes = Vec::new();
    entry
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| StorageError::EntryUnreadable {
            name: name.into(),
            error: e.to_string(),
        })?;
    if bytes.len() as u64 != size || bytes.len() as u64 > limit {
        return Err(StorageError::EntryTooLarge { name: name.into() }.into());
    }
    Ok(bytes)
}
impl Store {
    pub fn open(path: &Path) -> Result<(Self, Manifest, VersionIndex), Error> {
        let file = File::open(path).map_err(|e| StorageError::OpenFailed {
            error: e.to_string(),
        })?;
        if file.metadata().map_err(CoreError::external)?.len() > MAX_ARCHIVE_BYTES {
            return Err(StorageError::FileTooLarge.into());
        }
        let mut zip = ZipArchive::new(file).map_err(|e| StorageError::ZipUnreadable {
            error: e.to_string(),
        })?;
        if zip.len() > MAX_OBJECTS + 2 {
            return Err(StorageError::FileCount.into());
        }
        // `len` and `by_index` enumerate unique names; ZIP preserves the raw count separately.
        if zip.has_duplicate_names() {
            return Err(StorageError::DuplicateEntry {
                name: "ZIP central directory".into(),
            }
            .into());
        }
        let manifest_bytes = read_entry(&mut zip, "manifest.json", MAX_MANIFEST_BYTES)?;
        #[derive(serde::Deserialize)]
        struct Header {
            format: String,
            version: u32,
        }
        let header: Header = json(&manifest_bytes, "manifest.json")?;
        if header.format != "typstOgraphe" || header.version != FORMAT_VERSION {
            return Err(StorageError::Unsupported.into());
        }
        let mut names = BTreeSet::new();
        let mut sizes = BTreeMap::new();
        let mut total = 0u64;
        for index in 0..zip.len() {
            let entry = zip.by_index(index).map_err(CoreError::external)?;
            let name = entry.name().map_err(CoreError::external)?.into_owned();
            if !names.insert(name.clone()) {
                return Err(StorageError::DuplicateEntry { name }.into());
            }
            if entry.is_dir()
                || entry
                    .unix_mode()
                    .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(StorageError::UnexpectedEntry { name }.into());
            }
            let limit = match name.as_str() {
                "manifest.json" => MAX_MANIFEST_BYTES,
                "versions.json" => MAX_INDEX_BYTES,
                _ => {
                    let id = ObjectId::from_path(&name)
                        .ok_or_else(|| StorageError::UnexpectedEntry { name: name.clone() })?;
                    sizes.insert(id, entry.size());
                    MAX_DOCUMENT_BYTES
                }
            };
            if entry.size() > limit {
                return Err(StorageError::EntryTooLarge { name }.into());
            }
            total = total
                .checked_add(entry.size())
                .ok_or(StorageError::ExpandedTooLarge)?;
            if total > MAX_ARCHIVE_BYTES {
                return Err(StorageError::ExpandedTooLarge.into());
            }
        }
        if sizes.len() > MAX_OBJECTS {
            return Err(StorageError::FileCount.into());
        }
        let manifest: Manifest = json(&manifest_bytes, "manifest.json")?;
        if manifest.project_id.is_nil() {
            return Err(super::invalid_index());
        }
        let versions: VersionIndex = json(
            &read_entry(&mut zip, "versions.json", MAX_INDEX_BYTES)?,
            "versions.json",
        )?;
        versions.validate()?;
        Ok((
            Self {
                source: Some(Arc::new(Mutex::new(zip))),
                sizes,
                pending: BTreeMap::new(),
            },
            manifest,
            versions,
        ))
    }
    pub fn insert(&mut self, bytes: impl Into<Arc<[u8]>>) -> Result<ObjectId, Error> {
        let bytes = bytes.into();
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(StorageError::LetterTooLarge.into());
        }
        let id = ObjectId::of(&bytes);
        // Keep supplied confirmed bytes even if the original file has since been damaged.
        self.pending.entry(id.clone()).or_insert(bytes);
        Ok(id)
    }
    pub fn insert_json<T: Serialize>(&mut self, value: &T) -> Result<ObjectId, Error> {
        self.insert(serialize(value)?)
    }
    pub fn size(&self, id: &ObjectId) -> Result<u64, Error> {
        self.pending
            .get(id)
            .map(|b| b.len() as u64)
            .or_else(|| self.sizes.get(id).copied())
            .ok_or_else(|| {
                StorageError::MissingEntry {
                    name: id.as_str().into(),
                }
                .into()
            })
    }
    pub fn read(&self, id: &ObjectId) -> Result<Arc<[u8]>, Error> {
        if let Some(bytes) = self.pending.get(id) {
            return Ok(bytes.clone());
        }
        let source = self
            .source
            .as_ref()
            .ok_or_else(|| StorageError::MissingEntry {
                name: id.as_str().into(),
            })?;
        let mut zip = source.lock().map_err(CoreError::external)?;
        let bytes = read_entry(&mut zip, &id.path(), MAX_DOCUMENT_BYTES)?;
        if ObjectId::of(&bytes) != *id {
            return Err(StorageError::ObjectIntegrity {
                name: id.as_str().into(),
            }
            .into());
        }
        Ok(bytes.into())
    }
    pub fn read_json<T: DeserializeOwned>(&self, id: &ObjectId) -> Result<T, Error> {
        json(&self.read(id)?, id.as_str())
    }
}
