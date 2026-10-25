use super::*;

impl App {
    pub(super) fn modal_open(&self) -> bool {
        self.recovery.open
            || self.dialogs.assistance_open
            || self.dialogs.new_document_open
            || self.dialogs.preferences_open
            || self.dialogs.palette_open
            || self.file.export_open
            || self.file.pending_action.is_some()
            || self.file.read_only_offer.is_some()
            || self.file.error.is_some()
    }
    pub(super) fn welcome_ui(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        ui.add_space((height * 0.3).max(20.0));
        ui.vertical_centered(|ui| {
            ui.heading("typstOgraphe");
            ui.add_space(8.0);
            ui.label(crate::tr!("welcome-help"));
            ui.add_space(16.0);
            for command in [
                Command::File(FileCommand::New),
                Command::File(FileCommand::Open),
                Command::Preferences,
            ] {
                let response = ui.add_sized([240.0, 32.0], egui::Button::new(command.label()));
                if response.clicked() {
                    self.apply(command);
                }
            }
        });
    }
    pub fn capture_creation(&mut self, ctx: &egui::Context) {
        self.perform_action(PendingAction::New, ctx);
    }
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        self.new_document_dialog(ctx);
        self.palette_dialog(ctx);
        self.export_dialog(ctx);
        if !self.bands.parameter_ime
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.recovery.open = false;
            self.dialogs.assistance_open = false;
            self.dialogs.palette_open = false;
            self.dialogs.information_open = false;
            self.file.export_open = false;
            self.file.pending_action = None;
            self.file.error = None;
            self.document.force_selection = true;
        }
    }
    fn new_document_dialog(&mut self, ctx: &egui::Context) {
        if self.dialogs.new_document_open {
            let mut create = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("new-document")).show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.heading(crate::tr!("new-document-title"));
                ui.label(crate::tr!("field-document-type"));
                for kind in DocumentType::ALL {
                    ui.add_enabled_ui(kind.can_create(), |ui| {
                        if ui
                            .radio_value(&mut self.dialogs.new_document_type, kind, kind.label())
                            .changed()
                        {
                            self.dialogs.new_document_model =
                                kind.models().next().map(|entry| entry.model);
                        }
                    })
                    .response
                    .on_disabled_hover_text(crate::tr!("type-no-model"));
                }
                ui.add_space(8.0);
                ui.label(crate::tr!("field-template"));
                for entry in self.dialogs.new_document_type.models() {
                    ui.radio_value(
                        &mut self.dialogs.new_document_model,
                        Some(entry.model),
                        Command::Model(entry.model).short_label(),
                    );
                }
                ui.separator();
                ui.horizontal(|ui| {
                    cancel = ui.button(crate::tr!("cancel")).clicked();
                    let valid = self.dialogs.new_document_model.is_some_and(|model| {
                        model.definition().kind == self.dialogs.new_document_type
                    });
                    create = ui
                        .add_enabled(valid, egui::Button::new(crate::tr!("new-document-create")))
                        .clicked();
                });
            });
            if create {
                self.create_document(self.dialogs.new_document_type);
            } else if cancel || modal.should_close() {
                self.dialogs.new_document_open = false;
            }
        }
    }
    fn palette_dialog(&mut self, ctx: &egui::Context) {
        if self.dialogs.palette_open {
            let mut open = true;
            let mut choice = None;
            egui::Window::new(Command::Palette.label())
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(430.0)
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 85.0))
                .show(ctx, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.dialogs.palette_query)
                            .hint_text(crate::tr!("command-search"))
                            .desired_width(f32::INFINITY),
                    );
                    if !response.has_focus() {
                        response.request_focus();
                    }
                    if response.changed() {
                        self.dialogs.palette_index = 0;
                    }
                    let commands = self
                        .keymap
                        .registry()
                        .definitions()
                        .filter(|definition| {
                            definition.in_palette() && self.command_enabled(definition.command)
                        })
                        .map(|definition| definition.command);
                    let query = self.dialogs.palette_query.to_lowercase();
                    let filtered: Vec<_> = commands
                        .into_iter()
                        .filter(|c| c.label().to_lowercase().contains(&query))
                        .collect();
                    self.dialogs.palette_index = self
                        .dialogs
                        .palette_index
                        .min(filtered.len().saturating_sub(1));
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
                        && !filtered.is_empty()
                    {
                        self.dialogs.palette_index =
                            (self.dialogs.palette_index + 1) % filtered.len();
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
                        && !filtered.is_empty()
                    {
                        self.dialogs.palette_index =
                            (self.dialogs.palette_index + filtered.len() - 1) % filtered.len();
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                        choice = filtered.get(self.dialogs.palette_index).copied();
                    }
                    for (index, command) in filtered.into_iter().enumerate() {
                        if ui
                            .add(
                                egui::Button::new(command.label())
                                    .frame(false)
                                    .selected(index == self.dialogs.palette_index)
                                    .shortcut_text(self.keymap.shortcut_text(ui.ctx(), command)),
                            )
                            .clicked()
                        {
                            choice = Some(command);
                        }
                    }
                });
            self.dialogs.palette_open &= open;
            if let Some(command) = choice {
                self.dialogs.palette_open = false;
                self.dispatch_action(command, ctx);
            }
        }
    }
    fn export_dialog(&mut self, ctx: &egui::Context) {
        if self.file.export_open {
            let mut open = true;
            let mut export = false;
            egui::Window::new(crate::tr!("export-title"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(470.0)
                .show(ctx, |ui| {
                    ui.label(crate::tr!("export-folder"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.file.export_path)
                            .desired_width(f32::INFINITY),
                    );
                    ui.horizontal(|ui| {
                        ui.label(crate::tr!("export-format"));
                        for (format, key) in [
                            (crate::engine::ExportFormat::Pdf, "export-pdf"),
                            (crate::engine::ExportFormat::Typst, "export-typst"),
                            (crate::engine::ExportFormat::Both, "export-both"),
                        ] {
                            ui.selectable_value(
                                &mut self.file.export_format,
                                format,
                                crate::i18n::text(key),
                            );
                        }
                    });
                    ui.checkbox(
                        &mut self.file.export_page_images,
                        crate::tr!("export-page-images"),
                    );
                    ui.weak(crate::tr!("export-help"));
                    export = ui.button(crate::tr!("export-button")).clicked();
                });
            self.file.export_open &= open;
            if export && let Some(guard) = self.file.export_guard {
                self.export_document(
                    crate::engine::ExportParams {
                        guard,
                        directory: PathBuf::from(&self.file.export_path),
                        format: self.file.export_format,
                        page_images: self.file.export_page_images,
                    },
                    ctx,
                );
            }
        }
    }
}
