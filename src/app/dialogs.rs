use super::*;

impl App {
    pub(super) fn modal_open(&self) -> bool {
        self.recovery_open
            || self.new_document_open
            || self.image_info.is_some()
            || self.preferences_open
            || self.palette_open
            || self.link_open
            || self.export_open
            || self.pending_action.is_some()
            || self.file_error.is_some()
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
            self.recovery_open = false;
            self.palette_open = false;
            self.link_open = false;
            self.information_open = false;
            self.export_open = false;
            self.pending_action = None;
            self.file_error = None;
            self.force_selection = true;
        }
    }
    fn new_document_dialog(&mut self, ctx: &egui::Context) {
        if self.new_document_open {
            let mut create = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("new-document")).show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.heading(crate::tr!("new-document-title"));
                ui.label(crate::tr!("field-document-type"));
                for kind in DocumentType::ALL {
                    ui.add_enabled_ui(kind.can_create(), |ui| {
                        if ui
                            .radio_value(&mut self.new_document_type, kind, kind.label())
                            .changed()
                        {
                            self.new_document_model = kind.models().next().map(|entry| entry.model);
                        }
                    })
                    .response
                    .on_disabled_hover_text(crate::tr!("type-no-model"));
                }
                ui.add_space(8.0);
                ui.label(crate::tr!("field-template"));
                for entry in self.new_document_type.models() {
                    ui.radio_value(
                        &mut self.new_document_model,
                        Some(entry.model),
                        entry.command.command.short_label(),
                    );
                }
                ui.separator();
                ui.horizontal(|ui| {
                    cancel = ui.button(crate::tr!("cancel")).clicked();
                    let valid = self
                        .new_document_model
                        .is_some_and(|model| model.definition().kind == self.new_document_type);
                    create = ui
                        .add_enabled(valid, egui::Button::new(crate::tr!("new-document-create")))
                        .clicked();
                });
            });
            if create {
                self.create_document(self.new_document_type);
            } else if cancel || modal.should_close() {
                self.new_document_open = false;
            }
        }
    }
    fn image_dialog(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.image_info {
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
                        egui::TextEdit::multiline(&mut self.image_alt)
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
                if let Some(image) = self.session.document.view.images.get_mut(&id) {
                    image.alt.clone_from(&self.image_alt);
                    self.remember(before, false);
                }
                open = false;
                self.force_selection = true;
                self.editor_scroll_pending = true;
            }
            if !open {
                self.image_info = None;
                if let Some(range) = self.session.document.view.image_range(id) {
                    self.focus_annotation_target(range.start + 1..range.start + 1);
                }
            }
        }
    }
    fn link_dialog(&mut self, ctx: &egui::Context) {
        if self.link_open {
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
                            .session
                            .document
                            .view
                            .text
                            .chars()
                            .skip(self.link_range.start)
                            .take(self.link_range.len())
                            .collect::<String>()
                    ));
                    ui.label(crate::tr!("field-address"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.link_url)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://…"),
                    );
                    if let Some(error) = &self.link_error {
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
            self.link_open &= open;
            if remove {
                if self.range() != self.link_range {
                    self.selection = editor::cursor(self.link_range.clone());
                }
                self.apply(Command::RemoveLink);
                self.link_open = false;
                self.force_selection = true;
            } else if apply {
                let url = self.link_url.trim();
                if (url.starts_with("https://")
                    || url.starts_with("http://")
                    || url.starts_with("mailto:"))
                    && !url.chars().any(char::is_whitespace)
                {
                    let before = self.snapshot();
                    self.session
                        .document
                        .view
                        .set_link(self.link_range.clone(), Some(url.into()));
                    self.remember(before, false);
                    self.link_open = false;
                    self.force_selection = true;
                } else {
                    self.link_error = Some(crate::tr!("link-invalid-url"));
                }
            }
        }
    }
    fn palette_dialog(&mut self, ctx: &egui::Context) {
        if self.palette_open {
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
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text(crate::tr!("command-search"))
                            .desired_width(f32::INFINITY),
                    );
                    if !response.has_focus() {
                        response.request_focus();
                    }
                    if response.changed() {
                        self.palette_index = 0;
                    }
                    let commands = self
                        .keymap
                        .registry()
                        .definitions()
                        .filter(|definition| {
                            definition.in_palette()
                                && (self.document_open
                                    || definition.command.available_without_document())
                        })
                        .map(|definition| definition.command);
                    let query = self.palette_query.to_lowercase();
                    let filtered: Vec<_> = commands
                        .into_iter()
                        .filter(|c| c.label().to_lowercase().contains(&query))
                        .collect();
                    self.palette_index = self.palette_index.min(filtered.len().saturating_sub(1));
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
                        && !filtered.is_empty()
                    {
                        self.palette_index = (self.palette_index + 1) % filtered.len();
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
                        && !filtered.is_empty()
                    {
                        self.palette_index =
                            (self.palette_index + filtered.len() - 1) % filtered.len();
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                        choice = filtered.get(self.palette_index).copied();
                    }
                    for (index, command) in filtered.into_iter().enumerate() {
                        if ui
                            .add(
                                egui::Button::new(command.label())
                                    .frame(false)
                                    .selected(index == self.palette_index)
                                    .shortcut_text(self.keymap.shortcut_text(ui.ctx(), command)),
                            )
                            .clicked()
                        {
                            choice = Some(command);
                        }
                    }
                });
            self.palette_open &= open;
            if let Some(command) = choice {
                self.palette_open = false;
                self.dispatch_action(command, ctx);
            }
        }
    }
    fn export_dialog(&mut self, ctx: &egui::Context) {
        if self.export_open {
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
                        egui::TextEdit::singleline(&mut self.export_path)
                            .desired_width(f32::INFINITY),
                    );
                    ui.weak(crate::tr!("export-help"));
                    export = ui.button(crate::tr!("export-button")).clicked();
                });
            self.export_open &= open;
            if export {
                if !self.session.metadata.ready_for_export() {
                    self.message = Some(crate::tr!("export-required-fields"));
                } else {
                    match self
                        .worker
                        .sender
                        .send(self.request(Some(PathBuf::from(&self.export_path))))
                    {
                        Ok(()) => {
                            self.export_open = false;
                            self.message = Some(crate::tr!("export-pending"));
                        }
                        Err(_) => self.message = Some(crate::tr!("renderer-stopped")),
                    }
                }
            }
        }
    }
}
