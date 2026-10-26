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
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub revision: u64,
    pub document: Node,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Checkpoint {
    pub schema: String,
    pub document: Node,
    pub source: String,
    pub update: Vec<u8>,
    pub resources: Vec<Resource>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub revision: u64,
    pub updates: Vec<Vec<u8>>,
    pub fixture: Node,
    pub initialize: bool,
    pub resources: Vec<Resource>,
    pub restored_document: Option<Node>,
    pub session_path: String,
}

pub struct Session {
    pub revision: u64,
    pub updates: Vec<Vec<u8>>,
    pub assets: Assets,
    pub root: PathBuf,
    peers: HashSet<String>,
    bytes: usize,
    initialized: bool,
    restored: Option<Node>,
}
impl Session {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        let mut session = Self {
            revision: 0,
            updates: vec![],
            assets: Assets::default(),
            root,
            peers: HashSet::new(),
            bytes: 0,
            initialized: false,
            restored: None,
        };
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
            if schema::source(&checkpoint.document, &session.assets)? != checkpoint.source {
                return Err("Source et instantané incohérents.".into());
            }
            session.bytes = checkpoint.update.len();
            session.updates.push(checkpoint.update);
            session.restored = Some(checkpoint.document);
            session.initialized = true;
        } else {
            session.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../../../../fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
        }
        Ok(session)
    }
    pub fn join(&mut self, peer: &str) -> Bootstrap {
        let initialize = !self.initialized;
        self.initialized = true;
        self.peers.insert(peer.to_owned());
        Bootstrap {
            revision: self.revision,
            updates: self.updates.clone(),
            fixture: serde_json::from_str(include_str!("../../fixture.json")).expect("fixture"),
            initialize,
            resources: self.resources(),
            restored_document: self.restored.take(),
            session_path: self.root.display().to_string(),
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
        self.updates.push(update.to_vec());
        self.revision += 1;
        Ok(self.revision)
    }
    pub fn validate(&self, snapshot: &Snapshot) -> Result<String, String> {
        if snapshot.revision != self.revision {
            return Err("stale".into());
        }
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
        let checkpoint = Checkpoint {
            schema: SCHEMA.into(),
            document: snapshot.document.clone(),
            source,
            update,
            resources: self.resources(),
        };
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
mod tests {
    use super::*;
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
        };
        s.save(&snapshot, vec![1, 2]).unwrap();
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
        assert_eq!(restored.updates, vec![vec![1, 2]]);
        assert!(restored.restored_document.is_some());
    }
    #[test]
    fn malformed_snapshots_missing_resources_and_unknown_versions_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::open(dir.path().into()).unwrap();
        let mut snapshot = Snapshot {
            revision: 0,
            document: s.join("a").fixture,
        };
        s.assets.clear();
        assert!(s.save(&snapshot, vec![1]).is_err());
        snapshot.document.kind = "script".into();
        assert!(s.save(&snapshot, vec![1]).is_err());
        std::fs::write(dir.path().join("session.json"), r#"{"schema":"other"}"#).unwrap();
        assert!(Session::open(dir.path().into()).is_err());
    }
}
