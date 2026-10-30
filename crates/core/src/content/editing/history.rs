//! View mementos belong to the native undo items, never to a second history.
use super::{Context, selection::AnchoredSelection};
use crate::content::commands::Style;
use std::sync::{Arc, Mutex};
use yrs::undo::UndoManager;

#[derive(Clone)]
pub(super) struct SavedView {
    pub selection: Option<AnchoredSelection>,
    pub styles: Vec<Style>,
}
pub(super) type ViewSlot = Arc<Mutex<Option<SavedView>>>;
pub(super) fn observe(
    history: &mut UndoManager<Option<SavedView>>,
    pending: ViewSlot,
    restored: ViewSlot,
) {
    // Keep the first selection when typing extends an existing capture. Undo
    // creates its inverse item through this same callback, capturing redo's view.
    history.observe_item_added("view", move |_, event| {
        *event.meta_mut() = pending.lock().unwrap().clone();
    });
    history.observe_item_popped("view", move |_, event| {
        *restored.lock().unwrap() = event.meta().clone();
    });
}
impl Context {
    pub(super) fn capture_view(&self) {
        *self.pending_view.lock().unwrap() = Some(SavedView {
            selection: self.selection.clone(),
            styles: self.styles.clone(),
        });
    }
    pub(super) fn restore_view(&mut self) {
        if let Some(view) = self.restored_view.lock().unwrap().take() {
            self.selection = view.selection;
            self.styles = view.styles;
        }
    }
}
