//! Isolated candidates and guarded publication by a trusted human-facing adapter.
mod acceptance;
use super::*;
use crate::{
    document::InlineStyle, document::structure::LogicalSelection, storage::StoredDocument,
};
use std::sync::{Arc, Weak};

/// An isolated candidate. Its logical positions describe the candidate, never a
/// reference into the published document. Dropping it is a side-effect-free refusal.
pub struct PreparedProposal {
    base: TargetGuard,
    context: EditContextIdentity,
    document: StoredDocument,
    // Preserve block/zone identities and resources for the approval path;
    // reparsing the serialized source at acceptance would lose those identities.
    candidate: Snapshot,
    positions: PositionMap,
}
struct EditContextIdentity {
    id: u64,
    lifetime: Weak<()>,
}
impl PreparedProposal {
    pub fn base_guard(&self) -> TargetGuard {
        self.base
    }
    pub fn document(&self) -> &StoredDocument {
        &self.document
    }
    /// Read-only structured candidate, for a human-facing review without reparsing.
    pub fn candidate_document(&self) -> &crate::document::Document {
        &self.candidate.document
    }
    pub fn selection(&self) -> LogicalSelection {
        self.candidate
            .document
            .structure
            .selection_at(
                self.candidate.selection.secondary.index,
                self.candidate.selection.primary.index,
            )
            .expect("validated candidate selection")
    }
    pub fn typing_style(&self) -> &InlineStyle {
        &self.candidate.document.view.typing_style
    }
    /// For review only; no speculative revision or guarded candidate target escapes.
    pub fn map_for_review(
        &self,
        target: SelectionTarget,
    ) -> Result<LogicalSelection, CommandError> {
        Ok(self.positions.map_retained(target)?.selection)
    }
}
impl Engine {
    /// Prepare the entire lot on a private context. The live document, user and
    /// participant contexts, undo history and files remain unchanged on success too.
    pub fn prepare_proposal(
        &self,
        context: &EditContext,
        p: BatchParams,
    ) -> Result<PreparedProposal, CommandError> {
        self.check_proposal_header(context, &p)?;
        let mut commands = self.check_batch(&p)?.into_iter();
        self.stage_proposal(context, &p, |_, _, _| {
            Ok(ResolvedBatchStep {
                command: commands.next().expect("checked steps"),
                selection: None,
            })
        })
    }
    /// Same bounded resolver contract as run_resolved_batch: resolve against the
    /// evolving private candidate, perform no writes and preserve declared IDs.
    pub fn prepare_resolved_proposal(
        &self,
        context: &EditContext,
        p: BatchParams,
        mut resolve: impl FnMut(usize, &Engine, &Call) -> Result<ResolvedBatchStep, CommandError>,
    ) -> Result<PreparedProposal, CommandError> {
        self.check_proposal_header(context, &p)?;
        let mut budget = batch::ByteLimit(MAX_BATCH_BYTES);
        self.stage_proposal(context, &p, |index, draft, call| {
            let resolved = resolve(index, draft, call)?;
            serde_json::to_writer(&mut budget, &resolved.command)
                .and_then(|_| serde_json::to_writer(&mut budget, &resolved.selection))
                .map_err(|_| CommandError::new(ErrorCode::InvalidParams, "batch_size_limit"))?;
            Ok(resolved)
        })
    }
    /// Freshness check only, not human consent and not publication. Any intervening
    /// document edit invalidates the whole proposal, even if its context was mapped.
    pub fn check_proposal(
        &self,
        context: &EditContext,
        proposal: &PreparedProposal,
    ) -> Result<(), CommandError> {
        self.check_context(context)?;
        self.check_guard(proposal.base)?;
        if proposal.context.id != context.id
            || !proposal
                .context
                .lifetime
                .ptr_eq(&Arc::downgrade(&context.lifetime))
        {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "edit_context_closed",
            ));
        }
        Ok(())
    }
    fn check_proposal_header(
        &self,
        context: &EditContext,
        p: &BatchParams,
    ) -> Result<(), CommandError> {
        self.check_context(context)?;
        if p.save.is_some() {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "proposal_files_forbidden",
            ));
        }
        self.check_batch_header(p)?;
        self.check_batch_ids(p)
    }
    fn stage_proposal(
        &self,
        context: &EditContext,
        p: &BatchParams,
        resolve: impl FnMut(usize, &Engine, &Call) -> Result<ResolvedBatchStep, CommandError>,
    ) -> Result<PreparedProposal, CommandError> {
        let state = self.editing_context(context)?;
        let (draft, _, edits) = self.stage_batch(p, Some(&state), resolve)?;
        let positions = PositionMap::new(
            p.guard,
            draft.target_guard(),
            self.document().structure.position_index(),
            draft.document().structure.position_index(),
            edits,
        );
        let document = draft.stored_document();
        let mut candidate = self.session.snapshot(draft.selection);
        candidate.document = draft.session.document;
        candidate.metadata = draft.session.metadata;
        candidate.template = draft.session.template;
        candidate.dependencies = draft.session.dependencies;
        Ok(PreparedProposal {
            base: p.guard,
            context: EditContextIdentity {
                id: context.id,
                lifetime: Arc::downgrade(&context.lifetime),
            },
            candidate,
            document,
            positions,
        })
    }
}
