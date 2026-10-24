//! Conversion du curseur natif en faits publiés ; les intentions sont dans le cœur.
use super::*;

impl App {
    fn view_selection(&self) -> Option<crate::engine::SelectionTarget> {
        if !self.document.open {
            return None;
        }
        self.document
            .engine
            .selection_target(editor::to_core(self.document.selection))
            .ok()
    }

    /// Actualiser la position sans créer une intention de navigation. Une
    /// préédition ne modifie pas document.selection et n'entre donc pas ici.
    pub(super) fn publish_view_selection(&mut self) {
        if let Some(selection) = self.view_selection() {
            self.preview.sync.apply(ViewEvent::DocumentPublished {
                selection,
                origin: ViewOrigin::User,
            });
        }
    }

    pub(super) fn view_selection_changed(&mut self, origin: ViewOrigin) {
        if let Some(selection) = self.view_selection() {
            self.preview
                .sync
                .apply(ViewEvent::SelectionChanged { selection, origin });
        }
    }

    pub(super) fn request_view_reveal(&mut self, destination: RevealDestination) {
        self.publish_view_selection();
        self.preview.sync.apply(ViewEvent::RevealRequested {
            guard: self.document.engine.target_guard(),
            destination,
        });
    }

    pub(super) fn reset_view_sync(&mut self) {
        let selection = self.view_selection();
        self.preview
            .sync
            .apply(ViewEvent::DocumentReplaced { selection });
    }
}
