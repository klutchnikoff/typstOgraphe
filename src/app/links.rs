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
        self.dialogs.link_url = self
            .document
            .engine
            .document()
            .view
            .styles
            .get(range.start)
            .and_then(|s| s.link.clone())
            .unwrap_or_default();
        self.dialogs.link_range = range;
        self.dialogs.link_target = self
            .document
            .engine
            .selection_target(editor::to_core(self.document.selection))
            .ok();
        self.dialogs.link_error = None;
        self.dialogs.link_open = true;
    }
}
