//! Audit the recorded edits before publication; never infer positions from matching prose.
use super::*;
use crate::position_map::Splice;
impl Session {
    pub(super) fn prepare_positions(
        &self,
        before: &Snapshot,
        candidate: &Snapshot,
        mapped: Option<Vec<Splice>>,
    ) -> Result<(Vec<Splice>, Vec<Splice>), EditError> {
        let mut edits = mapped.unwrap_or_else(|| {
            self.document
                .view
                .trace
                .between(&candidate.document.view.trace)
                .unwrap_or_default()
        });
        // Mapping is checked against the actual publication, not a fabricated undo snapshot.
        if !crate::position_map::verifies(
            &self.document.view.text,
            &candidate.document.view.text,
            &edits,
        ) {
            return Err(EditError::InvalidProjection);
        }
        let undo_edits = before
            .document
            .view
            .trace
            .between(&candidate.document.view.trace)
            .unwrap_or_else(|| edits.clone());
        if !crate::position_map::verifies(
            &before.document.view.text,
            &candidate.document.view.text,
            &undo_edits,
        ) {
            return Err(EditError::InvalidProjection);
        }
        let inverse = crate::position_map::inverse(&undo_edits);
        if self.document.view.text == candidate.document.view.text {
            edits.clear();
        }
        Ok((edits, inverse))
    }
}
