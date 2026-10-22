use super::*;

impl App {
    pub(super) fn install_model(&mut self, template: String) -> Result<(), String> {
        if template.len() > 1024 * 1024 {
            return Err(crate::tr!("model-too-large"));
        }
        let mut metadata = self.document.engine.metadata.clone();
        metadata.options_mut().bullet_marker = None;
        let mut world = crate::rendering::TypstWorld::default();
        let dependencies =
            crate::packages::with_existing(&template, &self.document.engine.dependencies);
        world.set_dependencies(&dependencies);
        world.set_assets(&self.document.engine.document.assets);
        let presentation = world
            .validate_model(self.document.engine.document.source(), &metadata, &template)
            .map_err(|error| format!("{}\n{}", metadata.model_signature(), error))?;
        let before = self.snapshot();
        self.document.engine.dependencies = dependencies;
        self.document.engine.template = template;
        self.document.engine.metadata = metadata;
        self.document.presentation = presentation;
        self.remember(before, false);
        Ok(())
    }
    pub(super) fn import_model(&mut self) {
        use std::io::Read;
        let Some(path) = rfd::FileDialog::new()
            .set_title(Command::ImportModel.label())
            .add_filter(crate::tr!("model-file-filter"), &["typ"])
            .pick_file()
        else {
            return;
        };
        let result = std::fs::File::open(path)
            .map_err(|error| error.to_string())
            .and_then(|file| {
                let mut source = String::new();
                file.take(1024 * 1024 + 1)
                    .read_to_string(&mut source)
                    .map_err(|error| error.to_string())?;
                self.install_model(source)
            });
        if let Err(error) = result {
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
