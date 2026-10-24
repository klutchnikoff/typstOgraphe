//! Adaptation des paramètres et des effets du moteur à l’interface egui.
use super::*;
use crate::engine::{Command as CoreCommand, CommandError};

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
        let model = matches!(
            command,
            CoreCommand::SetModel(_) | CoreCommand::PersonalModel(_)
        );
        let batch = matches!(command, CoreCommand::Batch(_)).then(|| {
            (
                self.document.engine.document().source().to_owned(),
                self.document.selection,
            )
        });
        let mut preserve_focus = model
            || matches!(
                command,
                CoreCommand::SetSignature(_)
                    | CoreCommand::SetMetadata(_)
                    | CoreCommand::SetOption(_)
                    | CoreCommand::SaveDocument(_)
                    | CoreCommand::SaveAs(_)
                    | CoreCommand::Export(_)
                    | CoreCommand::DeleteRecovery(_)
            );
        self.document
            .engine
            .set_adapter_selection(self.document.selection.into())?;
        let outcome = self.document.engine.run_call(command.to_call())?;
        if outcome
            .effects
            .contains(&crate::engine::Effect::DocumentReplaced)
        {
            self.after_document_replaced();
        }
        if outcome.effects.contains(&crate::engine::Effect::Saved) {
            self.recovery.recovered_name = None;
            self.clear_recovery();
        }
        self.document.selection = self.document.engine.selection().into();
        if let Some(presentation) = outcome
            .value
            .as_ref()
            .and_then(|value| value.get("presentation"))
            .filter(|value| !value.is_null())
        {
            self.document.presentation =
                serde_json::from_value(presentation.clone()).expect("typed model presentation");
            self.dialogs.marker_draft = None;
        }
        if let Some((source, selection)) = batch {
            preserve_focus = source == self.document.engine.document().source()
                && selection == self.document.selection;
        }
        if outcome
            .effects
            .contains(&crate::engine::Effect::RevealSelection)
        {
            self.preview.editor_scroll_pending = true;
        }
        if !preserve_focus {
            self.document.force_selection = true;
        }
        if outcome.changed || !outcome.effects.contains(&crate::engine::Effect::Saved) {
            self.preview.forward_pending = true;
        }
        if self.search.open {
            self.refresh_search();
        }
        Ok(outcome)
    }
    pub(super) fn run_document_command(&mut self, command: CoreCommand) -> bool {
        let preserve_focus = matches!(
            command,
            CoreCommand::SetMetadata(_)
                | CoreCommand::SetOption(_)
                | CoreCommand::SetModel(_)
                | CoreCommand::PersonalModel(_)
                | CoreCommand::Batch(_)
        );
        match self.execute_document_command(command) {
            Ok(_) => true,
            Err(error) => {
                self.document.force_selection |= !preserve_focus;
                self.dialogs.message = Some(Self::command_error_text(&error));
                false
            }
        }
    }
    pub(super) fn command_error_text(error: &CommandError) -> String {
        if error.reason() == "batch_step"
            && let Ok(cause) = serde_json::from_value::<CommandError>(error.params["error"].clone())
        {
            return crate::tr!(
                "engine-batch-step-error",
                step = (error.params["step"].as_u64().unwrap_or(0) + 1).to_string(),
                error = Self::command_error_text(&cause)
            );
        }
        if let Some(cause) = error.params.get("cause").and_then(|value| {
            serde_json::from_value::<crate::errors::CoreError>(value.clone()).ok()
        }) {
            let detail = crate::i18n::error(&cause);
            return if error.reason() == "model_validation" {
                format!(
                    "{}\n{detail}",
                    error.params["signature"].as_str().unwrap_or("")
                )
            } else if matches!(
                error.reason(),
                "image_decode" | "image_io" | "asset_validation"
            ) {
                crate::tr!("engine-image-error", error = detail)
            } else {
                detail
            };
        }
        if error.reason() == "model_validation" {
            return format!(
                "{}\n{}",
                error.params["signature"].as_str().unwrap_or(""),
                error.params["detail"].as_str().unwrap_or("")
            );
        }
        if matches!(
            error.reason(),
            "file_open"
                | "file_save"
                | "file_document"
                | "file_export"
                | "export_compile"
                | "recovery_load"
                | "recovery_delete"
        ) {
            return error.params["detail"].as_str().unwrap_or("").into();
        }
        if matches!(
            error.reason(),
            "model_io" | "model_utf8" | "model_dependencies"
        ) {
            return error.params["detail"].as_str().unwrap_or("").into();
        }
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
            "batch_step_limit" => "engine-batch-step-limit",
            "batch_resource_limit" => "engine-batch-resource-limit",
            "batch_size_limit" => "engine-batch-size-limit",
            "batch_step_forbidden" => "engine-batch-step-forbidden",
            "file_exists" => "file-already-exists",
            "unsaved_changes" => "unsaved-message",
            "creation_model" => "type-no-model",
            "export_required_fields" => "export-required-fields",
            "save_path_required" => "save-dialog",
            "model_too_large" => "model-too-large",
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
            "bullet_marker_invalid" => "bullet-marker-invalid",
            "bullet_marker_policy" => "model-bullet-marker-not-allowed",
            "model_fixed_page" => "model-fixed-page",
            _ => match error.code {
                crate::engine::ErrorCode::InvalidParams => "engine-invalid-params",
                _ => "engine-unavailable",
            },
        })
    }
    pub(super) fn apply_document_command(&mut self, trigger: Command, command: CoreCommand) {
        let history = matches!(trigger, Command::Undo | Command::Redo);
        let previous = history.then(|| {
            (
                self.document.engine.document().clone(),
                self.document.engine.template().to_owned(),
                self.document.engine.dependencies().clone(),
            )
        });
        if !self.run_document_command(command) {
            return;
        }
        if let Some((old_document, old_template, old_dependencies)) = previous {
            self.image_textures.clear();
            if self.document.engine.template() != old_template
                || self.document.engine.dependencies() != &old_dependencies
            {
                self.refresh_model_presentation();
            }
            self.document.force_selection = self.document.history_field.is_none()
                || self.document.engine.document() != &old_document;
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
