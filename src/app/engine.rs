//! Adaptation des paramètres et des effets du moteur à l’interface egui.
use super::*;
use crate::engine::{Availability, Command as CoreCommand, CommandError};

impl App {
    pub(super) fn sync_published_selection(&mut self) {
        // Le curseur egui reste un cache graphique ; les lectures et appels
        // explicites utilisent la sélection publiée détenue par le moteur.
        let _ = self
            .document
            .engine
            .set_adapter_selection(self.document.selection.into());
    }
    pub(super) fn run_document_command(&mut self, command: CoreCommand) -> bool {
        self.cancel_ime();
        let result = self
            .document
            .engine
            .set_adapter_selection(self.document.selection.into())
            .and_then(|()| self.document.engine.run_call(command.to_call()));
        match result {
            Ok(_) => {
                self.document.selection = self.document.engine.selection().into();
                self.document.force_selection = true;
                self.preview.forward_pending = true;
                if self.search.open {
                    self.refresh_search();
                }
                true
            }
            Err(error) => {
                self.document.force_selection = true;
                self.dialogs.message = Some(Self::command_error_text(&error));
                false
            }
        }
    }
    pub(super) fn command_error_text(error: &CommandError) -> String {
        crate::i18n::text(match error.reason() {
            "protected_boundary" => "edit-protected-boundary",
            "invalid_text" => "edit-invalid-text",
            "foreign_document" | "stale_revision" | "unknown_zone" | "unknown_block" => {
                "edit-stale-draft"
            }
            "invalid_selection" | "invalid_projection" => "edit-invalid-draft",
            "select_link" => "select-link",
            "select_link_remove" => "select-link-remove",
            "link_one_paragraph" => "link-one-paragraph",
            "invalid_url" => "link-invalid-url",
            _ => match error.code {
                crate::engine::ErrorCode::InvalidParams => "engine-invalid-params",
                _ => "engine-unavailable",
            },
        })
    }
    pub(super) fn apply_document_command(&mut self, trigger: Command, command: CoreCommand) {
        let history = matches!(trigger, Command::Undo | Command::Redo);
        if history
            && matches!(
                self.document.engine.available(&command),
                Availability::Disabled { .. }
            )
        {
            return;
        }
        let previous = history.then(|| {
            (
                self.document.engine.document.clone(),
                self.document.engine.template.clone(),
            )
        });
        if !self.run_document_command(command) {
            return;
        }
        if let Some((old_document, old_template)) = previous {
            self.image_textures.clear();
            if self.document.engine.template != old_template {
                self.document.presentation = ModelPresentation::default();
            }
            self.document.force_selection = self.document.history_field.is_none()
                || self.document.engine.document != old_document;
            if self.document.force_selection {
                self.document.history_field = None;
            }
        }
        if matches!(
            trigger,
            Command::InsertPostscript | Command::InsertAttachment
        ) {
            self.preview.editor_scroll_pending = true;
        }
    }
}
