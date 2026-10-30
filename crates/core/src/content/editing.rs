//! Participant-local editing state and undo. Nothing here is stored in the project.
//! Each participant keeps a durable Yrs replica: wire checkpoints do not encode
//! the internal redone links required by successive undo/redo operations.
mod anchors;
mod history;
mod selection;
pub use selection::{Selection, TextPoint};
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
use super::{
    commands::{Command, Style, Switch, TextTarget},
    contract::{Error, Guard, State, UnavailableReason},
    shared::{Publication, SharedDocument, SharedSession},
};
use crate::assets::Assets;
use selection::AnchoredSelection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use yrs::{ReadTxn, Transact, undo::UndoManager};

const MAX_CONTEXTS: usize = 64;
const ORIGIN: &str = "participant-edit";
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextId(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Undo,
    Redo,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextSnapshot {
    pub guard: Guard,
    pub selection: Option<Selection>,
    pub selection_lost: bool,
    pub typing_styles: Vec<Style>,
    pub undo_steps: usize,
    pub redo_steps: usize,
    pub pending: bool,
    pub availability: BTreeMap<String, super::availability::Availability>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RejectedDraft {
    pub base: Guard,
    pub state: Option<State>,
    pub update: Vec<u8>,
    pub error: Error,
}
struct Context {
    local: SharedDocument,
    history: UndoManager<Option<history::SavedView>>,
    pending_view: history::ViewSlot,
    restored_view: history::ViewSlot,
    owned_insertions: yrs::IdSet,
    confirmed_revision: u64,
    capture_group: Option<u64>,
    selection: Option<AnchoredSelection>,
    styles: Vec<Style>,
    rejected: Option<RejectedDraft>,
}
impl Context {
    fn open(owner: &SharedSession) -> Result<Self, Error> {
        let local = SharedDocument::from_update(&owner.update())?;
        // Grouping is explicit (capture_group), never time-based: a constant positive
        // clock keeps yrs merging until reset, without the system clock that is
        // unavailable on wasm32-unknown-unknown and may go backwards elsewhere.
        let mut history = UndoManager::with_options(yrs::undo::Options {
            capture_timeout_millis: u64::MAX,
            tracked_origins: Default::default(),
            capture_transaction: None,
            timestamp: std::sync::Arc::new(|| 1),
            init_undo_stack: Vec::new(),
            init_redo_stack: Vec::new(),
        });
        history.expand_scope(&local.doc, &local.doc.get_or_insert_xml_fragment("default"));
        history.expand_scope(&local.doc, &local.doc.get_or_insert_map("settings"));
        history.include_origin(ORIGIN);
        let pending_view = history::ViewSlot::default();
        let restored_view = history::ViewSlot::default();
        history::observe(&mut history, pending_view.clone(), restored_view.clone());
        Ok(Self {
            local,
            history,
            pending_view,
            restored_view,
            owned_insertions: yrs::IdSet::default(),
            confirmed_revision: owner.guard().revision,
            capture_group: None,
            selection: None,
            styles: vec![],
            rejected: None,
        })
    }
    fn sync(&mut self, owner: &SharedSession) -> Result<(), Error> {
        if self.rejected.is_some() {
            return Err(Error::EditingContext {
                reason: "pending_draft".into(),
            });
        }
        if self.confirmed_revision == owner.guard().revision {
            return Ok(());
        }
        super::shared::apply(&self.local.doc, &owner.difference(&self.local))?;
        self.confirmed_revision = owner.guard().revision;
        self.history.reset();
        self.capture_group = None;
        Ok(())
    }
    fn receive(&mut self, publication: &Publication) -> Result<(), Error> {
        self.receive_grouped(publication, None)
    }
    fn receive_grouped(
        &mut self,
        publication: &Publication,
        group: Option<u64>,
    ) -> Result<(), Error> {
        if !publication.changed {
            return Ok(());
        }
        let update =
            yrs::Update::decode_v1(&publication.update).map_err(|e| Error::InvalidSharedState {
                reason: e.to_string(),
            })?;
        self.owned_insertions.merge_with(update.insertions(true));
        if group.is_none() || self.capture_group != group {
            self.history.reset();
        }
        self.capture_view();
        self.local
            .doc
            .transact_mut_with(ORIGIN)
            .apply_update(update)
            .map_err(|e| Error::InvalidSharedState {
                reason: e.to_string(),
            })?;
        self.capture_group = group;
        if group.is_none() {
            self.history.reset();
        }
        self.confirmed_revision = publication.guard.revision;
        self.local.verify(&publication.state)
    }
}
use yrs::updates::decoder::Decode;

/// Owns confirmed publication and isolated participant replicas. The host owns
/// authentication, permissions, focus and IME; it serializes calls to this value.
pub struct EditingSession {
    owner: SharedSession,
    contexts: BTreeMap<ContextId, Context>,
    next_id: u64,
}
impl EditingSession {
    pub fn open(guard: Guard, update: &[u8], assets: Assets) -> Result<Self, Error> {
        Ok(Self {
            owner: SharedSession::open(guard, update, assets)?,
            contexts: BTreeMap::new(),
            next_id: 0,
        })
    }
    pub fn guard(&self) -> &Guard {
        self.owner.guard()
    }
    pub fn state(&self) -> &State {
        self.owner.state()
    }
    pub fn assets(&self) -> &Assets {
        self.owner.assets()
    }
    pub fn update(&self) -> Vec<u8> {
        self.owner.update()
    }
    fn check(&self, guard: &Guard, id: ContextId) -> Result<(), Error> {
        guard.check(self.guard())?;
        if !self.contexts.contains_key(&id) {
            return Err(Error::EditingContext {
                reason: "closed".into(),
            });
        }
        Ok(())
    }
    pub fn open_context(&mut self, guard: &Guard) -> Result<ContextId, Error> {
        guard.check(self.guard())?;
        if self.contexts.len() >= MAX_CONTEXTS {
            return Err(Error::StateLimitExceeded);
        }
        let next = self
            .next_id
            .checked_add(1)
            .ok_or(Error::StateLimitExceeded)?;
        let context = Context::open(&self.owner)?;
        self.next_id = next;
        let id = ContextId(next);
        self.contexts.insert(id, context);
        Ok(id)
    }
    /// Closing deliberately drops this participant's private state, including a
    /// rejected draft. Hosts must arrange any required user confirmation first.
    pub fn close_context(&mut self, guard: &Guard, id: ContextId) -> Result<(), Error> {
        self.check(guard, id)?;
        self.contexts.remove(&id);
        Ok(())
    }
    pub fn context(&mut self, guard: &Guard, id: ContextId) -> Result<ContextSnapshot, Error> {
        self.check(guard, id)?;
        let context = self.contexts.get_mut(&id).unwrap();
        if context.rejected.is_none() {
            context.sync(&self.owner)?;
        }
        let selection = if context.rejected.is_none() {
            context
                .selection
                .as_ref()
                .and_then(|s| s.resolve(&context.local, self.owner.state()).ok())
        } else {
            None
        };
        Ok(ContextSnapshot {
            guard: guard.clone(),
            selection_lost: context.selection.is_some() && selection.is_none(),
            availability: super::availability::for_selection(
                self.owner.state(),
                selection.as_ref(),
            ),
            selection,
            typing_styles: context.styles.clone(),
            undo_steps: context.history.undo_stack().len(),
            redo_steps: context.history.redo_stack().len(),
            pending: context.rejected.is_some(),
        })
    }
    pub fn select(
        &mut self,
        guard: &Guard,
        id: ContextId,
        target: Option<TextTarget>,
    ) -> Result<(), Error> {
        self.check(guard, id)?;
        if target.as_ref().is_some_and(|t| t.from > t.to) {
            return Err(super::shared::targets::invalid(
                super::contract::TargetReason::ReversedRange,
            ));
        }
        let context = self.contexts.get_mut(&id).unwrap();
        context.sync(&self.owner)?;
        let selection = target
            .as_ref()
            .map(|t| {
                AnchoredSelection::capture(
                    &context.local,
                    self.owner.state(),
                    &Selection::from(t.clone()),
                )
            })
            .transpose()?;
        context.selection = selection;
        Ok(())
    }
    /// Verify the causal checkpoint before accepting revision-relative view paths.
    pub fn check_frontier(&self, guard: &Guard, frontier: &[u8]) -> Result<(), Error> {
        guard.check(self.guard())?;
        self.owner.check_frontier(frontier)
    }
    /// Replace the participant's view state atomically. No publication, revision,
    /// undo entry or shared awareness is produced by navigation/stored marks.
    pub fn set_context(
        &mut self,
        guard: &Guard,
        id: ContextId,
        selection: Option<Selection>,
        styles: Vec<Style>,
    ) -> Result<ContextSnapshot, Error> {
        self.check(guard, id)?;
        super::shared::commands::validate_styles(&styles)?;
        let context = self.contexts.get_mut(&id).unwrap();
        context.sync(&self.owner)?;
        let anchored = selection
            .as_ref()
            .map(|s| AnchoredSelection::capture(&context.local, self.owner.state(), s))
            .transpose()?;
        if !styles.is_empty() {
            let target = selection
                .as_ref()
                .ok_or_else(|| Error::EditingContext {
                    reason: "selection_required".into(),
                })?
                .head_target()?;
            if !self
                .owner
                .inspect_zone(guard, &target.zone)?
                .supports_styles
            {
                return Err(Error::CommandUnavailable {
                    command: "typing.style".into(),
                    reason: UnavailableReason::PlainTextOnly,
                });
            }
        }
        context.selection = anchored;
        context.styles = styles;
        self.context(guard, id)
    }
    pub fn typing_style(
        &mut self,
        guard: &Guard,
        id: ContextId,
        style: Style,
        value: Switch,
    ) -> Result<(), Error> {
        self.check(guard, id)?;
        super::shared::commands::validate_styles(std::slice::from_ref(&style))?;
        let context = self.contexts.get_mut(&id).unwrap();
        context.sync(&self.owner)?;
        if let Some(selection) = &context.selection {
            let target = selection
                .resolve(&context.local, self.owner.state())?
                .head_target()?;
            let info = self.owner.inspect_zone(guard, &target.zone)?;
            if !info.supports_styles {
                return Err(Error::CommandUnavailable {
                    command: "typing.style".into(),
                    reason: UnavailableReason::PlainTextOnly,
                });
            }
        }
        context
            .styles
            .retain(|s| std::mem::discriminant(s) != std::mem::discriminant(&style));
        if value == Switch::On {
            context.styles.push(style);
        }
        Ok(())
    }
    /// A validated batch is captured in one local transaction after the owner's
    /// atomic publication. Navigation/style changes never enter its history.
    pub fn execute(
        &mut self,
        guard: &Guard,
        id: ContextId,
        commands: &[Command],
    ) -> Result<Publication, Error> {
        self.check(guard, id)?;
        self.contexts.get_mut(&id).unwrap().sync(&self.owner)?;
        let prepared = self.owner.prepare_commands(guard, commands)?;
        let publication = self.owner.publish(prepared)?;
        self.contexts.get_mut(&id).unwrap().receive(&publication)?;
        Ok(publication)
    }
    pub fn prepare_calls(
        &self,
        guard: &Guard,
        calls: &[super::api::Call],
    ) -> Result<super::shared::PreparedUpdate, Error> {
        self.owner
            .prepare_resolved(guard, calls, super::api::resolve_call)
    }
    pub fn prepare_resolved_calls(
        &self,
        guard: &Guard,
        calls: &[super::api::Call],
        resolve: impl FnMut(&State, &super::api::Call) -> Result<Command, Error>,
    ) -> Result<super::shared::PreparedUpdate, Error> {
        self.owner.prepare_resolved(guard, calls, resolve)
    }
    pub fn publish_prepared(
        &mut self,
        guard: &Guard,
        id: ContextId,
        prepared: super::shared::PreparedUpdate,
    ) -> Result<Publication, Error> {
        self.check(guard, id)?;
        self.contexts.get_mut(&id).unwrap().sync(&self.owner)?;
        let publication = self.owner.publish(prepared)?;
        self.contexts.get_mut(&id).unwrap().receive(&publication)?;
        Ok(publication)
    }
    pub fn availability(&self, guard: &Guard, command: &Command) -> super::commands::Availability {
        self.owner.availability(guard, command)
    }
    pub fn insert(
        &mut self,
        guard: &Guard,
        id: ContextId,
        text: String,
    ) -> Result<Publication, Error> {
        self.check(guard, id)?;
        let context = self.contexts.get_mut(&id).unwrap();
        context.sync(&self.owner)?;
        let target = context
            .selection
            .as_ref()
            .ok_or_else(|| Error::EditingContext {
                reason: "selection_required".into(),
            })?
            .resolve(&context.local, self.owner.state())?
            .text_target()?;
        let supports = self
            .owner
            .inspect_zone(guard, &target.zone)?
            .supports_styles;
        let marks = if supports {
            context.styles.clone()
        } else {
            vec![]
        };
        let offset = target
            .from
            .checked_add(text.encode_utf16().count() as u32)
            .ok_or(Error::StateLimitExceeded)?;
        let publication = self.execute(
            guard,
            id,
            &[Command::Insert {
                target: target.clone(),
                text,
                marks,
            }],
        )?;
        let context = self.contexts.get_mut(&id).unwrap();
        let zone = context
            .selection
            .as_ref()
            .unwrap()
            .text_zone(&context.local, self.owner.state())?;
        context.selection = Some(AnchoredSelection::capture(
            &context.local,
            self.owner.state(),
            &TextTarget {
                zone,
                from: offset,
                to: offset,
            }
            .into(),
        )?);
        Ok(publication)
    }
    pub fn execute_updates(
        &mut self,
        guard: &Guard,
        id: ContextId,
        updates: &[Vec<u8>],
    ) -> Result<Publication, Error> {
        self.check(guard, id)?;
        self.contexts.get_mut(&id).unwrap().sync(&self.owner)?;
        let prepared = self.owner.prepare(guard, updates)?;
        let publication = self.owner.publish(prepared)?;
        self.contexts.get_mut(&id).unwrap().receive(&publication)?;
        Ok(publication)
    }
    /// Group identifiers are private to the host/context. The host decides typing
    /// boundaries; commands and remote synchronization always end a capture.
    /// A refused update remains private and blocks this context until explicitly closed.
    pub fn execute_grouped_update(
        &mut self,
        guard: &Guard,
        id: ContextId,
        update: &[u8],
        group: u64,
    ) -> Result<Publication, Error> {
        self.check(guard, id)?;
        self.contexts.get_mut(&id).unwrap().sync(&self.owner)?;
        match self
            .owner
            .prepare(guard, &[update.to_vec()])
            .and_then(|p| self.owner.publish(p))
        {
            Ok(publication) => {
                self.contexts
                    .get_mut(&id)
                    .unwrap()
                    .receive_grouped(&publication, Some(group))?;
                Ok(publication)
            }
            Err(error) => {
                let proposed = [self.owner.update(), update.to_vec()];
                let state = SharedDocument::from_updates(&proposed)
                    .and_then(|d| d.state())
                    .ok();
                self.contexts.get_mut(&id).unwrap().rejected = Some(RejectedDraft {
                    base: guard.clone(),
                    state,
                    update: update.to_vec(),
                    error: error.clone(),
                });
                Err(error)
            }
        }
    }
    /// Incoming participant updates use the same final validation but never join
    /// another participant's undo stack. Rejected drafts are left untouched.
    pub fn publish_updates(
        &mut self,
        guard: &Guard,
        updates: &[Vec<u8>],
    ) -> Result<Publication, Error> {
        let prepared = self.owner.prepare(guard, updates)?;
        self.owner.publish(prepared)
    }
    pub fn rejected_draft(
        &self,
        guard: &Guard,
        id: ContextId,
    ) -> Result<Option<&RejectedDraft>, Error> {
        self.check(guard, id)?;
        Ok(self.contexts[&id].rejected.as_ref())
    }
    pub fn history(
        &mut self,
        guard: &Guard,
        id: ContextId,
        direction: Direction,
    ) -> Result<Publication, Error> {
        self.check(guard, id)?;
        let context = self.contexts.get_mut(&id).unwrap();
        context.sync(&self.owner)?;
        context.history.reset();
        context.capture_group = None;
        context.capture_view();
        *context.restored_view.lock().unwrap() = None;
        let before = context.local.doc.transact().snapshot();
        match direction {
            Direction::Undo => context.history.undo_blocking(),
            Direction::Redo => context.history.redo_blocking(),
        };
        let update = context
            .local
            .doc
            .transact()
            .encode_diff_v1(&before.state_map);
        let after = context.local.doc.transact().snapshot();
        // Undoing a container must not silently delete a later contributor's descendants.
        let generated = yrs::Update::decode_v1(&update).map_err(|e| Error::InvalidSharedState {
            reason: e.to_string(),
        })?;
        context
            .owned_insertions
            .merge_with(generated.insertions(true));
        let foreign_deletions = after
            .delete_set
            .diff(&before.delete_set)
            .diff(&context.owned_insertions);
        let candidate = if foreign_deletions.is_empty() {
            self.owner.prepare(guard, std::slice::from_ref(&update))
        } else {
            Err(Error::EditingContext {
                reason: "history_conflict".into(),
            })
        };
        match candidate.and_then(|prepared| self.owner.publish(prepared)) {
            Ok(publication) => {
                context.confirmed_revision = publication.guard.revision;
                context.local.verify(&publication.state)?;
                context.restore_view();
                Ok(publication)
            }
            Err(error) => {
                context.rejected = Some(RejectedDraft {
                    base: guard.clone(),
                    state: context.local.state().ok(),
                    update,
                    error: error.clone(),
                });
                Err(error)
            }
        }
    }
}
