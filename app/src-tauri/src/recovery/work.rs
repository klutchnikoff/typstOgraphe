//! Recoverable work is copied durably before publication. Never publish private input.
use super::drafts;
use crate::session::{Checkpoint, Session};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use typstographe_core::{
    DocumentLease,
    assets::MAX_DOCUMENT_BYTES,
    content::{contract::State, shared::SharedDocument},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub payload: Value,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub document_type: String,
    pub document_type_label: String,
    pub saved_at: u64,
    pub window: String,
    pub original_path: Option<PathBuf>,
    pub pending: usize,
    pub error: Option<String>,
}
struct Claim {
    directory: PathBuf,
    _lease: Option<DocumentLease>,
}
fn uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
fn file_name(value: &str) -> bool {
    value.strip_suffix(".json").is_some_and(|stem| {
        (stem.len() == 64
            && stem
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()))
            || stem.strip_prefix("retained-").is_some_and(uuid)
    })
}
fn plain_dir(path: &Path) -> Result<(), String> {
    let meta = path.symlink_metadata().map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("recovery_invalid_directory".into());
    }
    Ok(())
}
impl Session {
    pub fn recovery_checkpoint(&mut self) -> Result<Checkpoint, String> {
        let resources = self.resources();
        let owner = self.ensure_owner()?;
        Ok(Checkpoint {
            schema: "typstographe-web-editor-v1".into(),
            source: String::new(),
            document: owner.state().document.clone(),
            settings: owner.state().settings.clone(),
            update: owner.update_bytes(),
            resources,
        })
    }
    fn claim_work(&self, id: &str) -> Result<(Claim, PathBuf), String> {
        let parts: Vec<_> = id.split('/').collect();
        if parts.len() != 3 {
            return Err("recovery_invalid_reference".into());
        }
        let own = self
            .root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("current");
        let is_own = parts[0] == "current" || parts[0] == own;
        let directory = if is_own {
            self.root.clone()
        } else {
            if !uuid(parts[0]) {
                return Err("recovery_invalid_reference".into());
            }
            let store = self.recovery.as_ref().ok_or("recovery_invalid_reference")?;
            plain_dir(&store.base.join("sessions"))?;
            store.base.join("sessions").join(parts[0])
        };
        plain_dir(&directory)?;
        let lease = if is_own {
            None
        } else {
            Some(
                DocumentLease::acquire(&directory.join("owner"))
                    .map_err(|e| typstographe_i18n::command_error(&e))?,
            )
        };
        let path = if parts[1] == "confirmed" && parts[2] == "session" {
            if is_own && self.opened {
                return Err("recovery_live_copy".into());
            }
            directory.join("session.json")
        } else {
            if !uuid(parts[1]) || !file_name(parts[2]) {
                return Err("recovery_invalid_reference".into());
            }
            plain_dir(&directory.join("drafts"))?;
            let parent = directory.join("drafts").join(parts[1]);
            plain_dir(&parent)?;
            // The live current view is never offered as a second recovery project.
            if is_own && parts[1] == self.generation && !parts[2].starts_with("retained-") {
                return Err("recovery_live_copy".into());
            }
            parent.join(parts[2])
        };
        Ok((
            Claim {
                directory,
                _lease: lease,
            },
            path,
        ))
    }
    pub fn work_list(&self) -> Result<Vec<Entry>, String> {
        let mut roots = vec![("current".to_owned(), self.root.clone())];
        if let Some(store) = &self.recovery
            && let Ok(dirs) = fs::read_dir(store.base.join("sessions"))
        {
            for entry in dirs.flatten().take(4096) {
                if entry.path() != self.root
                    && entry.file_type().is_ok_and(|m| m.is_dir())
                    && let Some(name) = entry.file_name().to_str().filter(|s| uuid(s))
                {
                    roots.push((name.into(), entry.path()));
                }
            }
        }
        let mut entries = vec![];
        for (session, root) in roots {
            let mut ids = vec![];
            if let Ok(generations) = fs::read_dir(root.join("drafts")) {
                for generation in generations.flatten().take(4096) {
                    if !generation.file_type().is_ok_and(|m| m.is_dir()) {
                        continue;
                    }
                    let Some(generation_id) = generation
                        .file_name()
                        .to_str()
                        .filter(|s| uuid(s))
                        .map(str::to_owned)
                    else {
                        continue;
                    };
                    if let Ok(files) = fs::read_dir(generation.path()) {
                        for file in files.flatten().take(4096) {
                            if let Some(name) = file.file_name().to_str().filter(|s| file_name(s)) {
                                ids.push(format!("{session}/{generation_id}/{name}"));
                            }
                        }
                    }
                }
            }
            // Legacy confirmed copies remain usable, without duplicating a richer envelope.
            if ids.is_empty()
                && root.join("session.json").is_file()
                && !(session == "current" && self.opened)
            {
                ids.push(format!("{session}/confirmed/session"));
            }
            for id in ids {
                if entries.len() >= 4096 {
                    break;
                }
                let Ok((_claim, path)) = self.claim_work(&id) else {
                    continue;
                };
                match read_work(&path) {
                    Ok(record) => entries.push(Entry {
                        id,
                        name: name(&record),
                        document_type: record
                            .confirmed
                            .settings
                            .metadata
                            .document_type()
                            .file_stem()
                            .into(),
                        document_type_label: typstographe_i18n::text(
                            record
                                .confirmed
                                .settings
                                .metadata
                                .document_type()
                                .label_key(),
                        ),
                        saved_at: record.saved_at,
                        window: record.window.clone(),
                        original_path: record.file_path.clone(),
                        pending: items(&record).len(),
                        error: None,
                    }),
                    Err(error) => entries.push(Entry {
                        id,
                        name: typstographe_i18n::text("recovery-unreadable"),
                        document_type: String::new(),
                        document_type_label: String::new(),
                        saved_at: 0,
                        window: String::new(),
                        original_path: None,
                        pending: 0,
                        error: Some(error),
                    }),
                }
            }
        }
        entries.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then(a.id.cmp(&b.id)));
        Ok(entries)
    }
    pub fn work_delete(&self, id: &str) -> Result<(), String> {
        let (claim, path) = self.claim_work(id)?;
        let base = read_work(&path)
            .ok()
            .and_then(|record| serde_json::to_vec(&record.confirmed).ok());
        if path
            .symlink_metadata()
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("recovery_invalid_file".into());
        }
        fs::remove_file(path).map_err(|e| e.to_string())?;
        if let Some(base) = base {
            clean_summary(&claim.directory, &base);
        }
        Ok(())
    }
    pub fn work_restore(&mut self, window: &str, id: &str, revision: u64) -> Result<(), String> {
        self.lifecycle(revision)?;
        let (claim, path) = self.claim_work(id)?;
        let record = read_work(&path)?;
        let base = serde_json::to_vec(&record.confirmed).map_err(|e| e.to_string())?;
        self.restore_work_record(window, record)?;
        // Destination is durable before the source is consumed. Retain unreadable siblings.
        fs::remove_file(path)
            .map_err(|e| eprintln!("Recovered copy retained: {e}"))
            .ok();
        if claim.directory != self.root {
            clean_summary(&claim.directory, &base);
        }
        Ok(())
    }
    pub fn restore_work_record(
        &mut self,
        window: &str,
        record: drafts::Record,
    ) -> Result<(), String> {
        let validated = Session::from_checkpoint(self.root.clone(), record.confirmed.clone())?;
        let recovered = items(&record);
        if recovered.len() > 2048 {
            return Err("recovery_too_large".into());
        }
        let mut ids = std::collections::HashSet::new();
        for item in &recovered {
            if item.id.is_empty()
                || item.id.len() > 256
                || !ids.insert(&item.id)
                || !["form", "field", "body"].contains(&item.kind.as_str())
                || !item.payload.is_object()
            {
                return Err("recovery_invalid_item".into());
            }
        }
        let generation = uuid::Uuid::new_v4().to_string();
        let mut copied = record.clone();
        copied.name = Some(name(&record));
        copied.window = window.into();
        copied.file_path = None;
        copied.input = drafts::Input {
            generation: generation.clone(),
            epoch: 0,
            sequence: 0,
            update: record.confirmed.update.clone(),
            document: serde_json::to_value(&record.confirmed.document)
                .map_err(|e| e.to_string())?,
            settings: serde_json::to_value(&record.confirmed.settings)
                .map_err(|e| e.to_string())?,
            fields: vec![],
            forms: vec![],
            resources: record.confirmed.resources.clone(),
            rejected: false,
            recovered: recovered.clone(),
            retain: true,
        };
        copied.rejected_edition = None;
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        drafts::write(&drafts::path(&self.root, &generation, window)?, &copied)?;
        // Cleanup belongs to the prepared transition, after the destination is durable.
        // No fallible operation follows this cleanup before publication.
        self.mark_clean()?;
        self.file_lease = None;
        self.replace(
            validated.fixture.unwrap(),
            validated.settings,
            validated.assets,
            validated.updates.into_iter().next().unwrap(),
        );
        self.generation = generation;
        self.recovered_name = copied.name;
        self.recovered_items.insert(window.into(), recovered);
        Ok(())
    }
    pub fn abandon_window_work(&mut self, window: &str, generation: &str) -> Result<(), String> {
        self.check_peer(window, generation)?;
        let path = drafts::path(&self.root, generation, window)?;
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        if self.synchronization().windows.len() <= 1
            && let Some(store) = &self.recovery
        {
            store.clear()?;
        }
        self.recovered_items.remove(window);
        self.draft_epochs.remove(window);
        Ok(())
    }
}
fn has_envelopes(root: &Path) -> bool {
    fs::read_dir(root.join("drafts")).is_ok_and(|dirs| {
        dirs.flatten()
            .filter(|d| d.file_type().is_ok_and(|t| t.is_dir()))
            .any(|d| {
                fs::read_dir(d.path()).is_ok_and(|files| {
                    files
                        .flatten()
                        .any(|f| f.path().extension().is_some_and(|e| e == "json"))
                })
            })
    })
}
fn clean_summary(root: &Path, base: &[u8]) {
    if has_envelopes(root) {
        return;
    }
    let legacy = root.join("session.json");
    if read_work(&legacy)
        .ok()
        .and_then(|record| serde_json::to_vec(&record.confirmed).ok())
        .is_some_and(|bytes| bytes == base)
    {
        let _ = fs::remove_file(legacy);
    }
}
fn name(record: &drafts::Record) -> String {
    record
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|p| p.to_string_lossy().into_owned())
        .or_else(|| record.name.clone())
        .unwrap_or_else(|| typstographe_i18n::text("untitled-letter"))
}
pub fn items(record: &drafts::Record) -> Vec<Item> {
    let mut items = record.input.recovered.clone();
    for form in &record.input.forms {
        if let Some(source) = form["sourceId"]
            .as_str()
            .and_then(|id| items.iter_mut().find(|i| i.id == id))
        {
            source.payload = form.clone();
        } else {
            items.push(Item {
                id: uuid::Uuid::new_v4().to_string(),
                kind: "form".into(),
                payload: form.clone(),
            });
        }
    }
    for field in &record.input.fields {
        let mut payload = field.clone();
        let target = &field["target"];
        let base = match target["kind"].as_str() {
            Some("metadata") => record
                .confirmed
                .settings
                .metadata
                .field_values()
                .into_iter()
                .find(|f| serde_json::to_value(f.field).ok().as_ref() == Some(&target["field"]))
                .map(|f| f.value),
            Some("option") => serde_json::to_value(record.confirmed.settings.metadata.options())
                .ok()
                .and_then(|v| v.get(target["name"].as_str()?).cloned()),
            Some("model") => Some(
                json!({"modelId":record.confirmed.settings.model_id,"template":record.confirmed.settings.template}),
            ),
            _ => None,
        };
        if payload.get("base").is_none()
            && let Some(base) = base
        {
            payload["base"] = base;
        }
        if let Some(source) = field["sourceId"]
            .as_str()
            .and_then(|id| items.iter_mut().find(|i| i.id == id))
        {
            source.payload = payload;
        } else {
            items.push(Item {
                id: uuid::Uuid::new_v4().to_string(),
                kind: "field".into(),
                payload,
            });
        }
    }
    let same = SharedDocument::from_update(&record.input.update)
        .and_then(|shared| {
            shared.verify(&State::new(
                record.confirmed.document.clone(),
                record.confirmed.settings.clone(),
            ))
        })
        .is_ok();
    if record.input.rejected || !same {
        items.push(Item {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "body".into(),
            payload: json!({"input":record.input,"rejectedEdition":record.rejected_edition}),
        });
    }
    items
}
pub fn read_work(path: &Path) -> Result<drafts::Record, String> {
    if !path
        .symlink_metadata()
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("recovery_invalid_file".into());
    }
    if let Ok(record) = drafts::read(path) {
        return Ok(record);
    }
    if path
        .symlink_metadata()
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("recovery_invalid_file".into());
    }
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("recovery_too_large".into());
    }
    let mut value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if value["format"] == "typstographe-rejected-draft-v1" {
        let checkpoint: Checkpoint = if value.get("confirmed").is_some() {
            serde_json::from_value(value["confirmed"].take()).map_err(|e| e.to_string())?
        } else {
            let state = State::new(
                serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph"}]}))
                    .map_err(|e| e.to_string())?,
                Default::default(),
            );
            let shared = SharedDocument::from_state(&state).map_err(crate::owner::error)?;
            Checkpoint {
                schema: "typstographe-web-editor-v1".into(),
                source: String::new(),
                document: state.document,
                settings: state.settings,
                update: shared.update(),
                resources: vec![],
            }
        };
        let input = drafts::Input {
            generation: uuid::Uuid::new_v4().to_string(),
            epoch: 0,
            sequence: 0,
            update: checkpoint.update.clone(),
            document: json!(checkpoint.document),
            settings: json!(checkpoint.settings),
            fields: vec![],
            forms: vec![],
            resources: checkpoint.resources.clone(),
            rejected: false,
            retain: true,
            recovered: vec![Item {
                id: uuid::Uuid::new_v4().to_string(),
                kind: "body".into(),
                payload: value,
            }],
        };
        return Ok(drafts::Record {
            format: drafts::FORMAT.into(),
            name: path.file_name().map(|p| p.to_string_lossy().into_owned()),
            window: String::new(),
            saved_at: 0,
            file_path: None,
            confirmed_revision: 0,
            confirmed: checkpoint,
            input,
            rejected_edition: None,
        });
    }
    let legacy: super::Record = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let cp: Checkpoint = legacy.checkpoint;
    Ok(drafts::Record {
        format: drafts::FORMAT.into(),
        name: Some(legacy.name),
        window: String::new(),
        saved_at: legacy.saved_at,
        file_path: None,
        input: drafts::Input {
            generation: uuid::Uuid::new_v4().to_string(),
            epoch: 0,
            sequence: 0,
            update: cp.update.clone(),
            document: serde_json::to_value(&cp.document).map_err(|e| e.to_string())?,
            settings: serde_json::to_value(&cp.settings).map_err(|e| e.to_string())?,
            fields: vec![],
            forms: vec![],
            resources: cp.resources.clone(),
            rejected: false,
            recovered: vec![],
            retain: false,
        },
        confirmed_revision: 0,
        confirmed: cp,
        rejected_edition: None,
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
