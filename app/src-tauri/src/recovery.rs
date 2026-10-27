//! Per-process crash recovery, distinct from durable project versions.
use crate::{Bridge, session::Checkpoint};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, State};
use typstographe_core::{DocumentLease, assets::MAX_DOCUMENT_BYTES};

pub struct Store {
    base: PathBuf,
    pub directory: PathBuf,
    _owner: DocumentLease,
}
#[derive(Serialize, Deserialize)]
struct Record {
    name: String,
    saved_at: u64,
    checkpoint: Checkpoint,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    id: String,
    name: String,
    saved_at: u64,
    error: Option<String>,
}
struct Candidate {
    path: PathBuf,
    _owner: DocumentLease,
}
impl Candidate {
    fn read(&self) -> Result<Record, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(&self.path)
            .map_err(|e| e.to_string())?
            .take(MAX_DOCUMENT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err("Copie de récupération trop volumineuse.".into());
        }
        // Keep the last draft made before per-process recovery was introduced.
        serde_json::from_slice(&bytes)
            .or_else(|_| {
                serde_json::from_slice::<Checkpoint>(&bytes).map(|checkpoint| Record {
                    name: "Document récupéré".into(),
                    saved_at: 0,
                    checkpoint,
                })
            })
            .map_err(|e| e.to_string())
    }
    fn remove(&self) -> Result<(), String> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
