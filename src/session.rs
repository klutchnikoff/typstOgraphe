//! État et transactions d’un document ouvert, indépendants de l’interface.
use crate::{
    document::{
        Document, Projection,
        edits::{EditError, TextInput},
        structure::{LogicalSelection, TargetError},
    },
    document_types::Metadata,
    storage::StoredDocument,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    ops::Range,
    path::PathBuf,
    sync::atomic::AtomicU64,
    time::{Duration, Instant},
};

#[path = "session/batches.rs"]
mod batches;
#[path = "session/edits.rs"]
mod edits;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "session/edits_tests.rs"]
mod edits_tests;

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct DocumentId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetGuard {
    pub document_id: DocumentId,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectionTarget {
    pub guard: TargetGuard,
    pub selection: LogicalSelection,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caret {
    pub index: usize,
    pub prefer_next_row: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub primary: Caret,
    pub secondary: Caret,
}
impl Selection {
    pub fn range(self) -> Range<usize> {
        self.primary.index.min(self.secondary.index)..self.primary.index.max(self.secondary.index)
    }
}
#[derive(Clone)]
pub struct Snapshot {
    guard: TargetGuard,
    pub document: Document,
    pub metadata: Metadata,
    pub template: String,
    pub dependencies: crate::packages::Dependencies,
    pub selection: Selection,
    logical_selection: Option<SelectionTarget>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditGroup {
    Body,
    Metadata(crate::document_types::MetadataField),
    Discrete,
}
pub struct Session {
    id: DocumentId,
    /// Une copie récupérée doit être enregistrée, même sans nouvelle édition.
    pub recovered: bool,
    pub document: Document,
    pub metadata: Metadata,
    pub template: String,
    pub dependencies: crate::packages::Dependencies,
    pub file_path: Option<PathBuf>,
    pub saved: StoredDocument,
    pub undo: Vec<Snapshot>,
    pub redo: Vec<Snapshot>,
    pub typing_group: bool,
    pub revision: u64,
    pub changed_at: Instant,
    last_edit: Instant,
    last_group: EditGroup,
}
impl Session {
    pub fn new(
        letter: StoredDocument,
        mut document: Document,
        path: Option<PathBuf>,
        revision: u64,
    ) -> Self {
        document.assets = letter.assets.clone();
        Self {
            id: DocumentId(crate::identity::next(&NEXT_SESSION)),
            recovered: false,
            document,
            metadata: letter.metadata.clone(),
            template: letter.template.clone(),
            dependencies: letter.dependencies.clone(),
            file_path: path,
            saved: letter,
            undo: vec![],
            redo: vec![],
            typing_group: false,
            revision,
            changed_at: Instant::now() - Duration::from_secs(1),
            last_edit: Instant::now(),
            last_group: EditGroup::Discrete,
        }
    }
    pub fn snapshot(&self, selection: Selection) -> Snapshot {
        Snapshot {
            guard: self.target_guard(),
            document: self.document.clone(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            dependencies: self.dependencies.clone(),
            selection,
            logical_selection: self.selection_target(selection).ok(),
        }
    }

    pub fn target_guard(&self) -> TargetGuard {
        TargetGuard {
            document_id: self.id,
            revision: self.revision,
        }
    }

    pub fn selection_target(&self, selection: Selection) -> Result<SelectionTarget, TargetError> {
        Ok(SelectionTarget {
            guard: self.target_guard(),
            selection: self
                .document
                .structure
                .selection_at(selection.secondary.index, selection.primary.index)?,
        })
    }

    pub fn resolve_target(&self, target: SelectionTarget) -> Result<(usize, usize), TargetError> {
        if target.guard.document_id != self.id {
            return Err(TargetError::ForeignDocument);
        }
        if target.guard.revision != self.revision {
            return Err(TargetError::StaleRevision);
        }
        self.document.structure.resolve_selection(target.selection)
    }
    pub fn stored_document(&self) -> StoredDocument {
        StoredDocument {
            source: self.document.source().into(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            dependencies: self.dependencies.clone(),
            assets: self.document.assets.clone(),
        }
    }
    pub fn is_dirty(&self) -> bool {
        self.recovered
            || self.saved.source != self.document.source()
            || self.saved.metadata != self.metadata
            || self.saved.template != self.template
            || self.saved.assets != self.document.assets
            || self.saved.dependencies != self.dependencies
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn changed(&mut self, mut selection: Selection) -> Selection {
        for at in self.document.sync_source() {
            for cursor in [&mut selection.primary, &mut selection.secondary] {
                if cursor.index > at {
                    cursor.index += 1;
                }
            }
        }
        for cursor in [&mut selection.primary, &mut selection.secondary] {
            cursor.index = cursor.index.min(self.document.view.len());
        }
        self.revision += 1;
        self.changed_at = Instant::now();
        selection
    }
    pub fn history(&mut self, undo: bool, selection: Selection) -> Option<Selection> {
        let current = self.snapshot(selection);
        let item = if undo {
            self.undo.pop()
        } else {
            self.redo.pop()
        }?;
        if undo {
            self.redo.push(current);
        } else {
            self.undo.push(current);
        }
        self.document = item.document;
        self.metadata = item.metadata;
        self.template = item.template;
        self.dependencies = item.dependencies;
        self.typing_group = false;
        self.revision += 1;
        self.changed_at = Instant::now();
        let mut selection = item.selection;
        if let Some(mut target) = item.logical_selection {
            // Restauration d’un instantané connu : nouvelle révision, mêmes objets.
            // Cette remise à jour interne n’est jamais appliquée à une cible externe.
            target.guard.revision = self.revision;
            if let Ok((anchor, head)) = self.resolve_target(target) {
                selection.primary.index = head;
                selection.secondary.index = anchor;
            }
        }
        Some(selection)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "session/targets_tests.rs"]
mod targets_tests;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::document_types::DocumentType;
    #[test]
    fn replacing_results_preserves_structures_and_starting_styles_with_one_undo() {
        let mut letter = StoredDocument::blank(DocumentType::Courrier);
        letter.source = "= *été*\n\nUn _été_ et un #strong[é]té.\n\n/ été: été\n\n#quote(block: true, attribution: [été])[été]\n\n#figure(image(\"images/parc.svg\"), caption: [été])".into();
        let document = Document::from_source(&letter.source).unwrap();
        let original = document.clone();
        let mut session = Session::new(letter, document, None, 1);
        let mut search = crate::search::Search::default();
        search.query = "été".into();
        search.refresh(session.revision, &session.document.view);
        assert_eq!(search.matches.len(), 8);
        let original_ranges = search.matches.clone();
        let old_selection = Selection {
            primary: Caret {
                index: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let after = session
            .replace_ranges(old_selection, &search.matches, "Noël")
            .unwrap();
        assert_eq!(session.undo.len(), 1);
        assert_eq!(session.document.view.blocks, original.view.blocks);
        assert_eq!(session.document.view.images, original.view.images);
        let mut expected = original.view.text.clone();
        for range in search.matches.iter().rev() {
            expected.replace_range(
                original.view.byte_index(range.start)..original.view.byte_index(range.end),
                "Noël",
            );
        }
        assert_eq!(session.document.view.text, expected);
        search.query = "Noël".into();
        search.refresh(session.revision, &session.document.view);
        for (range, old_range) in search.matches.iter().zip(original_ranges) {
            let style = original.view.style_at(old_range.start);
            assert!(
                session.document.view.styles[range.clone()]
                    .iter()
                    .all(|value| *value == style)
            );
        }
        assert_eq!(session.history(true, after), Some(old_selection));
        assert_eq!(session.document, original);
        assert!(!session.is_dirty());
        session.history(false, old_selection);
        assert_eq!(session.document.view.text, expected);
    }
    #[test]
    fn unified_history_restores_content_metadata_model_selection_and_clean_state() {
        let mut letter = StoredDocument::demo(DocumentType::Courrier);
        letter.source = "Un _été_.".into();
        let document = Document::from_source(&letter.source).unwrap();
        let mut session = Session::new(letter.clone(), document, None, 1);
        let selection = Selection {
            primary: Caret {
                index: 4,
                prefer_next_row: true,
            },
            secondary: Caret {
                index: 2,
                prefer_next_row: false,
            },
        };
        let before = session.snapshot(selection);
        session.metadata.courrier_mut().objet = "Nouvel objet".into();
        session
            .record(
                before,
                selection,
                EditGroup::Metadata(crate::document_types::MetadataField::LetterSubject),
            )
            .unwrap();
        assert_eq!(session.document.source(), letter.source);
        let before = session.snapshot(selection);
        session.template.push_str("\n// Autre modèle");
        session
            .record(before, selection, EditGroup::Discrete)
            .unwrap();
        let before = session.snapshot(selection);
        session
            .document
            .view
            .insert_text("Suite.", session.document.view.len());
        session.remember(before, selection, true).unwrap();
        assert_eq!(session.undo.len(), 3);
        for _ in 0..3 {
            assert_eq!(session.history(true, selection), Some(selection));
        }
        assert_eq!(session.stored_document(), letter);
        assert!(!session.is_dirty());
        for _ in 0..3 {
            assert_eq!(session.history(false, selection), Some(selection));
        }
        assert!(session.is_dirty());
        assert_eq!(session.metadata.courrier().objet, "Nouvel objet");
        assert!(session.template.ends_with("// Autre modèle"));
    }
    #[test]
    fn no_op_and_field_changes_do_not_merge_unrelated_transactions() {
        let letter = StoredDocument::demo(DocumentType::Courrier);
        let document = Document::from_source(&letter.source).unwrap();
        let mut session = Session::new(letter, document, None, 1);
        let selection = Selection::default();
        session
            .record(session.snapshot(selection), selection, EditGroup::Discrete)
            .unwrap();
        assert!(session.undo.is_empty());
        use crate::document_types::MetadataField::{LetterReference, LetterSubject};
        for (field, value) in [
            (LetterSubject, "A"),
            (LetterSubject, "AB"),
            (LetterReference, "ABC"),
        ] {
            let before = session.snapshot(selection);
            *session.metadata.field_mut(field).unwrap() = value.into();
            session
                .record(before, selection, EditGroup::Metadata(field))
                .unwrap();
        }
        assert_eq!(session.undo.len(), 2);
        session.history(true, selection);
        assert_eq!(session.metadata.courrier().objet, "AB");
    }
}
