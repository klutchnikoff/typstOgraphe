//! État et transactions d’un document ouvert, indépendants de l’interface.
use crate::{
    document::{Document, Projection},
    document_types::Metadata,
    storage::StoredDocument,
};
use std::{
    ops::Range,
    path::PathBuf,
    time::{Duration, Instant},
};

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
    pub document: Document,
    pub metadata: Metadata,
    pub template: String,
    pub selection: Selection,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditGroup {
    Body,
    Metadata(u64),
    Discrete,
}
pub struct Session {
    pub document: Document,
    pub metadata: Metadata,
    pub template: String,
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
            document,
            metadata: letter.metadata.clone(),
            template: letter.template.clone(),
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
            document: self.document.clone(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            selection,
        }
    }
    pub fn stored_document(&self) -> StoredDocument {
        StoredDocument {
            source: self.document.source().into(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            assets: self.document.assets.clone(),
        }
    }
    pub fn is_dirty(&self) -> bool {
        self.saved.source != self.document.source()
            || self.saved.metadata != self.metadata
            || self.saved.template != self.template
            || self.saved.assets != self.document.assets
    }
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
    pub fn remember(&mut self, before: Snapshot, selection: Selection, typing: bool) -> Selection {
        self.record(
            before,
            selection,
            if typing {
                EditGroup::Body
            } else {
                EditGroup::Discrete
            },
        )
    }
    pub fn record(
        &mut self,
        before: Snapshot,
        selection: Selection,
        group: EditGroup,
    ) -> Selection {
        if before.document == self.document
            && before.metadata == self.metadata
            && before.template == self.template
        {
            return selection;
        }
        let body_changed = before.document != self.document;
        if group == EditGroup::Discrete
            || !self.typing_group
            || self.last_group != group
            || self.last_edit.elapsed() > Duration::from_millis(650)
        {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.typing_group = group != EditGroup::Discrete;
        self.last_group = group;
        self.last_edit = Instant::now();
        if body_changed {
            self.changed(selection)
        } else {
            self.revision += 1;
            self.changed_at = Instant::now();
            selection
        }
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
        self.typing_group = false;
        self.revision += 1;
        self.changed_at = Instant::now();
        Some(item.selection)
    }
    pub fn edit(
        &mut self,
        mut selection: Selection,
        edit: impl FnOnce(&mut Projection, Range<usize>),
    ) -> Selection {
        for cursor in [&mut selection.primary, &mut selection.secondary] {
            cursor.index = cursor.index.min(self.document.view.len());
        }
        let before = self.snapshot(selection);
        let range = selection.range();
        edit(&mut self.document.view, range.clone());
        if !range.is_empty() {
            self.document.view.typing_style = self.document.view.style_at(range.start);
        }
        self.remember(before, selection, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_types::DocumentType;
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
        session.record(before, selection, EditGroup::Metadata(1));
        assert_eq!(session.document.source(), letter.source);
        let before = session.snapshot(selection);
        session.template.push_str("\n// Autre modèle");
        session.record(before, selection, EditGroup::Discrete);
        let before = session.snapshot(selection);
        session
            .document
            .view
            .insert_text("Suite.", session.document.view.len());
        session.remember(before, selection, true);
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
        session.record(session.snapshot(selection), selection, EditGroup::Discrete);
        assert!(session.undo.is_empty());
        for (field, value) in [(1, "A"), (1, "AB"), (2, "ABC")] {
            let before = session.snapshot(selection);
            session.metadata.courrier_mut().objet = value.into();
            session.record(before, selection, EditGroup::Metadata(field));
        }
        assert_eq!(session.undo.len(), 2);
        session.history(true, selection);
        assert_eq!(session.metadata.courrier().objet, "AB");
    }
}
