//! Autonomous format-11 CAS owner. Adapters must preserve this owner across saves.
//! Preparing a candidate is pure: publish it only after `save` succeeds.
mod backup;
mod schema;
mod state;
mod store;
use super::archive::Archive;
pub use super::archive::Error;
use crate::{
    DocumentLease,
    errors::{CoreError, StorageError},
};
pub use backup::Backup;
pub use schema::{
    FORMAT_VERSION, MAX_ARCHIVE_BYTES, MAX_OBJECTS, MAX_SNAPSHOTS, ObjectId, ReviewReference,
    SnapshotDetails, SnapshotReason, SnapshotRecord,
};
use schema::{Manifest, VersionIndex};
use std::{
    io::{Cursor, Seek, Write},
    path::Path,
    sync::Arc,
};
use store::{Store, serialize};
use uuid::Uuid;

fn invalid_index() -> Error {
    StorageError::ProjectIndexInvalid.into()
}
fn unknown(id: Uuid) -> Error {
    StorageError::UnknownSnapshot { id: id.to_string() }.into()
}

#[derive(Clone)]
pub struct Project {
    project_id: Uuid,
    current: Archive,
    current_id: ObjectId,
    index: VersionIndex,
    store: Store,
}
impl Project {
    pub fn new(current: Archive) -> Result<Self, Error> {
        let current = state::prepare(current, true)?;
        let mut store = Store::default();
        let current_id = state::encode(&mut store, &current)?;
        Ok(Self {
            project_id: Uuid::new_v4(),
            current,
            current_id,
            index: VersionIndex::empty(),
            store,
        })
    }
    pub fn project_id(&self) -> Uuid {
        self.project_id
    }
    pub fn current(&self) -> &Archive {
        &self.current
    }
    /// Preserve owned dependencies on ordinary edits; resolve the catalogue only for a new model.
    pub fn dependencies_for(&self, template: &str) -> crate::packages::Dependencies {
        if template == self.current.template {
            return self.current.dependencies.clone();
        }
        let provided = crate::document_types::DocumentType::ALL
            .into_iter()
            .any(|kind| kind.models().any(|model| model.source == template));
        if !provided
            && crate::packages::validate_embedded(template, &self.current.dependencies).is_ok()
        {
            self.current.dependencies.clone()
        } else {
            crate::packages::for_template(template)
        }
    }
    pub fn versions(&self) -> &[SnapshotRecord] {
        &self.index.snapshots
    }
    pub fn review_references(&self) -> &[ReviewReference] {
        &self.index.review_references
    }
    /// Save As creates an independent project without renumbering its versions.
    pub fn independent_copy(&self) -> Self {
        Self {
            project_id: Uuid::new_v4(),
            ..self.clone()
        }
    }
    pub fn with_current(&self, current: Archive) -> Result<Self, Error> {
        let mut candidate = self.clone();
        candidate.current = state::prepare(current, false)?;
        candidate.current_id = state::encode(&mut candidate.store, &candidate.current)?;
        Ok(candidate)
    }
    pub fn version(&self, id: Uuid) -> Result<Archive, Error> {
        let record = self
            .index
            .snapshots
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| unknown(id))?;
        state::decode(&self.store, &record.state)
    }
    pub fn review_reference(&self, review_id: Uuid) -> Result<Archive, Error> {
        let record = self
            .index
            .review_references
            .iter()
            .find(|r| r.review_id == review_id)
            .ok_or_else(|| unknown(review_id))?;
        state::decode(&self.store, &record.state)
    }
    /// Candidate only; the caller must save before publishing the new version.
    pub fn prepare_version(
        &self,
        mut details: SnapshotDetails,
        reason: SnapshotReason,
    ) -> Result<Self, Error> {
        if self.index.snapshots.len() >= MAX_SNAPSHOTS {
            return Err(StorageError::SnapshotLimit {
                limit: MAX_SNAPSHOTS as u32,
            }
            .into());
        }
        let now = chrono::Utc::now();
        if details.name.trim().is_empty() {
            details.name = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        }
        let mut candidate = self.clone();
        candidate.index.snapshots.push(SnapshotRecord {
            id: Uuid::new_v4(),
            created_at: now.to_rfc3339(),
            details,
            reason,
            state: self.current_id.clone(),
        });
        candidate.index.validate()?;
        Ok(candidate)
    }
    pub fn prepare_remove_version(&self, id: Uuid) -> Result<Self, Error> {
        let mut candidate = self.clone();
        let len = candidate.index.snapshots.len();
        candidate.index.snapshots.retain(|s| s.id != id);
        if candidate.index.snapshots.len() == len {
            return Err(unknown(id));
        }
        Ok(candidate)
    }
    /// Only labels change; content, identity, origin and creation time are immutable.
    pub fn prepare_version_details(
        &self,
        id: Uuid,
        details: SnapshotDetails,
    ) -> Result<Self, Error> {
        let mut candidate = self.clone();
        let record = candidate
            .index
            .snapshots
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| unknown(id))?;
        record.details = details;
        candidate.index.validate()?;
        Ok(candidate)
    }
    /// Starting a review captures once; resuming keeps its original reference.
    /// Review lifecycle and release belong to the review manager, not Versions.
    pub fn prepare_review_reference(&self, review_id: Uuid) -> Result<Self, Error> {
        if review_id.is_nil() {
            return Err(invalid_index());
        }
        if self
            .index
            .review_references
            .iter()
            .any(|r| r.review_id == review_id)
        {
            return Ok(self.clone());
        }
        let mut candidate = self.clone();
        candidate.index.review_references.push(ReviewReference {
            id: Uuid::new_v4(),
            review_id,
            created_at: chrono::Utc::now().to_rfc3339(),
            state: self.current_id.clone(),
            active: true,
        });
        candidate.index.validate()?;
        Ok(candidate)
    }
    pub fn read(path: &Path) -> Result<Self, Error> {
        let (store, manifest, index) = Store::open(path)?;
        Self::from_store(store, manifest, index)
    }
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Result<Self, Error> {
        let (store, manifest, index) = Store::from_bytes(bytes.into())?;
        Self::from_store(store, manifest, index)
    }
    fn from_store(store: Store, manifest: Manifest, index: VersionIndex) -> Result<Self, Error> {
        let current = state::decode(&store, &manifest.current)?;
        Ok(Self {
            project_id: manifest.project_id,
            current,
            current_id: manifest.current,
            store,
            index,
        })
    }
    /// Single publication of current state, index and retained objects, under the lease.
    /// Any error keeps the original file and the lease fingerprint unchanged.
    pub fn save(&self, lease: &mut DocumentLease) -> Result<(), Error> {
        lease.check()?;
        let mut options = atomic_write_file::AtomicWriteFile::options();
        options.read(true);
        #[cfg(unix)]
        let options = {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = options;
            options.mode(0o600);
            options
        };
        let mut temporary = options.open(lease.path()).map_err(CoreError::external)?;
        self.write_archive(&mut temporary)?;
        if temporary
            .as_file_mut()
            .metadata()
            .map_err(CoreError::external)?
            .len()
            > MAX_ARCHIVE_BYTES
        {
            return Err(StorageError::FileTooLarge.into());
        }
        lease.check()?;
        let fingerprint = lease.prepare_fingerprint(temporary.as_file_mut())?;
        temporary.commit().map_err(CoreError::external)?;
        lease.record_fingerprint(fingerprint);
        Ok(())
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut output = Cursor::new(Vec::new());
        self.write_archive(&mut output)?;
        if output.get_ref().len() as u64 > MAX_ARCHIVE_BYTES {
            return Err(StorageError::FileTooLarge.into());
        }
        Ok(output.into_inner())
    }
    fn write_archive(&self, output: &mut (impl Write + Seek)) -> Result<(), Error> {
        self.index.validate()?;
        let reachable = state::reachable(
            &self.store,
            std::iter::once(&self.current_id).chain(self.index.roots()),
        )?;
        let manifest = serialize(&Manifest {
            format: "typstOgraphe".into(),
            version: FORMAT_VERSION,
            project_id: self.project_id,
            current: self.current_id.clone(),
        })?;
        let versions = serialize(&self.index)?;
        let total =
            reachable
                .iter()
                .try_fold((manifest.len() + versions.len()) as u64, |total, id| {
                    total
                        .checked_add(self.store.size(id)?)
                        .ok_or_else(|| Error::from(StorageError::ExpandedTooLarge))
                })?;
        if total > MAX_ARCHIVE_BYTES {
            return Err(StorageError::ExpandedTooLarge.into());
        }
        {
            let mut zip = zip::ZipWriter::new(output);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (path, bytes) in [("manifest.json", manifest), ("versions.json", versions)] {
                zip.start_file(path, options).map_err(CoreError::external)?;
                zip.write_all(&bytes).map_err(CoreError::external)?;
            }
            for id in reachable {
                let bytes = self.store.read(&id)?;
                zip.start_file(id.path(), options)
                    .map_err(CoreError::external)?;
                zip.write_all(&bytes).map_err(CoreError::external)?;
            }
            zip.finish().map_err(CoreError::external)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod backup_tests;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod coherence_tests;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod safety_tests;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
