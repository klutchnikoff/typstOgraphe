//! No catalogue entry: only the human-facing adapter may approve a prepared lot.
use super::*;

impl Engine {
    /// Publish the whole candidate after explicit human approval. This method does
    /// not collect consent: the caller must do so, then discard the pending lot on
    /// success. Never expose it as an agent-callable command or MCP tool.
    ///
    /// Recheck freshness and write access even after review. All fallible work is
    /// staged before adoption. Preserve the primary selection and typing style;
    /// update only the proposing participant to its final candidate context.
    /// No file writes or implicit navigation are performed.
    pub fn accept_proposal(
        &mut self,
        context: &EditContext,
        proposal: &PreparedProposal,
    ) -> Result<Outcome, CommandError> {
        self.check_access("document.batch")?;
        self.check_proposal(context, proposal)?;
        let before = self.session.snapshot(self.selection);
        let mut candidate = proposal.candidate.clone();
        candidate.document.view.typing_style = before.document.view.typing_style.clone();
        let model_changed = self.session.template != candidate.template
            || self.session.dependencies != candidate.dependencies;
        let mut prepared: Engine = self.session.fork_for_batch().into();
        prepared.selection = prepared.session.commit_mapped_snapshot(
            proposal.base,
            before,
            candidate,
            proposal.positions.map_selection(self.selection),
            proposal.positions.edits.clone(),
        )?;
        let presentation = if model_changed {
            Some(prepared.option_policy()?)
        } else {
            None
        };
        let participant = ContextSnapshot {
            selection: prepared.selection_target(proposal.candidate.selection)?,
            typing_style: proposal.typing_style().clone(),
        };
        let outcome = Outcome {
            changed: prepared.session.revision != self.session.revision,
            guard: prepared.target_guard(),
            selection: Some(prepared.selection_target(prepared.selection)?),
            effects: if prepared.session.revision != self.session.revision {
                vec![Effect::Render]
            } else {
                vec![]
            },
            value: None,
        };
        // No validation, file IO or fallible operation after adoption.
        self.session.adopt_batch(prepared.session);
        self.selection = prepared.selection;
        self.presentation_cache = Default::default();
        if let Some(policy) = presentation {
            self.cache_presentation(policy);
        }
        self.refresh_contexts();
        self.contexts.set(context, participant);
        Ok(outcome)
    }
}
