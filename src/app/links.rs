use super::*;

impl App {
    pub(super) fn open_link(&mut self) {
        let range = self.range();
        if range.is_empty() {
            self.dialogs.message = Some(crate::tr!("select-link"));
            return;
        }
        if !self
            .document
            .session
            .document
            .view
            .is_single_paragraph(range.clone())
        {
            self.dialogs.message = Some(crate::tr!("link-one-paragraph"));
            return;
        }
        self.dialogs.link_url = self
            .document
            .session
            .document
            .view
            .styles
            .get(range.start)
            .and_then(|s| s.link.clone())
            .unwrap_or_default();
        self.dialogs.link_range = range;
        self.dialogs.link_error = None;
        self.dialogs.link_open = true;
    }
}
