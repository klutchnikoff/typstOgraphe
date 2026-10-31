//! Per-window recovery envelopes. Private input is stored, never published or compiled.
use crate::session::{Checkpoint, Resource, Session};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use typstographe_core::{
    assets::MAX_DOCUMENT_BYTES,
    content::{contract::State, shared::SharedDocument},
};

pub(super) const FORMAT: &str = "typstographe-private-recovery-v1";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub generation: String,
    pub epoch: u64,
    pub sequence: u64,
    pub update: Vec<u8>,
    pub document: Value,
    pub settings: Value,
    pub fields: Vec<Value>,
    pub forms: Vec<Value>,
    pub resources: Vec<Resource>,
    pub rejected: bool,
    #[serde(default)]
    pub recovered: Vec<super::work::Item>,
    #[serde(default)]
    pub retain: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub format: String,
    #[serde(default)]
    pub name: Option<String>,
    pub window: String,
    pub saved_at: u64,
    pub file_path: Option<PathBuf>,
    pub confirmed_revision: u64,
    pub confirmed: Checkpoint,
    pub input: Input,
    pub rejected_edition: Option<Value>,
}

fn directory(root: &Path, generation: &str) -> Result<PathBuf, String> {
    if uuid::Uuid::parse_str(generation)
        .map(|id| id.to_string())
        .as_deref()
        != Ok(generation)
    {
        return Err("recovery_invalid_generation".into());
    }
    let mut path = root.to_path_buf();
    for part in ["drafts", generation] {
        path.push(part);
        if let Ok(meta) = path.symlink_metadata() {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err("recovery_invalid_directory".into());
            }
        } else {
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = builder;
                builder.mode(0o700);
                builder
            };
            builder.create(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(path)
}

pub fn path(root: &Path, generation: &str, window: &str) -> Result<PathBuf, String> {
    if window.is_empty() || window.len() > 128 {
        return Err("recovery_invalid_window".into());
    }
    let name = Sha256::digest(window.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(directory(root, generation)?.join(format!("{name}.json")))
}

pub fn read(path: &Path) -> Result<Record, String> {
    if path
        .symlink_metadata()
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("recovery_invalid_file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("recovery_too_large".into());
    }
    let record: Record = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if record.format != FORMAT {
        return Err("recovery_unknown_format".into());
    }
    Ok(record)
}

pub(super) fn private(record: &Record) -> bool {
    let input = &record.input;
    if input.retain
        || !input.recovered.is_empty()
        || input.rejected
        || !input.fields.is_empty()
        || !input.forms.is_empty()
        || record.rejected_edition.is_some()
    {
        return true;
    }
    let Ok(shared) = SharedDocument::from_update(&input.update) else {
        return true;
    };
    let Ok(document) = serde_json::from_value(input.document.clone()) else {
        return true;
    };
    let Ok(settings) = serde_json::from_value(input.settings.clone()) else {
        return true;
    };
    shared
        .verify(&State::new(
            record.confirmed.document.clone(),
            record.confirmed.settings.clone(),
        ))
        .is_err()
        || shared.verify(&State::new(document, settings)).is_err()
}

pub(super) fn write(path: &Path, record: &Record) -> Result<(), String> {
    let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("recovery_too_large".into());
    }
    let options = atomic_write_file::AtomicWriteFile::options();
    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = options;
        options.mode(0o600);
        options
    };
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.commit().map_err(|e| e.to_string())
}

