//! Yjs-compatible state owned by Rust. Shared types never cross this boundary.
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod command_tests;
pub(super) mod commands;
mod normalization;
mod objects;
pub(super) mod projection;
mod records;
mod resources;
mod seed;
mod settings;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod settings_tests;
pub(super) mod structure;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod structure_tests;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod table_tests;
mod tables;
pub(super) mod targets;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
mod whitespace;

use super::contract::{Error, Guard, State};
use crate::assets::Assets;
use yrs::updates::decoder::Decode;
use yrs::{Doc, OffsetKind, Options, ReadTxn, StateVector, Transact, Update};

pub const MAX_STATE_BYTES: usize = 32 * 1024 * 1024;

pub struct SharedDocument {
    pub(super) doc: Doc,
}
fn document() -> Doc {
    // Preserve deleted content while undo and snapshot retention are being coordinated.
    let options = Options {
        offset_kind: OffsetKind::Utf16,
        skip_gc: true,
        ..Options::default()
    };
    let doc = Doc::with_options(options);
    doc.get_or_insert_xml_fragment("default");
    doc.get_or_insert_map("settings");
    doc
}
pub(super) fn apply(doc: &Doc, bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() || bytes.len() > MAX_STATE_BYTES {
        return Err(Error::StateLimitExceeded);
    }
    let update = Update::decode_v1(bytes).map_err(|e| Error::InvalidSharedState {
        reason: e.to_string(),
    })?;
    doc.transact_mut()
        .apply_update(update)
        .map_err(|e| Error::InvalidSharedState {
            reason: e.to_string(),
        })
}
fn complete(doc: &Doc) -> Result<(), Error> {
    let tx = doc.transact();
    if tx.store().pending_update().is_some() || tx.store().pending_ds().is_some() {
        Err(Error::IncompleteSharedState)
    } else {
        Ok(())
    }
}
impl SharedDocument {
    /// A saved state must be self-contained; pending differential dependencies are refused.
    pub fn from_update(bytes: &[u8]) -> Result<Self, Error> {
        let doc = document();
        apply(&doc, bytes)?;
        complete(&doc)?;
        Ok(Self { doc })
    }
    /// Reassemble a confirmed relay log before projecting it; intermediate updates
    /// may depend on later entries or lack settings during bootstrap.
    pub fn from_updates(updates: &[Vec<u8>]) -> Result<Self, Error> {
        let total = updates
            .iter()
            .try_fold(0usize, |n, u| n.checked_add(u.len()))
            .ok_or(Error::StateLimitExceeded)?;
        if total > MAX_STATE_BYTES {
            return Err(Error::StateLimitExceeded);
        }
        let doc = document();
        for bytes in updates {
            apply(&doc, bytes)?;
        }
        complete(&doc)?;
        Ok(Self { doc })
    }
    pub fn same_history(&self, other: &Self) -> bool {
        self.doc.transact().snapshot() == other.doc.transact().snapshot()
    }
    /// Bootstrap a new project once. Editing an existing state must preserve its identities.
    pub fn from_state(state: &State) -> Result<Self, Error> {
        if state.version != super::contract::VERSION {
            return Err(Error::UnsupportedContractVersion {
                version: state.version,
            });
        }
        state.settings.validate()?;
        if state.document.kind != "doc"
            || state.document.text.is_some()
            || !state.document.marks.is_empty()
        {
            return Err(Error::InvalidDocument {
                reason: "invalid_root".into(),
            });
        }
        let doc = document();
        seed::write(&doc, state)?;
        Ok(Self { doc })
    }
    pub fn state(&self) -> Result<State, Error> {
        projection::read(&self.doc)
    }
    pub fn update(&self) -> Vec<u8> {
        self.doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }
    /// Compact causal checkpoint, compatible with Yjs encodeSnapshot. Contains
    /// clocks and deletion intervals, never the document's text or resources.
    pub fn frontier(&self) -> Vec<u8> {
        use yrs::updates::encoder::Encode;
        self.doc.transact().snapshot().encode_v1()
    }
    /// Compare projections, allowing only documented editor representation defaults.
    pub fn verify(&self, expected: &State) -> Result<(), Error> {
        if expected.version != super::contract::VERSION {
            return Err(Error::UnsupportedContractVersion {
                version: expected.version,
            });
        }
        let actual = self.state()?;
        if normalization::node(&actual.document) != normalization::node(&expected.document) {
            return Err(Error::InconsistentState {
                field: "document".into(),
            });
        }
        if normalization::settings(&actual.settings) != normalization::settings(&expected.settings)
        {
            return Err(Error::InconsistentState {
                field: "settings".into(),
            });
        }
        Ok(())
    }
}

