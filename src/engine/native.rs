//! Contrat de saisie : l’UI possède une projection, le moteur garde l’instantané original.
use super::*;

#[derive(Clone)]
pub(crate) struct NativeEdit {
    pub(super) before: Snapshot,
}
impl NativeEdit {
    pub fn selection(&self) -> Selection {
        self.before.selection
    }
    pub fn view(&self) -> &Projection {
        &self.before.document.view
    }
    pub fn guard(&self) -> TargetGuard {
        self.before.guard()
    }
}
#[derive(Clone, Copy)]
pub(crate) enum NativeInput {
    Typing,
    Composition,
}
impl Engine {
    pub(crate) fn begin_native_edit(&self, selection: Selection) -> NativeEdit {
        NativeEdit {
            before: self.session.snapshot(selection),
        }
    }
    pub(crate) fn commit_native_edit(
        &mut self,
        edit: NativeEdit,
        view: Projection,
        selection: Selection,
        input: NativeInput,
    ) -> Result<Selection, EditError> {
        self.commit_projection(
            edit.guard(),
            edit.before,
            view,
            selection,
            match input {
                NativeInput::Typing => EditGroup::Body,
                NativeInput::Composition => EditGroup::Discrete,
            },
        )
    }
    /// Un déplacement natif met à jour le contexte de frappe et clôt son groupe,
    /// sans publier une édition documentaire ni écraser un style activé sans déplacement.
    pub(crate) fn update_adapter_cursor(
        &mut self,
        selection: Selection,
    ) -> Result<(), CommandError> {
        self.set_adapter_selection(selection)?;
        let range = selection.range();
        self.session.document.view.typing_style = if !range.is_empty() {
            self.session.document.view.style_at(range.start)
        } else if selection.primary.index > 0 {
            self.session
                .document
                .view
                .style_at(selection.primary.index - 1)
        } else {
            Default::default()
        };
        self.session.typing_group = false;
        Ok(())
    }
}
