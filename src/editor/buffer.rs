//! buffer : détail interne de l’adaptateur egui.
use super::*;
impl From<CCursorRange> for crate::session::Selection {
    fn from(range: CCursorRange) -> Self {
        let caret = |c: CCursor| crate::session::Caret {
            index: c.index,
            prefer_next_row: c.prefer_next_row,
        };
        Self {
            primary: caret(range.primary),
            secondary: caret(range.secondary),
        }
    }
}
impl From<crate::session::Selection> for CCursorRange {
    fn from(range: crate::session::Selection) -> Self {
        let caret = |c: crate::session::Caret| CCursor {
            index: c.index,
            prefer_next_row: c.prefer_next_row,
        };
        Self {
            primary: caret(range.primary),
            secondary: caret(range.secondary),
            h_pos: None,
        }
    }
}

// Adaptateur de saisie : le document et ses opérations ne dépendent pas d’egui.
impl TextBuffer for Projection {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        &self.text
    }
    fn insert_text(&mut self, text: &str, at: usize) -> usize {
        Projection::insert_text(self, text, at)
    }
    fn delete_char_range(&mut self, range: Range<usize>) {
        Projection::delete_char_range(self, range);
    }
    fn replace_with(&mut self, text: &str) {
        Projection::replace_with(self, text);
    }
}

impl TextBuffer for crate::document::edits::TextInput {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        &self.view.text
    }
    fn insert_text(&mut self, text: &str, at: usize) -> usize {
        Self::insert_text(self, text, at)
    }
    fn delete_char_range(&mut self, range: Range<usize>) {
        Self::delete_char_range(self, range);
    }
    fn replace_with(&mut self, text: &str) {
        Self::replace_with(self, text);
    }
}