impl Store {
    pub fn new(base: PathBuf) -> Result<Self, String> {
        let directory = base.join("sessions").join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let owner = DocumentLease::acquire(&directory.join("owner")).map_err(|e| e.to_string())?;
        Ok(Self {
            base,
            directory,
            _owner: owner,
        })
    }
    fn claim(&self, id: &str) -> Result<Candidate, String> {
        let directory = if id == "previous" {
            self.base.clone()
        } else {
            let value =
                uuid::Uuid::parse_str(id).map_err(|_| "Référence de récupération invalide.")?;
            if value.to_string() != id {
                return Err("Référence de récupération invalide.".into());
            }
            self.base.join("sessions").join(id)
        };
        if directory
            .symlink_metadata()
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Dossier de récupération invalide.".into());
        }
        let path = directory.join("session.json");
        if path
            .symlink_metadata()
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Copie de récupération invalide.".into());
        }
        let owner = DocumentLease::acquire(&directory.join("owner")).map_err(|e| e.to_string())?;
        Ok(Candidate {
            path,
            _owner: owner,
        })
    }
    pub fn list(&self) -> Result<Vec<Entry>, String> {
        let mut ids = vec!["previous".to_owned()];
        for entry in std::fs::read_dir(self.base.join("sessions"))
            .map_err(|e| e.to_string())?
            .take(4096)
        {
            let entry = entry.map_err(|e| e.to_string())?;
            if let Some(name) = entry.file_name().to_str() {
                ids.push(name.into());
            }
        }
        let mut entries = Vec::new();
        for id in ids {
            // A live owner, including our own session, is never offered for recovery.
            let Ok(candidate) = self.claim(&id) else {
                continue;
            };
            match candidate.read() {
                Ok(record) => entries.push(Entry {
                    id,
                    name: record.name,
                    saved_at: record.saved_at,
                    error: None,
                }),
                Err(error) => entries.push(Entry {
                    id,
                    name: "Copie illisible".into(),
                    saved_at: 0,
                    error: Some(error),
                }),
            }
        }
        entries.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then(a.id.cmp(&b.id)));
        Ok(entries)
    }
    pub fn write(&self, checkpoint: Checkpoint, name: String) -> Result<PathBuf, String> {
        let path = self.directory.join("session.json");
        let record = Record {
            checkpoint,
            name,
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs(),
        };
        let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err("Copie de récupération trop volumineuse.".into());
        }
        let mut file =
            atomic_write_file::AtomicWriteFile::open(&path).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.commit().map_err(|e| e.to_string())?;
        Ok(path)
    }
    pub fn clear(&self) -> Result<(), String> {
        match std::fs::remove_file(self.directory.join("session.json")) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
#[tauri::command]
pub fn recovery_list(state: State<Bridge>) -> Result<Vec<Entry>, String> {
    let s = state.session.lock().map_err(|e| e.to_string())?;
    s.recovery.as_ref().map_or(Ok(vec![]), Store::list)
}
#[tauri::command]
pub fn recovery_delete(state: State<Bridge>, id: String) -> Result<(), String> {
    let s = state.session.lock().map_err(|e| e.to_string())?;
    s.recovery
        .as_ref()
        .ok_or("Récupération indisponible.")?
        .claim(&id)?
        .remove()
}
fn restore(s: &mut crate::session::Session, id: &str) -> Result<(), String> {
    let candidate = s
        .recovery
        .as_ref()
        .ok_or("Récupération indisponible.")?
        .claim(id)?;
    let record = candidate.read()?;
    let validated =
        crate::session::Session::from_checkpoint(s.root.clone(), record.checkpoint.clone())?;
    s.recovery
        .as_ref()
        .unwrap()
        .write(record.checkpoint, record.name)?;
    s.file_lease = None;
    s.replace(
        validated.fixture.unwrap(),
        validated.settings,
        validated.assets,
        validated.updates.into_iter().next().unwrap(),
    );
    // A retained duplicate is preferable to reporting a failed restoration after publication.
    if let Err(error) = candidate.remove() {
        eprintln!("Nettoyage de la copie restaurée : {error}");
    }
    Ok(())
}
#[tauri::command]
pub fn recovery_restore(
    app: tauri::AppHandle,
    state: State<Bridge>,
    id: String,
    revision: u64,
) -> Result<(), String> {
    let mut s = state.session.lock().map_err(|e| e.to_string())?;
    s.lifecycle(revision)?;
    restore(&mut s, &id)?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    agent.snapshot = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn recovery_discard(app: tauri::AppHandle, state: State<Bridge>) -> Result<(), String> {
    if app.webview_windows().len() > 1 {
        return Ok(());
    }
    let mut s = state.session.lock().map_err(|e| e.to_string())?;
    s.mark_clean()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    fn checkpoint() -> Checkpoint {
        let document = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Brouillon conservé"}]}]})).unwrap();
        Checkpoint {
            schema: "typstographe-web-editor-v1".into(),
            source: String::new(),
            document,
            update: vec![0, 0],
            resources: vec![],
            settings: Default::default(),
        }
    }
    #[test]
    fn live_processes_have_distinct_slots_and_cannot_recover_or_delete_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let first = Store::new(dir.path().into()).unwrap();
        let second = Store::new(dir.path().into()).unwrap();
        assert_ne!(first.directory, second.directory);
        first.write(checkpoint(), "Lettre.typsto".into()).unwrap();
        assert!(second.list().unwrap().is_empty());
        let id = first
            .directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        assert!(second.claim(&id).is_err());
        drop(first);
        let entries = second.list().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Lettre.typsto");
        assert!(second.claim("../../elsewhere").is_err());
    }
    #[test]
    fn restoration_copies_before_consumption_and_a_clean_revision_cannot_reappear() {
        let dir = tempfile::tempdir().unwrap();
        let first = Store::new(dir.path().into()).unwrap();
        let id = first
            .directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        first.write(checkpoint(), "Brouillon".into()).unwrap();
        drop(first);
        let mut session = crate::session::Session::managed(dir.path().into()).unwrap();
        restore(&mut session, &id).unwrap();
        assert!(session.opened);
        assert!(
            session
                .recovery
                .as_ref()
                .unwrap()
                .list()
                .unwrap()
                .is_empty()
        );
        assert!(session.root.join("session.json").exists());
        session.mark_clean().unwrap();
        let snapshot = crate::session::Snapshot {
            revision: session.revision,
            document: session.fixture.clone().unwrap(),
            settings: Some(session.settings.clone()),
        };
        session.save(&snapshot, vec![0, 0]).unwrap();
        assert!(!session.root.join("session.json").exists());
        session.revision += 1;
        let snapshot = crate::session::Snapshot {
            revision: session.revision,
            ..snapshot
        };
        session.save(&snapshot, vec![0, 0]).unwrap();
        assert!(session.root.join("session.json").exists());
    }
    #[test]
    fn corrupt_copies_do_not_block_startup_and_failed_restoration_preserves_them() {
        let dir = tempfile::tempdir().unwrap();
        let first = Store::new(dir.path().into()).unwrap();
        let id = first
            .directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        std::fs::write(first.directory.join("session.json"), b"broken").unwrap();
        drop(first);
        let mut session = crate::session::Session::managed(dir.path().into()).unwrap();
        assert!(
            session.recovery.as_ref().unwrap().list().unwrap()[0]
                .error
                .is_some()
        );
        let before = session.revision;
        assert!(restore(&mut session, &id).is_err());
        assert_eq!(session.revision, before);
        session
            .recovery
            .as_ref()
            .unwrap()
            .claim(&id)
            .unwrap()
            .remove()
            .unwrap();
        assert!(
            session
                .recovery
                .as_ref()
                .unwrap()
                .list()
                .unwrap()
                .is_empty()
        );
    }
}
