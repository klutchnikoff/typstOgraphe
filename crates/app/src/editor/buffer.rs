//! Types locaux à l’adaptateur : aucun trait egui n’est implémenté sur un type du cœur.
use super::*;

pub fn to_core(range: CCursorRange) -> crate::engine::Selection {
    let caret = |c: CCursor| crate::engine::Caret {
        index: c.index,
        prefer_next_row: c.prefer_next_row,
    };
    crate::engine::Selection {
        primary: caret(range.primary),
        secondary: caret(range.secondary),
    }
}
pub fn to_editor(range: crate::engine::Selection) -> CCursorRange {
    let caret = |c: crate::engine::Caret| CCursor {
        index: c.index,
        prefer_next_row: c.prefer_next_row,
    };
    CCursorRange {
        primary: caret(range.primary),
        secondary: caret(range.secondary),
        h_pos: None,
    }
}
pub struct ProjectionBuffer<'a>(pub &'a mut Projection);
impl TextBuffer for ProjectionBuffer<'_> {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Projection>()
    }
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        &self.0.text
    }
    fn insert_text(&mut self, text: &str, at: usize) -> usize {
        self.0.insert_text(text, at)
    }
    fn delete_char_range(&mut self, range: Range<usize>) {
        self.0.delete_char_range(range);
    }
    fn replace_with(&mut self, text: &str) {
        self.0.replace_with(text);
    }
}

pub struct InputBuffer<'a>(pub &'a mut crate::document::edits::TextInput);
impl TextBuffer for InputBuffer<'_> {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<crate::document::edits::TextInput>()
    }
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        &self.0.view.text
    }
    fn insert_text(&mut self, text: &str, at: usize) -> usize {
        self.0.insert_text(text, at)
    }
    fn delete_char_range(&mut self, range: Range<usize>) {
        self.0.delete_char_range(range);
    }
    fn replace_with(&mut self, text: &str) {
        self.0.replace_with(text);
    }
}
