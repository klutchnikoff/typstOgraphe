use super::*;

impl App {
    #[cfg(test)]
    pub(super) fn install_model(&mut self, template: String) -> Result<(), String> {
        self.install_model_input(
            self.document.engine.target_guard(),
            crate::engine::ModelInput::Source { source: template },
        )
    }
    pub(super) fn install_model_input(
        &mut self,
        guard: crate::session::TargetGuard,
        model: crate::engine::ModelInput,
    ) -> Result<(), String> {
        self.execute_document_command(crate::engine::Command::SetModel(
            crate::engine::SetModelParams { guard, model },
        ))
        .map(|_| ())
        .map_err(|e| Self::command_error_text(&e))
    }
    pub(super) fn select_model(&mut self, model: Model) {
        let command = crate::engine::Command::from_call(
            model.definition().command.id,
            serde_json::json!({"guard":self.document.engine.target_guard()}),
        )
        .expect("preset registered in the typed catalogue");
        self.run_document_command(command);
    }
    pub(super) fn refresh_model_presentation(&mut self) {
        self.document.presentation = self
            .document
            .engine
            .query_call(crate::engine::Query::ModelPresentation(crate::engine::Empty {}).to_call())
            .ok()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        self.dialogs.marker_draft = None;
    }
    pub(super) fn import_model(&mut self) {
        let guard = self.document.engine.target_guard();
        let Some(path) = rfd::FileDialog::new()
            .set_title(Command::ImportModel.label())
            .add_filter(crate::tr!("model-file-filter"), &["typ"])
            .pick_file()
        else {
            return;
        };
        if let Err(error) =
            self.install_model_input(guard, crate::engine::ModelInput::Path { path })
        {
            self.file.error = Some(crate::tr!("model-import-error", error = error));
        }
    }
    pub(super) fn template_selector(&mut self, ui: &mut egui::Ui) {
        let commands: Vec<_> = self
            .keymap
            .registry()
            .definitions()
            .filter(|d| d.in_model_menu())
            .map(|d| d.command)
            .collect();
        let current = commands.iter().copied().find(|command| {
            command.model_source().as_deref() == Some(self.document.engine.template.as_str())
        });
        let mut selected = current;
        egui::ComboBox::from_id_salt("template")
            .selected_text(
                current.map_or_else(|| crate::tr!("embedded-template"), Command::short_label),
            )
            .show_ui(ui, |ui| {
                if current.is_none() {
                    ui.selectable_value(&mut selected, None, crate::tr!("embedded-template"));
                }
                for command in commands {
                    ui.selectable_value(&mut selected, Some(command), command.short_label())
                        .on_hover_text(self.keymap.hint(ui.ctx(), command));
                }
            });
        if selected != current
            && let Some(command) = selected
        {
            self.apply(command);
        }
    }
}
