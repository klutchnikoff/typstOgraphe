//! Coordination source ↔ aperçu par vue, séparée du moteur documentaire.
//!
//! L'adaptateur publie des faits ordonnés puis consomme les actions au moment
//! où il peut les peindre. Aucun focus, pixel, IME, délai, worker ou accès disque.
//! Les lectures géométriques empruntent le moteur sans modifier sa sélection.
use crate::engine::{RenderReference, SelectionTarget, TargetGuard};

mod geometry;
mod types;
pub use types::*;

/// Une instance par vue ; ce composant ne crée pas de contexte d'édition moteur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewSync {
    selection: Option<SelectionTarget>,
    render: Option<RenderReference>,
    render_failed: bool,
    forward_pending: bool,
    editor_pending: bool,
    follow_on_install: bool,
}

impl Default for ViewSync {
    fn default() -> Self {
        Self {
            selection: None,
            render: None,
            render_failed: false,
            forward_pending: true,
            editor_pending: false,
            follow_on_install: true,
        }
    }
}

impl ViewSync {
    pub fn selection(&self) -> Option<SelectionTarget> {
        self.selection
    }

    pub fn preview_pending(&self) -> bool {
        self.forward_pending
    }

    pub fn editor_pending(&self) -> bool {
        self.editor_pending
    }

    /// False : fait périmé, d'une autre session ou incohérent ; aucun état changé.
    /// La validation documentaire reste celle du moteur, avant cet appel.
    pub fn apply(&mut self, event: ViewEvent) -> bool {
        match event {
            ViewEvent::DocumentReplaced { selection } => {
                self.replace_document(selection);
            }
            ViewEvent::DocumentPublished { selection, origin } => {
                if !self.publish(selection) {
                    return false;
                }
                self.follow_on_install = origin != ViewOrigin::Agent;
            }
            ViewEvent::CommandCompleted { result, origin } => {
                return self.command_completed(result, origin);
            }
            ViewEvent::SelectionChanged { selection, origin } => {
                if origin == ViewOrigin::Agent || !self.matches_guard(selection.guard) {
                    return false;
                }
                self.selection = Some(selection);
                self.forward_pending = origin != ViewOrigin::Preview;
                if origin == ViewOrigin::Preview {
                    self.editor_pending = true;
                }
            }
            ViewEvent::RevealRequested { guard, destination } => {
                if !self.matches_guard(guard) {
                    return false;
                }
                self.editor_pending |= destination != RevealDestination::Preview;
                self.forward_pending |= destination != RevealDestination::Editor;
            }
            ViewEvent::RenderInstalled { render } => {
                if !self.matches_guard(render.guard) {
                    return false;
                }
                self.render = Some(render);
                self.render_failed = false;
                self.forward_pending |= self.follow_on_install;
            }
            ViewEvent::RenderFailed { guard } => {
                if !self.matches_guard(guard) {
                    return false;
                }
                self.render_failed = true;
            }
        }
        true
    }

    fn replace_document(&mut self, selection: Option<SelectionTarget>) {
        *self = Self {
            selection,
            editor_pending: true,
            ..Default::default()
        };
    }

    fn matches_guard(&self, guard: TargetGuard) -> bool {
        self.selection.is_some_and(|target| target.guard == guard)
    }

    fn publish(&mut self, selection: SelectionTarget) -> bool {
        let Some(current) = self.selection else {
            return false;
        };
        if current.guard.document_id != selection.guard.document_id
            || selection.guard.revision < current.guard.revision
        {
            return false;
        }
        self.selection = Some(selection);
        true
    }

    fn command_completed(&mut self, result: ViewCommandResult, origin: ViewOrigin) -> bool {
        if result
            .selection
            .is_some_and(|selection| selection.guard != result.guard)
        {
            return false;
        }
        if result.document_replaced {
            self.replace_document(result.selection);
        } else if !result.selection.is_some_and(|target| self.publish(target)) {
            return false;
        }
        self.follow_on_install = origin != ViewOrigin::Agent;
        if origin != ViewOrigin::Agent {
            self.editor_pending |= result.reveal_selection || origin == ViewOrigin::Preview;
            if origin == ViewOrigin::Preview {
                self.forward_pending = false;
            } else {
                self.forward_pending |= result.changed || !result.saved;
            }
        }
        true
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
