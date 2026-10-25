//! Independent participant contexts; never switch the published engine to an agent.
mod queries;
use super::*;
use crate::document::InlineStyle;
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak, atomic::AtomicU64},
};
static NEXT_CONTEXT: AtomicU64 = AtomicU64::new(1);

/// Opaque, document-scoped lifetime. Dropping the last handle releases the participant.
/// Not a wire-format target or permission to edit the document.
#[derive(Clone, Debug)]
pub struct EditContext {
    pub(super) id: u64,
    document: crate::session::DocumentId,
    pub(super) lifetime: Arc<()>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextSnapshot {
    pub selection: SelectionTarget,
    pub typing_style: InlineStyle,
}
#[derive(Default)]
pub(super) struct Contexts {
    entries: BTreeMap<u64, Entry>,
}
struct Entry {
    lifetime: Weak<()>,
    state: ContextSnapshot,
}
impl Contexts {
    pub(super) fn set(&mut self, context: &EditContext, state: ContextSnapshot) {
        self.entries.insert(
            context.id,
            Entry {
                lifetime: Arc::downgrade(&context.lifetime),
                state,
            },
        );
    }
    pub fn advance(&mut self, guard: TargetGuard, map: Option<&PositionMap>) {
        self.entries.retain(|_, entry| {
            if entry.lifetime.strong_count() == 0
                || entry.state.selection.guard.document_id != guard.document_id
            {
                return false;
            }
            if entry.state.selection.guard == guard {
                return true;
            }
            let Some(map) = map.filter(|m| m.after == guard) else {
                return false;
            };
            let Ok(target) = map.map_retained(entry.state.selection) else {
                return false;
            };
            entry.state.selection = target;
            true
        });
    }
}
impl Engine {
    /// Start with a neutral caret, or an explicitly captured selection. Never follow
    /// the primary selection implicitly. No document revision/history/style change.
    pub fn create_edit_context(
        &mut self,
        guard: TargetGuard,
        selection: Option<SelectionTarget>,
    ) -> Result<EditContext, CommandError> {
        self.check_rule(AvailabilityRule::Read)?;
        self.check_guard(guard)?;
        let state = match selection {
            Some(target) => self.context_state_at(target)?,
            None => ContextSnapshot {
                selection: self.selection_target(Selection::default())?,
                typing_style: InlineStyle::default(),
            },
        };
        self.refresh_contexts();
        let context = EditContext {
            id: crate::identity::next(&NEXT_CONTEXT),
            document: guard.document_id,
            lifetime: Arc::new(()),
        };
        self.contexts.set(&context, state);
        Ok(context)
    }
    pub fn primary_context(&self) -> Option<ContextSnapshot> {
        self.open.then(|| ContextSnapshot {
            selection: self
                .selection_target(self.selection)
                .expect("published selection"),
            typing_style: self.document().view.typing_style.clone(),
        })
    }
    pub fn editing_context(&self, context: &EditContext) -> Result<ContextSnapshot, CommandError> {
        self.check_context(context)?;
        Ok(self.contexts.entries[&context.id].state.clone())
    }
    pub fn select_in_context(
        &mut self,
        context: &EditContext,
        selection: SelectionTarget,
    ) -> Result<ContextSnapshot, CommandError> {
        self.check_context(context)?;
        let state = self.context_state_at(selection)?;
        self.contexts
            .entries
            .get_mut(&context.id)
            .expect("checked context")
            .state = state.clone();
        Ok(state)
    }
    /// Navigate a participant without replacing the primary editor's selection/style.
    pub fn navigate_context(
        &mut self,
        context: &EditContext,
        mut params: SelectTextParams,
    ) -> Result<ContextSnapshot, CommandError> {
        let current = self.editing_context(context)?;
        if matches!(params.target, TextTarget::Current {}) {
            params.target = current.selection.into();
        }
        self.check_rule(AvailabilityRule::Read)?;
        let selection = if let Some(part) = params.part {
            self.relative_destination(params.target, part)?
        } else {
            self.resolve(params.target)?
        };
        let target = self.selection_target(selection)?;
        self.select_in_context(context, target)
    }
    fn context_state_at(&self, target: SelectionTarget) -> Result<ContextSnapshot, CommandError> {
        let selection = self.resolve(target.into())?;
        let range = selection.range();
        Ok(ContextSnapshot {
            selection: target,
            typing_style: if !range.is_empty() {
                self.document().view.style_at(range.start)
            } else if selection.primary.index > 0 {
                self.document().view.style_at(selection.primary.index - 1)
            } else {
                InlineStyle::default()
            },
        })
    }
    pub(super) fn check_context(&self, context: &EditContext) -> Result<(), CommandError> {
        if !self.open || context.document != self.target_guard().document_id {
            return Err(CommandError::new(ErrorCode::Document, "foreign_document"));
        }
        let entry = self
            .contexts
            .entries
            .get(&context.id)
            .filter(|e| e.lifetime.ptr_eq(&Arc::downgrade(&context.lifetime)))
            .ok_or_else(|| CommandError::new(ErrorCode::Unavailable, "edit_context_closed"))?;
        self.check_guard(entry.state.selection.guard)
    }
    pub(super) fn refresh_contexts(&mut self) {
        if !self.open {
            self.contexts.entries.clear();
        } else {
            self.contexts
                .advance(self.target_guard(), self.session.last_positions.as_ref());
        }
    }
}
