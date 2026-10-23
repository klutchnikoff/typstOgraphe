//! Préparation isolée puis transfert sans validation faillible après une écriture disque.
use super::*;

impl Session {
    /// Même identité et même révision initiales, mais aucun historique à recopier.
    /// Les cibles explicites suivent ainsi les mêmes règles que des appels successifs.
    pub(crate) fn fork_for_batch(&self) -> Self {
        Self {
            id: self.id,
            recovered: self.recovered,
            document: self.document.clone(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            dependencies: self.dependencies.clone(),
            file_path: self.file_path.clone(),
            saved: self.saved.clone(),
            undo: vec![],
            redo: vec![],
            typing_group: false,
            revision: self.revision,
            changed_at: self.changed_at,
            last_edit: self.last_edit,
            last_group: EditGroup::Discrete,
        }
    }

    /// Le candidat a déjà passé commit_snapshot et, éventuellement, l’écriture
    /// atomique finale. Cette publication ne fait plus d’IO ni de validation.
    pub(crate) fn adopt_batch(&mut self, mut prepared: Self) {
        debug_assert_eq!(self.id, prepared.id);
        debug_assert!(prepared.revision == self.revision || prepared.revision == self.revision + 1);
        if prepared.revision != self.revision {
            debug_assert_eq!(prepared.undo.len(), 1);
            self.undo.append(&mut prepared.undo);
            if self.undo.len() > 100 {
                self.undo.drain(..self.undo.len() - 100);
            }
            self.redo.clear();
        }
        prepared.undo = std::mem::take(&mut self.undo);
        prepared.redo = std::mem::take(&mut self.redo);
        prepared.typing_group = false;
        *self = prepared;
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::document_types::DocumentType;

    fn session() -> Session {
        let mut stored = StoredDocument::blank(DocumentType::Courrier);
        stored.source = "Avant".into();
        Session::new(stored, Document::from_source("Avant").unwrap(), None, 4)
    }
    fn insert(session: &mut Session, selection: Selection, text: &str) -> Selection {
        session
            .replace_text(session.selection_target(selection).unwrap(), text)
            .unwrap()
    }
    #[test]
    fn staging_keeps_identity_without_sharing_edits_or_history_and_adoption_is_one_transaction() {
        let mut original = session();
        let initial = original.stored_document();
        let selection = insert(&mut original, Selection::default(), "Un ");
        let before = original.snapshot(selection);
        let guard = original.target_guard();
        let mut draft = original.fork_for_batch();
        assert_eq!(draft.target_guard(), guard);
        assert!(draft.undo.is_empty() && draft.redo.is_empty());
        let after = insert(&mut draft, selection, "deux ");
        assert_eq!(original.stored_document().source, "Un Avant");
        let mut prepared = original.fork_for_batch();
        let mut candidate = before.clone();
        candidate.document = draft.document;
        let published = prepared
            .commit_snapshot(guard, before, candidate, after, EditGroup::Discrete)
            .unwrap();
        original.adopt_batch(prepared);
        assert_eq!(original.revision, guard.revision + 1);
        assert_eq!(original.undo.len(), 2);
        assert_eq!(original.document.source(), "Un deux Avant");
        let restored = original.history(true, published).unwrap();
        assert_eq!(restored, selection);
        assert_eq!(original.document.source(), "Un Avant");
        original.history(true, restored).unwrap();
        assert_eq!(original.stored_document(), initial);
    }
    #[test]
    fn adopting_selection_only_preserves_redo_and_saved_state() {
        let mut original = session();
        let after = insert(&mut original, Selection::default(), "Un ");
        original.history(true, after).unwrap();
        let guard = original.target_guard();
        let saved = original.saved.clone();
        let mut prepared = original.fork_for_batch();
        prepared.document.view.typing_style.bold = true;
        original.adopt_batch(prepared);
        assert_eq!(original.target_guard(), guard);
        assert_eq!(original.saved, saved);
        assert_eq!(original.redo.len(), 1);
        assert!(original.document.view.typing_style.bold);
        original.history(false, Selection::default()).unwrap();
        assert_eq!(original.document.source(), "Un Avant");
    }
}