/// Empty bytes mean a new project not yet initialized by an editor, never a second state.
pub fn verify_or_initialize(state: &State, bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let shared = if bytes.is_empty() {
        SharedDocument::from_state(state)?
    } else {
        SharedDocument::from_update(bytes)?
    };
    shared.verify(state)?;
    let update = shared.update();
    if update.len() > MAX_STATE_BYTES {
        return Err(Error::StateLimitExceeded);
    }
    Ok(update)
}

/// A private candidate cannot be changed by an adapter after validation.
pub struct PreparedUpdate {
    guard: Guard,
    candidate: SharedDocument,
    state: State,
    update: Vec<u8>,
    changed: bool,
    assets: Option<Assets>,
    resources: Vec<super::commands::ImageResource>,
}

impl PreparedUpdate {
    pub fn changed(&self) -> bool {
        self.changed
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn checkpoint(&self) -> Vec<u8> {
        self.candidate.update()
    }
    pub fn assets<'a>(&'a self, original: &'a Assets) -> &'a Assets {
        self.assets.as_ref().unwrap_or(original)
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Publication {
    pub resources: Vec<super::commands::ImageResource>,
    pub guard: Guard,
    pub changed: bool,
    pub state: State,
    pub update: Vec<u8>,
}

/// Confirmed documentary state. Host-specific focus, participants and transports live elsewhere.
pub struct SharedSession {
    guard: Guard,
    shared: SharedDocument,
    state: State,
    assets: Assets,
}
impl SharedSession {
    pub fn open(guard: Guard, bytes: &[u8], assets: Assets) -> Result<Self, Error> {
        if guard.document.is_empty() {
            return Err(Error::InvalidDocument {
                reason: "empty_document_identity".into(),
            });
        }
        let shared = SharedDocument::from_update(bytes)?;
        let state = shared.state()?;
        structure::validate(&state)?;
        state.validate(&assets)?;
        Ok(Self {
            guard,
            shared,
            state,
            assets,
        })
    }
    pub fn guard(&self) -> &Guard {
        &self.guard
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn update(&self) -> Vec<u8> {
        self.shared.update()
    }
    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub(in crate::content) fn difference(&self, other: &SharedDocument) -> Vec<u8> {
        self.shared
            .doc
            .transact()
            .encode_diff_v1(&other.doc.transact().state_vector())
    }

    /// Compact Yjs/Yrs causal frontier, including deletions. A state vector alone
    /// cannot distinguish two checkpoints separated only by a deletion.
    pub(in crate::content) fn check_frontier(&self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > MAX_STATE_BYTES {
            return Err(Error::StateLimitExceeded);
        }
        let supplied = yrs::Snapshot::decode_v1(bytes).map_err(|e| Error::InvalidSharedState {
            reason: e.to_string(),
        })?;
        if supplied != self.shared.doc.transact().snapshot() {
            return Err(Error::InconsistentState {
                field: "checkpoint".into(),
            });
        }
        Ok(())
    }

    pub fn inspect_zone(
        &self,
        guard: &Guard,
        zone: &super::commands::TextZone,
    ) -> Result<super::commands::ZoneInfo, Error> {
        guard.check(&self.guard)?;
        targets::resolve(&self.shared.doc, &self.state, zone)?.info(&self.shared.doc.transact())
    }

    /// Typed commands share the same guarded publication as incoming shared updates.
    /// A batch sees its previous steps; only its final candidate is published.
    pub fn prepare_commands(
        &self,
        guard: &Guard,
        commands: &[super::commands::Command],
    ) -> Result<PreparedUpdate, Error> {
        guard.check(&self.guard)?;
        if commands.len() > super::commands::MAX_BATCH_STEPS {
            return Err(Error::StateLimitExceeded);
        }
        let mut size = 0usize;
        for command in commands {
            size = size
                .checked_add(
                    serde_json::to_vec(command)
                        .expect("command serialization")
                        .len(),
                )
                .ok_or(Error::StateLimitExceeded)?;
            if size > MAX_STATE_BYTES {
                return Err(Error::StateLimitExceeded);
            }
        }
        self.prepare_resolved(guard, commands, |_, command| Ok(command.clone()))
    }
    /// Resolve convenience selectors on the evolving isolated draft. Resolution
    /// never executes a second editing implementation; only the final candidate validates.
    pub fn prepare_resolved<T>(
        &self,
        guard: &Guard,
        steps: &[T],
        mut resolve: impl FnMut(&State, &T) -> Result<super::commands::Command, Error>,
    ) -> Result<PreparedUpdate, Error> {
        guard.check(self.guard())?;
        if steps.len() > super::commands::MAX_BATCH_STEPS {
            return Err(Error::StateLimitExceeded);
        }
        let candidate = SharedDocument::from_update(&self.shared.update())?;
        let mut assets = None;
        let mut bytes = 0usize;
        for (index, step) in steps.iter().enumerate() {
            let result = (|| {
                let state = candidate.state()?;
                let command = resolve(&state, step)?;
                bytes += serde_json::to_vec(&command).expect("command").len();
                if bytes > MAX_STATE_BYTES {
                    return Err(Error::StateLimitExceeded);
                }
                if let super::commands::Command::AddResource { resource } = &command {
                    resources::add(assets.get_or_insert_with(|| self.assets.clone()), resource)
                } else {
                    commands::apply(&candidate.doc, &state, &command)
                }
            })();
            result.map_err(|cause| Error::BatchFailed {
                index,
                cause: Box::new(cause),
            })?;
        }
        whitespace::apply(&candidate.doc, &candidate.state()?)?;
        self.finish_preparation_with_assets(guard, candidate, assets)
    }

    /// Same resolver, parameter validation and final validation as execution.
    /// This is a query; no candidate can be published and no revision is reserved.
    pub fn availability(
        &self,
        guard: &Guard,
        command: &super::commands::Command,
    ) -> super::commands::Availability {
        let reason = self
            .prepare_commands(guard, std::slice::from_ref(command))
            .err()
            .map(|error| {
                if let Error::BatchFailed { cause, .. } = error {
                    *cause
                } else {
                    error
                }
            });
        super::commands::Availability {
            enabled: reason.is_none(),
            reason,
        }
    }

    /// Validate only the final batch, permitting intermediate private states.
    /// The original shared document is never mutated by a failed or stale preparation.
    pub fn prepare(&self, guard: &Guard, updates: &[Vec<u8>]) -> Result<PreparedUpdate, Error> {
        guard.check(&self.guard)?;
        let total = updates
            .iter()
            .try_fold(0usize, |n, u| n.checked_add(u.len()))
            .ok_or(Error::StateLimitExceeded)?;
        if total > MAX_STATE_BYTES {
            return Err(Error::StateLimitExceeded);
        }
        let original = self.shared.update();
        let candidate = SharedDocument::from_update(&original)?;
        for bytes in updates {
            apply(&candidate.doc, bytes)?;
        }
        self.finish_preparation(guard, candidate)
    }

    fn finish_preparation(
        &self,
        guard: &Guard,
        candidate: SharedDocument,
    ) -> Result<PreparedUpdate, Error> {
        self.finish_preparation_with_assets(guard, candidate, None)
    }
    fn finish_preparation_with_assets(
        &self,
        guard: &Guard,
        candidate: SharedDocument,
        assets: Option<Assets>,
    ) -> Result<PreparedUpdate, Error> {
        // The common typing path borrows the immutable pool; clone its index only for an import.
        let validation_assets = assets.as_ref().unwrap_or(&self.assets);
        let before = self.shared.doc.transact().state_vector();
        let before_snapshot = self.shared.doc.transact().snapshot();
        complete(&candidate.doc)?;
        let full = candidate.update();
        if full.len() > MAX_STATE_BYTES {
            return Err(Error::StateLimitExceeded);
        }
        let state = candidate.state()?;
        structure::validate(&state)?;
        state.validate(validation_assets)?;
        if state.settings.metadata.document_type() != self.state.settings.metadata.document_type() {
            return Err(Error::InvalidDocument {
                reason: "document_type_changed".into(),
            });
        }
        if state.settings.template != self.state.settings.template
            || state.settings.model_id != self.state.settings.model_id
        {
            settings::validate_model(&state, validation_assets)?;
        }
        let update = candidate.doc.transact().encode_diff_v1(&before);
        let resources: Vec<_> = assets
            .as_ref()
            .into_iter()
            .flat_map(|assets| assets.iter())
            .filter(|(path, _)| !self.assets.contains_key(*path))
            .map(|(path, data)| super::commands::ImageResource {
                path: path.clone(),
                data: data.to_vec(),
            })
            .collect();
        let changed =
            before_snapshot != candidate.doc.transact().snapshot() || !resources.is_empty();
        Ok(PreparedUpdate {
            guard: guard.clone(),
            assets,
            resources,
            changed,
            state,
            candidate,
            update,
        })
    }
    /// Publication is synchronous under the host's session lock; no file IO or callbacks.
    pub fn publish(&mut self, prepared: PreparedUpdate) -> Result<Publication, Error> {
        prepared.guard.check(&self.guard)?;
        if prepared.changed {
            let revision = self
                .guard
                .revision
                .checked_add(1)
                .ok_or(Error::StateLimitExceeded)?;
            if let Some(assets) = prepared.assets {
                self.assets = assets;
            }
            self.shared = prepared.candidate;
            self.state = prepared.state;
            self.guard.revision = revision;
        }
        Ok(Publication {
            resources: prepared.resources,
            guard: self.guard.clone(),
            changed: prepared.changed,
            state: self.state.clone(),
            update: if prepared.changed {
                prepared.update
            } else {
                vec![]
            },
        })
    }
}
