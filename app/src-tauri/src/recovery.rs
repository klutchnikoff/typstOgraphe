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
        for entry in std::fs::read_dir(self.base.join("sessions")).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            // Empty slots retain their stable lock sidecar, but must not hide
            // recovery copies after many normal application launches.
            if !entry.path().join("session.json").is_file() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                ids.push(name.into());
            }
            if ids.len() > 4096 {
                break;
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
        let options = atomic_write_file::AtomicWriteFile::options();
        #[cfg(unix)]
        let options = {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = options;
            options.mode(0o600);
            options
        };
        let mut file = options.open(&path).map_err(|e| e.to_string())?;
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
        let document: crate::schema::Node = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Brouillon conservé"}]}]})).unwrap();
        let update = typstographe_core::content::shared::SharedDocument::from_state(
            &typstographe_core::content::contract::State::new(document.clone(), Default::default()),
        )
        .unwrap()
        .update();
        Checkpoint {
            schema: "typstographe-web-editor-v1".into(),
            source: String::new(),
            document,
            update,
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
        session.save(&snapshot, session.updates[0].clone()).unwrap();
        assert!(!session.root.join("session.json").exists());
        session.revision += 1;
        let snapshot = crate::session::Snapshot {
            revision: session.revision,
            ..snapshot
        };
        session.save(&snapshot, session.updates[0].clone()).unwrap();
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
    fn working(base: &std::path::Path) -> crate::session::Session {
        let mut s = crate::session::Session::managed(base.into()).unwrap();
        let mut cp = checkpoint();
        cp.document.content[0].content[0].text = Some("Travail courant à préserver".into());
        cp.update = typstographe_core::content::shared::SharedDocument::from_state(
            &typstographe_core::content::contract::State::new(
                cp.document.clone(),
                cp.settings.clone(),
            ),
        )
        .unwrap()
        .update();
        s.replace(cp.document, cp.settings, Default::default(), cp.update);
        s.join("human");
        s.ensure_owner().unwrap().snapshot("human").unwrap();
        let snapshot = crate::session::Snapshot {
            revision: s.revision,
            document: s.fixture.clone().unwrap(),
            settings: Some(s.settings.clone()),
        };
        s.save(&snapshot, s.updates[0].clone()).unwrap();
        s
    }
    fn preserved(s: &mut crate::session::Session) -> serde_json::Value {
        let context = s.ensure_owner().unwrap().snapshot("human").unwrap();
        serde_json::json!({"guard":s.guard(),"updates":s.updates,"settings":s.settings,
            "document":s.owner.as_ref().unwrap().state(),"history":context,"clean":s.clean_revision})
    }
    #[test]
    fn invalid_shared_checkpoint_restoration_keeps_the_current_work_and_the_original_copy() {
        let dir = tempfile::tempdir().unwrap();
        let old = Store::new(dir.path().into()).unwrap();
        let id = old
            .directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let mut cp = checkpoint();
        cp.document.content[0].content[0].text = Some("Visible content disagrees with Yjs".into());
        let path = old.write(cp, "Corrupt but valid JSON".into()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        drop(old);
        let mut s = working(dir.path());
        let current = std::fs::read(s.root.join("session.json")).unwrap();
        let state = preserved(&mut s);
        assert!(restore(&mut s, &id).is_err());
        assert_eq!(preserved(&mut s), state);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert_eq!(std::fs::read(s.root.join("session.json")).unwrap(), current);
    }
    #[test]
    fn recovery_copy_write_failure_precedes_publication_and_can_be_retried_without_losing_either_draft()
     {
        let dir = tempfile::tempdir().unwrap();
        let old = Store::new(dir.path().into()).unwrap();
        let id = old
            .directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let path = old.write(checkpoint(), "À récupérer".into()).unwrap();
        let candidate = std::fs::read(&path).unwrap();
        drop(old);
        let mut s = working(dir.path());
        let state = preserved(&mut s);
        let current_path = s.root.join("session.json");
        let current = std::fs::read(&current_path).unwrap();
        let backup = s.root.join("current.backup");
        std::fs::rename(&current_path, &backup).unwrap();
        std::fs::create_dir(&current_path).unwrap(); // Portable deterministic IO failure.
        assert!(restore(&mut s, &id).is_err());
        assert_eq!(preserved(&mut s), state);
        assert_eq!(std::fs::read(&path).unwrap(), candidate);
        assert_eq!(std::fs::read(&backup).unwrap(), current);
        std::fs::remove_dir(&current_path).unwrap();
        std::fs::rename(&backup, &current_path).unwrap();
        restore(&mut s, &id).unwrap();
        assert!(!path.exists());
        let restored: Record =
            serde_json::from_slice(&std::fs::read(&current_path).unwrap()).unwrap();
        assert_eq!(restored.name, "À récupérer");
        assert_eq!(restored.checkpoint.document, s.fixture.clone().unwrap());
        assert_eq!(s.clean_revision, None);
        assert_ne!(
            s.guard(),
            serde_json::from_value(state["guard"].clone()).unwrap()
        );
    }
    #[cfg(unix)]
    #[test]
    fn recovery_symlinks_cannot_read_or_delete_an_external_draft() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let old = Store::new(dir.path().into()).unwrap();
        let id = old
            .directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let slot = old.directory.clone();
        drop(old);
        let external = dir.path().join("external.json");
        let bytes = serde_json::to_vec(&checkpoint()).unwrap();
        std::fs::write(&external, &bytes).unwrap();
        symlink(&external, slot.join("session.json")).unwrap();
        let s = working(dir.path());
        assert!(s.recovery.as_ref().unwrap().claim(&id).is_err());
        assert!(s.recovery.as_ref().unwrap().list().unwrap().is_empty());
        assert_eq!(std::fs::read(&external).unwrap(), bytes);
        std::fs::remove_file(slot.join("session.json")).unwrap();
        let elsewhere = dir.path().join("external-directory");
        std::fs::rename(&slot, &elsewhere).unwrap();
        std::fs::write(elsewhere.join("session.json"), &bytes).unwrap();
        symlink(&elsewhere, &slot).unwrap();
        assert!(s.recovery.as_ref().unwrap().claim(&id).is_err());
        assert_eq!(
            std::fs::read(elsewhere.join("session.json")).unwrap(),
            bytes
        );
    }
    fn signed_checkpoint() -> Checkpoint {
        use base64::Engine;
        let mut cp = checkpoint();
        let typstographe_core::document_types::Metadata::Courrier(meta) = &mut cp.settings.metadata
        else {
            unreachable!()
        };
        meta.signature = Some("images/signature.svg".into());
        cp.resources = vec![crate::session::Resource {
            path: "images/signature.svg".into(),
            data: base64::engine::general_purpose::STANDARD
                .encode(include_bytes!("../../../fixtures/signature.svg")),
        }];
        cp.update = typstographe_core::content::shared::SharedDocument::from_state(
            &typstographe_core::content::contract::State::new(
                cp.document.clone(),
                cp.settings.clone(),
            ),
        )
        .unwrap()
        .update();
        cp
    }
    #[test]
    fn restoration_requires_signature_bytes_before_replacing_work_and_preserves_valid_resources() {
        for missing in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let old = Store::new(dir.path().into()).unwrap();
            let id = old
                .directory
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let mut cp = signed_checkpoint();
            if missing {
                cp.resources.clear();
            }
            let expected = cp.clone();
            let path = old.write(cp, "Courrier signé".into()).unwrap();
            let original = std::fs::read(&path).unwrap();
            drop(old);
            let mut s = working(dir.path());
            let before = preserved(&mut s);
            let recovery = std::fs::read(s.root.join("session.json")).unwrap();
            let result = restore(&mut s, &id);
            if missing {
                assert!(result.is_err(), "a missing signature must not be published");
                assert_eq!(preserved(&mut s), before);
                assert_eq!(std::fs::read(path).unwrap(), original);
                assert_eq!(
                    std::fs::read(s.root.join("session.json")).unwrap(),
                    recovery
                );
            } else {
                result.unwrap();
                assert!(!path.exists());
                assert_eq!(s.settings, expected.settings);
                assert_eq!(s.resources()[0].data, expected.resources[0].data);
                let state = s.ensure_owner().unwrap().state();
                assert_eq!(state.document, expected.document);
            }
        }
    }
}
