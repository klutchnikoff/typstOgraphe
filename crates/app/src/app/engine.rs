//! Adaptation des paramètres et des effets du moteur à l’interface egui.
use super::*;
use crate::engine::{Command as CoreCommand, CommandError};

impl App {
    pub(super) fn select_description_term(&mut self, at: usize) {
        if let Ok(target) = self
            .document
            .engine
            .selection_target(editor::to_core(editor::cursor(at..at)))
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
            .set_adapter_selection(editor::to_core(self.document.selection));
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
            .set_adapter_selection(editor::to_core(self.document.selection))?;
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
        self.document.selection = editor::to_editor(self.document.engine.selection());
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
        crate::i18n::command_error(error)
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