/// Called only after an explicit successful save/abandon. Unreadable copies are retained.
pub fn clean_confirmed(root: &Path, generation: &str) {
    let Ok(dir) = directory(root, generation) else {
        return;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if let Ok(record) = read(&entry.path())
            && !private(&record)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

impl Session {
    pub fn open_private_recovery(&mut self, window: &str, generation: &str) -> Result<u64, String> {
        self.check_peer(window, generation)?;
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        // A reloaded view cannot silently consume private input that it has not
        // restored. Keep that envelope for the explicit recovery decision.
        let file = path(&self.root, generation, window)?;
        if let Ok(meta) = file.symlink_metadata()
            && (!meta.is_file() || meta.file_type().is_symlink())
        {
            return Err("recovery_invalid_file".into());
        }
        if file.exists()
            && read(&file).map_or(true, |record| record.input.epoch != 0 && private(&record))
        {
            let retained = file.with_file_name(format!("retained-{}.json", uuid::Uuid::new_v4()));
            fs::rename(&file, retained).map_err(|e| e.to_string())?;
        }
        let version = self.draft_epochs.entry(window.into()).or_default();
        version.0 += 1;
        version.1 = 0;
        Ok(version.0)
    }
    pub fn conserve_private(&mut self, window: &str, input: Input) -> Result<bool, String> {
        self.check_peer(window, &input.generation)?;
        let current = self
            .draft_epochs
            .get(window)
            .ok_or("recovery_unregistered_window")?;
        if current.0 != input.epoch {
            return Err("recovery_obsolete_view".into());
        }
        if input.sequence <= current.1 {
            return Ok(false);
        }
        SharedDocument::from_update(&input.update).map_err(crate::owner::error)?;
        if input.forms.len() > 128
            || input.fields.len() > 1024
            || input.resources.len() > 4096
            || input.recovered.len() > 2048
        {
            return Err("recovery_too_large".into());
        }
        let resource_size = input
            .resources
            .iter()
            .try_fold(0usize, |n, r| n.checked_add(r.data.len()));
        if resource_size.is_none_or(|n| n as u64 > MAX_DOCUMENT_BYTES) {
            return Err("recovery_too_large".into());
        }
        let known = self.resources();
        let confirmed_resources: std::collections::HashMap<_, _> = known
            .iter()
            .map(|r| (r.path.as_str(), r.data.as_str()))
            .collect();
        let mut paths = std::collections::HashSet::new();
        for resource in &input.resources {
            if !paths.insert(&resource.path) {
                return Err("recovery_duplicate_resource".into());
            }
            if confirmed_resources.get(resource.path.as_str()).copied()
                != Some(resource.data.as_str())
            {
                resource.decoded()?;
            }
        }
        let owner = self.ensure_owner()?;
        let state = owner.state().clone();
        let update = owner.update_bytes();
        let rejected_edition = owner
            .draft(window)?
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?;
        let record = Record {
            format: FORMAT.into(),
            name: self.recovered_name.clone(),
            window: window.into(),
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs(),
            file_path: self
                .file_lease
                .as_ref()
                .map(|lease| lease.path().to_path_buf()),
            confirmed_revision: self.revision,
            confirmed: Checkpoint {
                schema: "typstographe-web-editor-v1".into(),
                document: state.document,
                settings: state.settings,
                update,
                resources: known,
                source: String::new(),
            },
            input,
            rejected_edition,
        };
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let path = path(&self.root, &record.input.generation, window)?;
        if path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err("recovery_invalid_file".into());
        }
        if self.clean_revision == Some(self.revision) && !private(&record) {
            if path.exists() {
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
            if let Some(store) = &self.recovery {
                store.clear()?;
            }
        } else {
            write(&path, &record)?;
            // Keep the existing confirmed-state recovery usable until the dedicated
            // private-draft recovery interface is implemented. Never delete the envelope.
            if let Some(store) = &self.recovery {
                store.write(
                    record.confirmed.clone(),
                    record
                        .file_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Sans titre".into()),
                )?;
            }
        }
        self.recovered_items
            .insert(window.into(), record.input.recovered.clone());
        self.draft_epochs.get_mut(window).unwrap().1 = record.input.sequence;
        Ok(true)
    }
}

#[tauri::command(async)]
pub fn open_private_recovery(
    window: tauri::WebviewWindow,
    state: tauri::State<crate::Bridge>,
    generation: String,
) -> Result<u64, String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .open_private_recovery(window.label(), &generation)
}
#[tauri::command(async)]
pub fn conserve_private(
    window: tauri::WebviewWindow,
    state: tauri::State<crate::Bridge>,
    input: Input,
) -> Result<bool, String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .conserve_private(window.label(), input)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
