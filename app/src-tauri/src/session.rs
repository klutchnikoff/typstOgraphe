use crate::schema::{self, Node};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use typstographe_core::assets::{self, Assets};

const MAX_UPDATES: usize = 32 * 1024 * 1024;
const SCHEMA: &str = "typstographe-web-editor-v1";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub path: String,
    pub data: String,
}
impl Resource {
    pub fn decoded(&self) -> Result<Vec<u8>, String> {
        if !assets::valid_path(&self.path) {
            return Err("Chemin de ressource invalide.".into());
        }
        let bytes = STANDARD.decode(&self.data).map_err(|e| e.to_string())?;
        assets::decode(&bytes, self.path.rsplit('.').next().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}
pub use typstographe_core::content::contract::Snapshot;
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Checkpoint {
    pub schema: String,
    pub document: Node,
    pub source: String,
    pub update: Vec<u8>,
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub settings: crate::settings::Settings,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub generation: String,
    pub opened: bool,
    pub has_recovery: bool,
    pub initial_path: Option<String>,
    pub file_path: Option<String>,
    pub revision: u64,
    pub updates: Vec<Vec<u8>>,
    pub fixture: Node,
    pub initialize: bool,
    pub resources: Vec<Resource>,
    pub restored_document: Option<Node>,
    pub session_path: String,
    pub settings: crate::settings::Settings,
}

pub struct Session {
    pub recovery: Option<crate::recovery::Store>,
    pub initial_path: Option<String>,
    pub clean_revision: Option<u64>,
    pub offline: HashSet<String>,
    pub generation: String,
    pub opened: bool,
    pub fixture: Option<Node>,
    pub file_lease: Option<typstographe_core::DocumentLease>,
    pub revision: u64,
    pub updates: Vec<Vec<u8>>,
    pub assets: Assets,
    pub root: PathBuf,
    peers: HashSet<String>,
    bytes: usize,
    initialized: bool,
    restored: Option<Node>,
    pub settings: crate::settings::Settings,
}
impl Session {
    pub fn fresh(root: PathBuf) -> Self {
        Self {
            recovery: None,
            initial_path: None,
            clean_revision: None,
            offline: HashSet::new(),
            generation: uuid::Uuid::new_v4().to_string(),
            opened: false,
            fixture: None,
            file_lease: None,
            revision: 0,
            updates: vec![],
            assets: Assets::default(),
            root,
            peers: HashSet::new(),
            bytes: 0,
            initialized: false,
            restored: None,
            settings: Default::default(),
        }
    }
    pub fn open(root: PathBuf) -> Result<Self, String> {
        let mut session = Self::fresh(root);
        let path = session.root.join("session.json");
        if path.exists() {
            let mut bytes = Vec::new();
            std::fs::File::open(&path)
                .map_err(|e| e.to_string())?
                .take(assets::MAX_DOCUMENT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > assets::MAX_DOCUMENT_BYTES {
                return Err("Session trop volumineuse.".into());
            }
            let checkpoint: Checkpoint =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            session = Self::from_checkpoint(session.root.clone(), checkpoint)?;
        } else {
            session.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../../../fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
        }
        Ok(session)
    }
    pub fn from_checkpoint(root: PathBuf, checkpoint: Checkpoint) -> Result<Self, String> {
        let mut session = Self::fresh(root);
        if checkpoint.schema != SCHEMA
            || checkpoint.update.is_empty()
            || checkpoint.update.len() > MAX_UPDATES
        {
            return Err("Version ou état Yjs de session invalide.".into());
        }
        for resource in checkpoint.resources {
            let data = resource.decoded()?;
            if session.assets.insert(resource.path, data.into()).is_some() {
                return Err("Ressource dupliquée.".into());
            }
        }
        // Le source de récupération est un cache : régénérer depuis le schéma validé.
        schema::source(&checkpoint.document, &session.assets)?;
        session.bytes = checkpoint.update.len();
        checkpoint
            .settings
            .validate()
            .map_err(|e| typstographe_i18n::content_error(&e))?;
        typstographe_core::content::shared::SharedDocument::from_update(&checkpoint.update)
            .and_then(|shared| {
                shared.verify(&typstographe_core::content::contract::State::new(
                    checkpoint.document.clone(),
                    checkpoint.settings.clone(),
                ))
            })
            .map_err(|e| typstographe_i18n::content_error(&e))?;
        session.settings = checkpoint.settings;
        session.updates.push(checkpoint.update);
        session.restored = Some(checkpoint.document);
        session.initialized = true;
        session.fixture = session.restored.clone();
        Ok(session)
    }
    pub fn managed(base: PathBuf) -> Result<Self, String> {
        let recovery = crate::recovery::Store::new(base)?;
        let mut session = Self::fresh(recovery.directory.clone());
        session.fixture = Some(
            serde_json::from_value(
                serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}),
            )
            .expect("empty document"),
        );
        session.recovery = Some(recovery);
        Ok(session)
    }
    pub fn mark_clean(&mut self) -> Result<(), String> {
        if let Some(store) = &self.recovery {
            store.clear()?;
        }
        self.clean_revision = Some(self.revision);
        Ok(())
    }
    pub fn join(&mut self, peer: &str) -> Bootstrap {
        let initialize = !self.initialized;
        self.initialized = true;
        self.peers.insert(peer.to_owned());
        Bootstrap {
            generation: self.generation.clone(),
            opened: self.opened,
            has_recovery: if let Some(store) = &self.recovery {
                !store.list().unwrap_or_default().is_empty()
            } else {
                self.root.join("session.json").exists()
            },
            initial_path: self.initial_path.take(),
            file_path: self
                .file_lease
                .as_ref()
                .map(|lease| lease.path().to_string_lossy().into_owned()),
            revision: self.revision,
            updates: self.updates.clone(),
            fixture: self.fixture.clone().unwrap_or_else(|| {
                serde_json::from_str(include_str!("../../fixture.json")).expect("fixture")
            }),
            initialize,
            resources: self.resources(),
            restored_document: self.restored.take(),
            session_path: self.root.display().to_string(),
            settings: self.settings.clone(),
        }
    }
    pub fn relay(&mut self, peer: &str, update: &[u8]) -> Result<u64, String> {
        if !self.peers.contains(peer) {
            return Err("Fenêtre non rattachée.".into());
        }
        if update.is_empty()
            || update.len() > 4 * 1024 * 1024
            || self.bytes + update.len() > MAX_UPDATES
        {
            return Err(
                "Limite du journal Yjs atteinte ; enregistrez et relancez le diagnostic.".into(),
            );
        }
        self.bytes += update.len();
        self.restored = None;
        self.updates.push(update.to_vec());
        self.revision += 1;
        Ok(self.revision)
    }
    pub fn lifecycle(&self, revision: u64) -> Result<(), String> {
        if self.revision != revision {
            return Err("Le document a changé ; réessayez après synchronisation.".into());
        }
        if !self.offline.is_empty() {
            return Err("Reconnectez les autres fenêtres avant de remplacer le document.".into());
        }
        Ok(())
    }
    pub fn replace(
        &mut self,
        document: Node,
        settings: crate::settings::Settings,
        assets: Assets,
        yjs: Vec<u8>,
    ) {
        self.clean_revision = None;
        self.offline.clear();
        self.generation = uuid::Uuid::new_v4().to_string();
        self.revision += 1;
        self.bytes = yjs.len();
        self.initialized = !yjs.is_empty();
        self.updates = if yjs.is_empty() { vec![] } else { vec![yjs] };
        self.restored = self.initialized.then(|| document.clone());
        self.fixture = Some(document);
        self.settings = settings;
        self.assets = assets;
        self.opened = true;
    }
    pub fn validate(&self, snapshot: &Snapshot) -> Result<String, String> {
        if snapshot.revision != self.revision {
            return Err("stale".into());
        }
        let settings = snapshot.settings.as_ref().unwrap_or(&self.settings);
        settings
            .validate()
            .map_err(|e| typstographe_i18n::content_error(&e))?;
        typstographe_core::bibliography::validate_document(&snapshot.document, &settings.metadata)?;
        schema::source(&snapshot.document, &self.assets)
    }
    pub fn resources(&self) -> Vec<Resource> {
        self.assets
            .iter()
            .map(|(path, data)| Resource {
                path: path.clone(),
                data: STANDARD.encode(data),
            })
            .collect()
    }
    pub fn import(&mut self, extension: &str, data: &[u8]) -> Result<Resource, String> {
        assets::decode(data, extension).map_err(|e| e.to_string())?;
        let total: usize = self.assets.values().map(|data| data.len()).sum();
        if total + data.len() > assets::MAX_DOCUMENT_BYTES as usize {
            return Err("Limite des images atteinte.".into());
        }
        let extension = if extension == "jpeg" {
            "jpg"
        } else {
            extension
        };
        let hash: String = Sha256::digest(data)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let path = format!("images/{}.{}", &hash[..24], extension);
        if !assets::valid_path(&path) {
            return Err("Extension invalide.".into());
        }
        self.assets.insert(path.clone(), data.to_vec().into());
        Ok(Resource {
            path,
            data: STANDARD.encode(data),
        })
    }
    pub fn save(&self, snapshot: &Snapshot, update: Vec<u8>) -> Result<PathBuf, String> {
        let source = self.validate(snapshot)?;
        if update.is_empty() || update.len() > MAX_UPDATES {
            return Err("État Yjs invalide.".into());
        }
        typstographe_core::content::shared::SharedDocument::from_update(&update)
            .and_then(|shared| {
                shared.verify(&typstographe_core::content::contract::State::new(
                    snapshot.document.clone(),
                    snapshot
                        .settings
                        .clone()
                        .unwrap_or_else(|| self.settings.clone()),
                ))
            })
            .map_err(|e| e.to_string())?;
        let checkpoint = Checkpoint {
            schema: SCHEMA.into(),
            document: snapshot.document.clone(),
            source,
            update,
            resources: self.resources(),
            settings: snapshot
                .settings
                .clone()
                .unwrap_or_else(|| self.settings.clone()),
        };
        if let Some(store) = &self.recovery {
            if self.clean_revision == Some(snapshot.revision) {
                return Ok(self.root.join("session.json"));
            }
            let name = self
                .file_lease
                .as_ref()
                .and_then(|l| l.path().file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Sans titre".into());
            return store.write(checkpoint, name);
        }
        let bytes = serde_json::to_vec(&checkpoint).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > assets::MAX_DOCUMENT_BYTES {
            return Err("Session trop volumineuse.".into());
        }
        std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let path = self.root.join("session.json");
        atomic(&path, &bytes)?;
        Ok(path)
    }
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = atomic_write_file::AtomicWriteFile::open(path).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn document_replacement_requires_a_current_connected_session() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::open(dir.path().into()).unwrap();
        assert!(session.lifecycle(0).is_ok());
        assert!(session.lifecycle(1).is_err());
        session.offline.insert("peer".into());
        assert!(session.lifecycle(0).is_err());
        session.offline.clear();
        assert!(session.lifecycle(0).is_ok());
    }
    #[test]
    fn initialization_is_unique_and_stale_or_invalid_save_cannot_replace_a_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::open(dir.path().into()).unwrap();
        let a = s.join("a");
        assert!(a.initialize);
        assert!(!s.join("b").initialize);
        s.relay("a", &[1, 2]).unwrap();
        let snapshot = Snapshot {
            revision: 1,
            document: a.fixture,
            settings: None,
        };
        let update = typstographe_core::content::shared::SharedDocument::from_state(
            &typstographe_core::content::contract::State::new(
                snapshot.document.clone(),
                s.settings.clone(),
            ),
        )
        .unwrap()
        .update();
        s.save(&snapshot, update.clone()).unwrap();
        let before = std::fs::read(dir.path().join("session.json")).unwrap();
        s.relay("b", &[3]).unwrap();
        assert!(s.save(&snapshot, vec![3]).is_err());
        assert_eq!(
            before,
            std::fs::read(dir.path().join("session.json")).unwrap()
        );
        let mut reopened = Session::open(dir.path().into()).unwrap();
        let restored = reopened.join("c");
        assert!(!restored.initialize);
        assert_eq!(restored.updates, vec![update]);
        assert!(restored.restored_document.is_some());
    }
    #[test]
    fn malformed_snapshots_missing_resources_and_unknown_versions_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::open(dir.path().into()).unwrap();
        let mut snapshot = Snapshot {
            revision: 0,
            document: s.join("a").fixture,
            settings: None,
        };
        s.assets.clear();
        assert!(s.save(&snapshot, vec![1]).is_err());
        snapshot.document.kind = "script".into();
        assert!(s.save(&snapshot, vec![1]).is_err());
        std::fs::write(dir.path().join("session.json"), r#"{"schema":"other"}"#).unwrap();
        assert!(Session::open(dir.path().into()).is_err());
    }
}
