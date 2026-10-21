use super::*;

impl App {
    pub(super) fn open_link(&mut self) {
        let range = self.range();
        if range.is_empty() {
            self.message = Some(crate::tr!("select-link"));
            return;
        }
        if !self
            .session
            .document
            .view
            .is_single_paragraph(range.clone())
        {
            self.message = Some(crate::tr!("link-one-paragraph"));
            return;
        }
        self.link_url = self
            .session
            .document
            .view
            .styles
            .get(range.start)
            .and_then(|s| s.link.clone())
            .unwrap_or_default();
        self.link_range = range;
        self.link_error = None;
        self.link_open = true;
    }
}
