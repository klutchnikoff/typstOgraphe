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
        self.map_view_positions();
        if let Some(selection) = self.view_selection() {
            self.preview.sync.apply(ViewEvent::DocumentPublished {
                selection,
                origin: ViewOrigin::User,
            });
        }
    }
    pub(super) fn map_view_positions(&mut self) {
        if let Some(positions) = self.document.engine.position_map() {
            self.preview.sync.map_retained_positions(positions);
        }
    }
    pub(super) fn remember_visible_editor_position(
        &mut self,
        ui: &egui::Ui,
        output: &egui::text_edit::TextEditOutput,
    ) {
        // The IME draft and a newly edited galley must never supply a published anchor.
        if self.document.ime.draft.is_some()
            || output.galley.text() != self.document.engine.document().view.text
        {
            return;
        }
        let top = ui.clip_rect().top().max(output.galley_pos.y);
        let cursor = output
            .galley
            .cursor_from_pos(egui::vec2(0.0, top - output.galley_pos.y));
        let selection = crate::engine::Selection {
            primary: crate::engine::Caret {
                index: cursor.index,
                prefer_next_row: cursor.prefer_next_row,
            },
            secondary: crate::engine::Caret {
                index: cursor.index,
                prefer_next_row: cursor.prefer_next_row,
            },
        };
        if let Ok(target) = self.document.engine.selection_target(selection) {
            self.preview.sync.remember_visible_position(target);
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
