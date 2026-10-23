//! Lectures immuables de la session possédée par le moteur, sans accès au conteneur.
use super::*;
use crate::{document::Document, document_types::Metadata};
use std::{path::Path, time::Instant};

impl Engine {
    pub fn document(&self) -> &Document {
        &self.session.document
    }
    pub fn metadata(&self) -> &Metadata {
        &self.session.metadata
    }
    pub fn template(&self) -> &String {
        &self.session.template
    }
    pub fn dependencies(&self) -> &crate::packages::Dependencies {
        &self.session.dependencies
    }
    pub fn file_path(&self) -> Option<&Path> {
        self.session.file_path.as_deref()
    }
    pub fn revision(&self) -> u64 {
        self.session.revision
    }
    pub fn changed_at(&self) -> Instant {
        self.session.changed_at
    }
    pub fn is_recovered(&self) -> bool {
        self.session.recovered
    }
    pub fn is_dirty(&self) -> bool {
        self.open && self.session.is_dirty()
    }
    pub fn can_undo(&self) -> bool {
        !self.session.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.session.redo.is_empty()
    }
    pub fn target_guard(&self) -> TargetGuard {
        self.session.target_guard()
    }
    pub fn selection_target(
        &self,
        selection: Selection,
    ) -> Result<SelectionTarget, crate::document::structure::TargetError> {
        self.session.selection_target(selection)
    }
    pub fn resolve_target(
        &self,
        target: SelectionTarget,
    ) -> Result<(usize, usize), crate::document::structure::TargetError> {
        self.session.resolve_target(target)
    }
    pub fn stored_document(&self) -> crate::storage::StoredDocument {
        self.session.stored_document()
    }
}
