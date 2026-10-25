//! Coordination source ↔ aperçu par vue, séparée du moteur documentaire.
//!
//! L'adaptateur publie des faits ordonnés puis consomme les actions au moment
//! où il peut les peindre. Aucun focus, pixel, IME, délai, worker ou accès disque.
//! Les lectures géométriques empruntent le moteur sans modifier sa sélection.
use crate::engine::{PositionMap, RenderReference, SelectionTarget, TargetGuard};

mod geometry;
mod types;
pub use types::*;

/// Une instance par vue ; ce composant ne crée pas de contexte d'édition moteur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewSync {
    selection: Option<SelectionTarget>,
    visible_position: Option<SelectionTarget>,
    reveal_target: Option<SelectionTarget>,
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
            visible_position: None,
            reveal_target: None,
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
    pub fn reveal_target(&self) -> Option<SelectionTarget> {
        self.reveal_target
    }
    /// Voluntary navigation/highlight, independent from the user's selection.
    pub fn reveal_at(
        &mut self,
        engine: &crate::engine::Engine,
        target: SelectionTarget,
        destination: RevealDestination,
    ) -> bool {
        if !engine.is_open()
            || !self.matches_guard(target.guard)
            || engine.target_guard() != target.guard
            || engine.resolve_target(target).is_err()
        {
            return false;
        }
        self.reveal_target = Some(target);
        self.editor_pending = destination != RevealDestination::Preview;
        self.forward_pending = destination != RevealDestination::Editor;
        true
    }
    pub fn clear_reveal(&mut self) {
        if self.reveal_target.take().is_some() {
            self.editor_pending = false;
            self.forward_pending = false;
        }
    }
    /// Logical anchor of the visible passage. Pixel offsets belong to the adapter.
    pub fn visible_position(&self) -> Option<SelectionTarget> {
        self.visible_position
    }
    pub fn remember_visible_position(&mut self, target: SelectionTarget) -> bool {
        if !self.matches_guard(target.guard) {
            return false;
        }
        self.visible_position = Some(target);
        true
    }
    /// Advance retained user positions without creating a reveal/focus intention.
    /// This does not make stale command targets valid. Call once per publication,
    /// before publishing the native selection or adapting the command outcome.
    pub fn map_retained_positions(&mut self, positions: &PositionMap) -> bool {
        let Some(current) = self.selection.filter(|s| s.guard == positions.before) else {
            return false;
        };
        let Ok(selection) = positions.map_retained(current) else {
            return false;
        };
        let visible = match self.visible_position {
            Some(target) => {
                let Ok(mapped) = positions.map_retained(target) else {
                    return false;
                };
                Some(mapped)
            }
            None => None,
        };
        self.selection = Some(selection);
        self.visible_position = visible;
        if self
            .reveal_target
            .is_some_and(|t| t.guard != positions.after)
        {
            self.clear_reveal();
        }
        true
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
                self.clear_reveal();
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
        if self
            .reveal_target
            .is_some_and(|t| t.guard != selection.guard)
        {
            self.clear_reveal();
        }
        if self
            .visible_position
            .is_some_and(|p| p.guard != selection.guard)
        {
            self.visible_position = None;
        }
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
