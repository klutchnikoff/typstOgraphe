use crate::schema::{self, Node};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::Write,
    path::{Path, PathBuf},
};
use typstographe_core::assets::{self, Assets};

const MAX_UPDATES: usize = 32 * 1024 * 1024;
const SCHEMA: &str = "typstographe-web-editor-v2";

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
    pub project: crate::project::RecoveryContext,
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
    pub native_editing: bool,
    pub synchronization: Synchronization,
    pub opened: bool,
    pub has_recovery: bool,
    pub recovered_name: Option<String>,
    pub recovered_items: Vec<crate::recovery::work::Item>,
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

/// Ephemeral local services, never part of the document or its history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Synchronization {
    pub generation: String,
    pub version: u64,
    pub enabled: bool,
    pub windows: Vec<String>,
}

pub struct Session {
    pub project: std::sync::OnceLock<crate::project::Context>,
    pub owner: Option<crate::owner::Owner>,
    pub recovery: Option<crate::recovery::Store>,
    pub initial_path: Option<String>,
    pub recovered_name: Option<String>,
    pub recovered_items: std::collections::HashMap<String, Vec<crate::recovery::work::Item>>,
    pub transition: Option<crate::recovery::transition::Transition>,
    pub clean_revision: Option<u64>,
    pub(crate) draft_epochs: std::collections::HashMap<String, (u64, u64)>,
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
    synchronization_enabled: bool,
    synchronization_version: u64,
    bytes: usize,
    initialized: bool,
    restored: Option<Node>,
    pub settings: crate::settings::Settings,
}
impl Session {
    pub fn fresh(root: PathBuf) -> Self {
        Self {
            project: std::sync::OnceLock::new(),
            owner: None,
            recovery: None,
            initial_path: None,
            clean_revision: None,
            recovered_name: None,
            recovered_items: Default::default(),
            transition: None,
            draft_epochs: Default::default(),
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
            synchronization_enabled: false,
            synchronization_version: 0,
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
            let bytes = crate::project::read_recovery_file(&path)?;
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
        // Recovery is an untrusted document, not only a matching Yjs projection.
        let state = typstographe_core::content::contract::State::new(
            checkpoint.document.clone(),
            checkpoint.settings.clone(),
        );
        state
            .validate(&session.assets)
            .map_err(|e| typstographe_i18n::content_error(&e))?;
        session.bytes = checkpoint.update.len();
        typstographe_core::content::shared::SharedDocument::from_update(&checkpoint.update)
            .and_then(|shared| {
                shared.verify(&typstographe_core::content::contract::State::new(
                    checkpoint.document.clone(),
                    checkpoint.settings.clone(),
                ))
            })
            .map_err(|e| typstographe_i18n::content_error(&e))?;
        session.settings = checkpoint.settings;
        let base = checkpoint
            .project
            .archive
            .restore()
            .map_err(crate::project::error)?;
        let dependencies = base.dependencies_for(&session.settings.template);
        // The envelope may overlay a newer confirmed body, but its model must have
        // been pinned in the cached context; never resolve recovery from the catalogue.
        if base.current().template != session.settings.template
            || base.current().model_id != session.settings.model_id
        {
            return Err("recovery_invalid_file".into());
        }
        let current = crate::project::archive(
            state,
            checkpoint.update.clone(),
            session.assets.clone(),
            dependencies,
        );
        let project = base.with_current(current).map_err(crate::project::error)?;
        session.project = std::sync::OnceLock::from(crate::project::Context::new(
            project,
            checkpoint.project.saved_origin,
        ));
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
        if let Some(transition) = &self.transition {
            for window in &transition.abandon {
                let file = crate::recovery::drafts::path(&self.root, &self.generation, window)?;
                if file.exists() {
                    std::fs::remove_file(file).map_err(|e| e.to_string())?;
                }
            }
        }
        if let Some(store) = &self.recovery {
            store.clear()?;
        }
        crate::recovery::drafts::clean_confirmed(&self.root, &self.generation);
        self.clean_revision = Some(self.revision);
        Ok(())
    }
    pub fn join(&mut self, peer: &str) -> Bootstrap {
        let initialize = !self.initialized;
        self.initialized = true;
        if self.peers.insert(peer.to_owned()) {
            self.synchronization_version += 1;
        }
        self.activate_for_windows();
        Bootstrap {
            native_editing: true,
            synchronization: self.synchronization(),
            generation: self.generation.clone(),
            opened: self.opened,
            recovered_name: self.recovered_name.clone(),
            recovered_items: self.recovered_items.get(peer).cloned().unwrap_or_default(),
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
    #[cfg(test)]
    pub fn relay(&mut self, peer: &str, update: &[u8]) -> Result<u64, String> {
        Ok(self.relay_group(peer, update, None, true)?.revision)
    }
    pub fn connection(&mut self, peer: &str, generation: &str, online: bool) -> Result<(), String> {
        self.check_peer(peer, generation)?;
        let changed = if online {
            self.offline.remove(peer)
        } else {
            self.offline.insert(peer.into())
        };
        if changed {
            self.synchronization_version += 1;
        }
        self.activate_for_windows();
        Ok(())
    }
    pub(crate) fn registered_windows(&self) -> Vec<String> {
        self.peers.iter().cloned().collect()
    }
    pub fn leave(&mut self, peer: &str) {
        self.offline.remove(peer);
        if self.peers.remove(peer) {
            self.synchronization_version += 1;
        }
        if let Some(owner) = &mut self.owner {
            let _ = owner.close(peer);
        }
    }
    pub fn synchronization(&self) -> Synchronization {
        let mut windows: Vec<_> = self.peers.difference(&self.offline).cloned().collect();
        windows.sort();
        Synchronization {
            generation: self.generation.clone(),
            version: self.synchronization_version,
            enabled: self.synchronization_enabled,
            windows,
        }
    }
    fn activate_for_windows(&mut self) {
        if !self.synchronization_enabled && self.peers.difference(&self.offline).count() > 1 {
            self.synchronization_enabled = true;
            self.synchronization_version += 1;
        }
    }
    pub fn set_synchronization(
        &mut self,
        peer: &str,
        generation: &str,
        enabled: bool,
    ) -> Result<Synchronization, String> {
        self.check_peer(peer, generation)?;
        if self.offline.contains(peer) {
            return Err("synchronization_offline".into());
        }
        if !enabled && self.peers.difference(&self.offline).count() > 1 {
            return Err("synchronization_other_windows".into());
        }
        if enabled != self.synchronization_enabled {
            self.synchronization_enabled = enabled;
            self.synchronization_version += 1;
        }
        Ok(self.synchronization())
    }
    pub fn accepts_awareness(
        &self,
        peer: &str,
        generation: &str,
        version: u64,
    ) -> Result<bool, String> {
        self.check_peer(peer, generation)?;
        Ok(self.synchronization_enabled
            && self.synchronization_version == version
            && !self.offline.contains(peer))
    }
    /// Confirmation is already returned to the author. Agents are not window peers:
    /// their accepted changes must still reach the sole human window.
    pub fn should_broadcast(&self, origin: &str) -> bool {
        self.peers
            .difference(&self.offline)
            .any(|peer| peer != origin)
    }
    pub fn guard(&self) -> typstographe_core::content::contract::Guard {
        typstographe_core::content::contract::Guard {
            document: self.generation.clone(),
            revision: self.revision,
        }
    }
    pub fn check_peer(&self, peer: &str, generation: &str) -> Result<(), String> {
        if !self.peers.contains(peer) {
            return Err("Fenêtre non inscrite.".into());
        }
        if generation != self.generation {
            return Err(crate::owner::error(
                typstographe_core::content::contract::Error::WrongDocument,
            ));
        }
        Ok(())
    }
    pub fn ensure_owner(&mut self) -> Result<&mut crate::owner::Owner, String> {
        if self.owner.is_none() {
            let shared =
                typstographe_core::content::shared::SharedDocument::from_updates(&self.updates)
                    .map_err(crate::owner::error)?;
            self.owner = Some(crate::owner::Owner::open(
                self.guard(),
                &shared.update(),
                self.assets.clone(),
            )?);
        }
        Ok(self.owner.as_mut().unwrap())
    }
    pub fn receipt(
        &mut self,
        peer: &str,
        update: Vec<u8>,
    ) -> Result<crate::owner::Receipt, String> {
        let history = self.ensure_owner()?.snapshot(peer)?;
        Ok(crate::owner::Receipt {
            revision: self.revision,
            update,
            history,
        })
    }
    pub fn record(&mut self, publication: &typstographe_core::content::shared::Publication) {
        if let Some(owner) = &self.owner {
            for resource in &publication.resources {
                self.assets.insert(
                    resource.path.clone(),
                    owner.assets()[&resource.path].clone(),
                );
            }
        }
        self.revision = publication.guard.revision;
        if publication.changed {
            self.bytes += publication.update.len();
            self.restored = None;
            self.settings = publication.state.settings.clone();
            self.updates.push(publication.update.clone());
            // Compact only the transport log. Never rebuild durable participant histories.
            let base_bytes = self.updates.first().map_or(0, Vec::len);
            if (self.updates.len() > 512
                || self.bytes > MAX_UPDATES
                || self.bytes.saturating_sub(base_bytes) > 4 * 1024 * 1024)
                && let Some(owner) = &self.owner
            {
                let checkpoint = owner.update_bytes();
                self.bytes = checkpoint.len();
                self.updates = vec![checkpoint];
            }
        }
    }
    #[cfg(test)]
    pub fn relay_group(
        &mut self,
        peer: &str,
        update: &[u8],
        group: Option<u64>,
        capture: bool,
    ) -> Result<crate::owner::Receipt, String> {
        self.relay_view(peer, update, group, capture, None)
    }
    pub fn relay_view(
        &mut self,
        peer: &str,
        update: &[u8],
        group: Option<u64>,
        capture: bool,
        view: Option<crate::owner::HistoryViewRequest>,
    ) -> Result<crate::owner::Receipt, String> {
        self.check_peer(peer, &self.generation)?;
        if update.is_empty() || update.len() > 4 * 1024 * 1024 {
            return Err(crate::owner::error(
                typstographe_core::content::contract::Error::StateLimitExceeded,
            ));
        }
        if self.updates.is_empty() {
            // The first complete seed contains body AND settings. Initialization is not an edit.
            let mut guard = self.guard();
            guard.revision = guard
                .revision
                .checked_add(1)
                .ok_or("Révision maximale atteinte.")?;
            self.owner = Some(crate::owner::Owner::open(
                guard.clone(),
                update,
                self.assets.clone(),
            )?);
            self.revision = guard.revision;
            self.bytes = update.len();
            self.updates.push(update.to_vec());
            return self.receipt(peer, update.to_vec());
        }
        let publication = self
            .ensure_owner()?
            .update(peer, update, group, capture, view)?;
        self.record(&publication);
        self.receipt(peer, publication.update)
    }
    pub fn history(
        &mut self,
        peer: &str,
        direction: typstographe_core::content::editing::Direction,
    ) -> Result<crate::owner::Receipt, String> {
        let publication = self.ensure_owner()?.history(peer, direction)?;
        self.record(&publication);
        self.receipt(peer, publication.update)
    }
    pub fn lifecycle(&self, revision: u64) -> Result<(), String> {
        if self.transition.as_ref().is_some_and(|t| {
            !t.ready() || t.revision != self.revision || t.generation != self.generation
        }) {
            return Err("recovery_transition_not_ready".into());
        }
        if self.synchronization().windows.len() > 1 && self.transition.is_none() {
            return Err("recovery_transition_required".into());
        }
        if self.revision != revision {
            return Err("Le document a changé ; réessayez après synchronisation.".into());
        }
        if !self.offline.is_empty() && self.transition.is_none() {
            return Err("Reconnectez les autres fenêtres avant de remplacer le document.".into());
        }
        Ok(())
    }
    pub fn reset_document_generation(&mut self) {
        self.offline.clear();
        self.draft_epochs.clear();
        self.recovered_name = None;
        self.recovered_items.clear();
        self.transition = None;
        self.generation = uuid::Uuid::new_v4().to_string();
        self.synchronization_enabled = false;
        self.synchronization_version = 0;
    }
    pub fn replace(
        &mut self,
        document: Node,
        settings: crate::settings::Settings,
        assets: Assets,
        yjs: Vec<u8>,
    ) {
        self.project = std::sync::OnceLock::new();
        self.owner = None;
        self.clean_revision = None;
        self.offline.clear();
        self.reset_document_generation();
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
    pub fn checkpoint(&self, snapshot: &Snapshot, update: Vec<u8>) -> Result<Checkpoint, String> {
        let source = self.validate(snapshot)?;
        if update.is_empty() || update.len() > MAX_UPDATES {
            return Err("État Yjs invalide.".into());
        }
        let verify = || {
            use typstographe_core::content::{
                contract::{Error, State},
                shared::SharedDocument,
            };
            let supplied = SharedDocument::from_update(&update)?;
            let confirmed = SharedDocument::from_updates(&self.updates)?;
            if !confirmed.same_history(&supplied) {
                return Err(Error::InconsistentState {
                    field: "checkpoint".into(),
                });
            }
            confirmed.verify(&supplied.state()?)?;
            supplied.verify(&State::new(
                snapshot.document.clone(),
                snapshot
                    .settings
                    .clone()
                    .unwrap_or_else(|| self.settings.clone()),
            ))
        };
        verify().map_err(|e| typstographe_i18n::content_error(&e))?;
        Ok(Checkpoint {
            project: self.project_recovery(
                typstographe_core::content::contract::State::new(
                    snapshot.document.clone(),
                    snapshot
                        .settings
                        .clone()
                        .unwrap_or_else(|| self.settings.clone()),
                ),
                update.clone(),
            )?,
            schema: SCHEMA.into(),
            document: snapshot.document.clone(),
            source,
            update,
            resources: self.resources(),
            settings: snapshot
                .settings
                .clone()
                .unwrap_or_else(|| self.settings.clone()),
        })
    }
    pub fn save(&self, snapshot: &Snapshot, update: Vec<u8>) -> Result<PathBuf, String> {
        let checkpoint = self.checkpoint(snapshot, update)?;
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
        if bytes.len() as u64 > crate::project::MAX_RECOVERY_BYTES {
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
    fn services_are_ephemeral_without_touching_the_document_or_its_owner() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::open(dir.path().into()).unwrap();
        let initial = s.join("main");
        assert!(!initial.synchronization.enabled);
        assert!(!s.should_broadcast("main"));
        assert!(s.should_broadcast("agent")); // Agent attachment isn't a human window.
        let before = (s.guard(), s.updates.clone(), s.assets.clone());
        let active = s
            .set_synchronization("main", &initial.generation, true)
            .unwrap();
        assert!(
            s.accepts_awareness("main", &initial.generation, active.version)
                .unwrap()
        );
        assert!(
            !s.accepts_awareness("main", &initial.generation, active.version - 1)
                .unwrap()
        );
        assert!(
            s.set_synchronization("unknown", &initial.generation, false)
                .is_err()
        );
        assert!(s.set_synchronization("main", "old", false).is_err());
        s.set_synchronization("main", &initial.generation, false)
            .unwrap();
        assert_eq!(s.guard(), before.0);
        assert_eq!(s.updates, before.1);
        assert_eq!(s.assets.len(), before.2.len());
        assert!(s.owner.is_none());
        assert!(
            !s.accepts_awareness("main", &initial.generation, active.version)
                .unwrap()
        );
        assert!(
            !Session::open(dir.path().into())
                .unwrap()
                .join("reopened")
                .synchronization
                .enabled
        );
    }
    #[test]
    fn a_second_window_activates_services_and_cannot_be_silently_forked() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::open(dir.path().into()).unwrap();
        let initial = s.join("main");
        assert!(s.join("peer").synchronization.enabled);
        assert!(s.should_broadcast("main"));
        assert_eq!(
            s.set_synchronization("main", &initial.generation, false)
                .unwrap_err(),
            "synchronization_other_windows"
        );
        s.connection("peer", &initial.generation, false).unwrap();
        assert!(!s.should_broadcast("main"));
        assert!(s.synchronization().enabled); // No automatic stop on departure.
        assert!(
            s.set_synchronization("peer", &initial.generation, false)
                .is_err()
        );
        s.set_synchronization("main", &initial.generation, false)
            .unwrap();
        s.connection("peer", &initial.generation, true).unwrap();
        assert!(s.synchronization().enabled);
        let old = s.synchronization();
        s.leave("peer");
        let after = s.synchronization();
        assert!(after.version > old.version);
        assert_eq!(after.windows, ["main"]);
        assert!(after.enabled);
        s.set_synchronization("main", &initial.generation, false)
            .unwrap();
    }
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
        s.relay("a", &update).unwrap();
        s.save(&snapshot, update.clone()).unwrap();
        let before = std::fs::read(dir.path().join("session.json")).unwrap();
        assert!(s.relay("b", &[3]).is_err());
        s.revision += 1; // Saving an old snapshot remains forbidden.
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
