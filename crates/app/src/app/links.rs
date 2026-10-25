use super::*;

impl App {
    pub(super) fn open_link(&mut self) {
        let range = self.range();
        let Some(target) = self.ui_text_target() else {
            return;
        };
        if let crate::engine::Availability::Disabled { reason } = self
            .document
            .engine
            .available_intent(crate::engine::CommandIntent::Link, target)
        {
            self.dialogs.message = Some(Self::command_error_text(&reason));
            return;
        }
        let url = self
            .document
            .engine
            .document()
            .view
            .styles
            .get(range.start)
            .and_then(|style| style.link.clone())
            .unwrap_or_default();
        if let Ok(target) = self
            .document
            .engine
            .selection_target(editor::to_core(self.document.selection))
        {
            self.show_band(target, super::bands::BandContent::Link { url });
        }
    }
}
