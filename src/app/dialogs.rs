use super::*;

impl App {
    pub(super) fn modal_open(&self) -> bool {
        self.recovery.open
            || self.dialogs.new_document_open
            || self.dialogs.image_info.is_some()
            || self.dialogs.preferences_open
            || self.dialogs.palette_open
            || self.dialogs.link_open
            || self.file.export_open
            || self.file.pending_action.is_some()
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
        self.image_dialog(ctx);
        self.link_dialog(ctx);
        self.palette_dialog(ctx);
        self.export_dialog(ctx);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.recovery.open = false;
            self.dialogs.palette_open = false;
            self.dialogs.link_open = false;
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
                        entry.command.command.short_label(),
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
    fn image_dialog(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.dialogs.image_info {
            let mut open = true;
            let mut apply = false;
            let mut cancel = false;
            egui::Window::new(crate::tr!("image-information"))
                .open(&mut open)
                .collapsible(false)
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label(crate::tr!("image-alt-label"));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.dialogs.image_alt)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3),
                    );
                    ui.label(
                        egui::RichText::new(crate::tr!("image-alt-help"))
                            .small()
                            .weak(),
                    );
                    ui.horizontal(|ui| {
                        apply = ui.button(crate::tr!("image-apply")).clicked();
                        cancel = ui.button(crate::tr!("image-cancel")).clicked();
                    });
                });
            if cancel {
                open = false;
            }
            if apply {
                let before = self.snapshot();
                if let Some(image) = self.document.engine.document.view.images.get_mut(&id) {
                    image.alt.clone_from(&self.dialogs.image_alt);
                    self.remember(before, false);
                }
                open = false;
                self.document.force_selection = true;
                self.preview.editor_scroll_pending = true;
            }
            if !open {
                self.dialogs.image_info = None;
                if let Some(range) = self.document.engine.document.view.image_range(id) {
                    self.focus_annotation_target(range.start + 1..range.start + 1);
                }
            }
        }
    }
    fn link_dialog(&mut self, ctx: &egui::Context) {
        if self.dialogs.link_open {
            let mut open = true;
            let mut apply = false;
            let mut remove = false;
            egui::Window::new(crate::tr!("link-title"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label(crate::tr!(
                        "link-selected-text",
                        text = self
                            .document
                            .engine
                            .document
                            .view
                            .text
                            .chars()
                            .skip(self.dialogs.link_range.start)
                            .take(self.dialogs.link_range.len())
                            .collect::<String>()
                    ));
                    ui.label(crate::tr!("field-address"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.dialogs.link_url)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://…"),
                    );
                    if let Some(error) = &self.dialogs.link_error {
                        ui.colored_label(Color32::from_rgb(183, 69, 57), error);
                    }
                    ui.horizontal(|ui| {
                        apply = ui.button(crate::tr!("apply")).clicked();
                        remove = ui
                            .button(Command::RemoveLink.short_label())
                            .on_hover_text(self.keymap.hint(ui.ctx(), Command::RemoveLink))
                            .clicked();
                    });
                });
            self.dialogs.link_open &= open;
            if remove || apply {
                if let Some(target) = self.dialogs.link_target {
                    let command = if remove {
                        crate::engine::Command::RemoveLink(crate::engine::TargetParams {
                            target: target.into(),
                        })
                    } else {
                        crate::engine::Command::SetLink(crate::engine::LinkParams {
                            target: target.into(),
                            url: self.dialogs.link_url.clone(),
                        })
                    };
                    if self.run_document_command(command) {
                        self.dialogs.link_open = false;
                    } else {
                        self.dialogs.link_error.clone_from(&self.dialogs.message);
                    }
                } else {
                    self.dialogs.link_error = Some(crate::tr!("edit-stale-draft"));
                }
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
                            definition.in_palette()
                                && (self.document.open
                                    || definition.command.available_without_document())
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
                    ui.weak(crate::tr!("export-help"));
                    export = ui.button(crate::tr!("export-button")).clicked();
                });
            self.file.export_open &= open;
            if export {
                if !self.document.engine.metadata.ready_for_export() {
                    self.dialogs.message = Some(crate::tr!("export-required-fields"));
                } else {
                    match self
                        .preview
                        .worker
                        .sender
                        .send(self.request(Some(PathBuf::from(&self.file.export_path))))
                    {
                        Ok(()) => {
                            self.file.export_open = false;
                            self.dialogs.message = Some(crate::tr!("export-pending"));
                        }
                        Err(_) => self.dialogs.message = Some(crate::tr!("renderer-stopped")),
                    }
                }
            }
        }
    }
}
