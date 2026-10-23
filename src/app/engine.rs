//! Adaptation des paramètres et des effets du moteur à l’interface egui.
use super::*;
use crate::engine::{Availability, Command as CoreCommand, CommandError};

impl App {
    pub(super) fn select_description_term(&mut self, at: usize) {
        if let Ok(target) = self
            .document
            .engine
            .selection_target(editor::cursor(at..at).into())
        {
            self.run_document_command(CoreCommand::SelectText(crate::engine::SelectTextParams {
                target: target.into(),
                part: Some(crate::engine::SelectionPart::Term),
            }));
        }
    }
    pub(super) fn sync_published_selection(&mut self) {
        // Le curseur egui reste un cache graphique ; les lectures et appels
        // explicites utilisent la sélection publiée détenue par le moteur.
        let _ = self
            .document
            .engine
            .set_adapter_selection(self.document.selection.into());
    }
    pub(super) fn execute_document_command(
        &mut self,
        command: CoreCommand,
    ) -> Result<crate::engine::Outcome, CommandError> {
        self.cancel_ime();
        let signature = matches!(command, CoreCommand::SetSignature(_));
        self.document
            .engine
            .set_adapter_selection(self.document.selection.into())?;
        let outcome = self.document.engine.run_call(command.to_call())?;
        self.document.selection = self.document.engine.selection().into();
        if outcome
            .effects
            .contains(&crate::engine::Effect::RevealSelection)
        {
            self.preview.editor_scroll_pending = true;
        }
        if !signature {
            self.document.force_selection = true;
        }
        self.preview.forward_pending = true;
        if self.search.open {
            self.refresh_search();
        }
        Ok(outcome)
    }
    pub(super) fn run_document_command(&mut self, command: CoreCommand) -> bool {
        match self.execute_document_command(command) {
            Ok(_) => true,
            Err(error) => {
                self.document.force_selection = true;
                self.dialogs.message = Some(Self::command_error_text(&error));
                false
            }
        }
    }
    pub(super) fn command_error_text(error: &CommandError) -> String {
        if matches!(
            error.reason(),
            "image_decode" | "image_io" | "image_base64" | "asset_validation"
        ) {
            return crate::tr!(
                "engine-image-error",
                error = error.params["detail"].as_str().unwrap_or("")
            );
        }
        crate::i18n::text(match error.reason() {
            "image_too_large" => "image-too-large",
            "image_format" => "image-format-unsupported",
            "image_svg" => "image-invalid-svg",
            "document_too_large" => "storage-letter-too-large",
            "protected_boundary" => "edit-protected-boundary",
            "invalid_text" => "edit-invalid-text",
            "search_too_large" => "search-too-large",
            "search_one_line" => "search-one-line",
            "search_mismatch" => "edit-stale-draft",
            "spelling_mismatch" | "spelling_language" => "edit-stale-draft",
            "spelling_dictionary" => "spelling-error",
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
