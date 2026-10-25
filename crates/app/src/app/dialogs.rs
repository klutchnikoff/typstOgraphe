use super::*;

impl App {
    pub(super) fn modal_open(&self) -> bool {
        self.dialogs.palette_open || self.modal_without_palette()
    }
    pub(super) fn modal_without_palette(&self) -> bool {
        self.recovery.open
            || self.dialogs.assistance_open
            || self.dialogs.new_document_open
            || self.dialogs.preferences_open
            || self.file.export_open
            || self.file.pending_action.is_some()
            || self.file.read_only_offer.is_some()
            || self.file.error.is_some()
    }
    pub(super) fn welcome_ui(&mut self, ui: &mut egui::Ui) {
        use crate::ui_kit::{ButtonKind, button_sized};
        let height = ui.available_height();
        let muted = crate::theme::Tokens::new(ui.visuals().dark_mode).muted;
        ui.add_space((height * 0.3).max(20.0));
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("typstOgraphe")
                    .size(28.0)
                    .family(crate::theme::semibold()),
            );
            ui.add_space(6.0);
            ui.label(egui::RichText::new(crate::tr!("welcome-help")).color(muted));
            ui.add_space(20.0);
            for (command, kind) in [
                (Command::File(FileCommand::New), ButtonKind::Primary),
                (Command::File(FileCommand::Open), ButtonKind::Secondary),
                (Command::Preferences, ButtonKind::Ghost),
            ] {
                if button_sized(ui, kind, true, egui::vec2(260.0, 36.0), command.label()).clicked()
                {
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
            self.dialogs.side_rail.close();
            self.dialogs.information_open = false;
            self.file.export_open = false;
            self.file.pending_action = None;
            self.file.error = None;
            self.document.force_selection = true;
        }
    }
    fn new_document_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui_kit::{ButtonKind, Dialog, button, button_enabled};
        if self.dialogs.new_document_open {
            let mut create = false;
            let mut cancel = false;
            let valid = self
                .dialogs
                .new_document_model
                .is_some_and(|model| model.definition().kind == self.dialogs.new_document_type);
            let title = crate::tr!("new-document-title");
            let modal = Dialog::new("new-document", &title).width(440.0).show(
                ctx,
                |ui| {
                    ui.label(crate::tr!("field-document-type"));
                    for kind in DocumentType::ALL {
                        ui.add_enabled_ui(kind.can_create(), |ui| {
                            if ui
                                .radio_value(
                                    &mut self.dialogs.new_document_type,
                                    kind,
                                    kind.label(),
                                )
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
                },
                |ui| {
                    create = button_enabled(
                        ui,
                        ButtonKind::Primary,
                        valid,
                        crate::tr!("new-document-create"),
                    )
                    .clicked();
                    cancel = button(ui, ButtonKind::Ghost, crate::tr!("cancel")).clicked();
                },
            );
            if create {
                self.create_document(self.dialogs.new_document_type);
            } else if cancel || modal.should_close() {
                self.dialogs.new_document_open = false;
            }
        }
    }
    fn palette_dialog(&mut self, ctx: &egui::Context) {
        if !self.dialogs.palette_open {
            return;
        }
        let dark = ctx.style().visuals.dark_mode;
        let t = crate::theme::Tokens::new(dark);
        let mut choice = None;
        let id = egui::Id::new("command-palette");
        let frame = egui::Frame::new()
            .fill(t.raised)
            .stroke(egui::Stroke::new(1.0_f32, t.border))
            .corner_radius(crate::theme::scale::RADIUS_DIALOG)
            .shadow(egui::Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 0,
                color: egui::Color32::from_black_alpha(if dark { 115 } else { 40 }),
            });
        // Calque non modal : l'interface est déjà désactivée par `modal_open()`.
        let area = egui::Area::new(id)
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 85.0));
        let shown = area.show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(560.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.dialogs.palette_query)
                        .hint_text(crate::tr!("command-search"))
                        .desired_width(f32::INFINITY)
                        .font(egui::FontId::proportional(15.0))
                        .margin(egui::Margin::symmetric(16, 12))
                        .frame(false),
                );
                if !response.has_focus() {
                    response.request_focus();
                }
                let mut reveal_selection = response.changed() || response.gained_focus();
                if response.changed() {
                    self.dialogs.palette_index = 0;
                }
                ui.separator();
                let filtered = self.palette_entries();
                self.dialogs.palette_index = self
                    .dialogs
                    .palette_index
                    .min(filtered.len().saturating_sub(1));
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
                    && !filtered.is_empty()
                {
                    self.dialogs.palette_index = (self.dialogs.palette_index + 1) % filtered.len();
                    reveal_selection = true;
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
                    && !filtered.is_empty()
                {
                    self.dialogs.palette_index =
                        (self.dialogs.palette_index + filtered.len() - 1) % filtered.len();
                    reveal_selection = true;
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                    choice = filtered.get(self.dialogs.palette_index).map(|(_, c)| *c);
                }
                self.palette_rows(ui, filtered, reveal_selection, &mut choice);
            });
        });
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|pos| !shown.response.rect.contains(pos))
        });
        if outside {
            self.dialogs.palette_open = false;
        }
        if let Some(command) = choice {
            self.dialogs.palette_open = false;
            self.dispatch_action(command, ctx);
        }
    }
    /// Commandes disponibles correspondant à la recherche, rangées par groupe de menu.
    fn palette_entries(&self) -> Vec<(Option<crate::commands::MenuGroup>, Command)> {
        let query = self.dialogs.palette_query.to_lowercase();
        let group_of = |command: Command| {
            self.keymap
                .registry()
                .definitions()
                .find(|definition| definition.command == command)
                .and_then(|definition| definition.menu_position())
                .map(|(group, _)| group)
        };
        let mut entries: Vec<_> = self
            .keymap
            .registry()
            .definitions()
            .filter(|definition| {
                definition.in_palette() && self.command_enabled(definition.command)
            })
            .map(|definition| definition.command)
            .filter(|c| {
                c.label().to_lowercase().contains(&query)
                    || self.ui_command_label(*c).to_lowercase().contains(&query)
            })
            .map(|command| (group_of(command), command))
            .collect();
        entries.sort_by_key(|(group, _)| {
            crate::menus::GROUPS
                .iter()
                .position(|g| Some(*g) == *group)
                .unwrap_or(usize::MAX)
        });
        entries
    }
    fn palette_rows(
        &self,
        ui: &mut egui::Ui,
        entries: Vec<(Option<crate::commands::MenuGroup>, Command)>,
        reveal_selection: bool,
        choice: &mut Option<Command>,
    ) {
        let muted = crate::theme::Tokens::new(ui.visuals().dark_mode).muted;
        crate::toolbar::neutral_selection(ui);
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .animated(false)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let mut heading = None;
                        for (index, (group, command)) in entries.into_iter().enumerate() {
                            if group != heading {
                                heading = group;
                                if let Some(group) = group {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(group.label().to_uppercase())
                                            .size(11.0)
                                            .color(muted),
                                    );
                                }
                            }
                            let label = self.ui_command_label(command);
                            let button = match command.icon() {
                                Some(icon) => {
                                    egui::Button::new((icon.image(ui.ctx(), 16.0), label))
                                        .image_tint_follows_text_color(true)
                                }
                                None => egui::Button::new(label),
                            }
                            .frame_when_inactive(false)
                            .selected(index == self.dialogs.palette_index)
                            .min_size(egui::vec2(ui.available_width(), 30.0))
                            .shortcut_text(self.keymap.shortcut_text(ui.ctx(), command));
                            let response = ui.add(button);
                            // Suivre le clavier sans ramener la liste après un défilement manuel.
                            if reveal_selection && index == self.dialogs.palette_index {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() {
                                *choice = Some(command);
                            }
                        }
                    });
            });
    }
    fn export_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui_kit::{ButtonKind, Dialog, button, segmented, text_field};
        if self.file.export_open {
            let mut export = false;
            let mut cancel = false;
            let title = crate::tr!("export-title");
            let formats = [
                (crate::engine::ExportFormat::Pdf, "export-pdf"),
                (crate::engine::ExportFormat::Typst, "export-typst"),
                (crate::engine::ExportFormat::Both, "export-both"),
            ]
            .map(|(format, key)| (format, crate::i18n::text(key)));
            let modal = Dialog::new("export-document", &title).width(480.0).show(
                ctx,
                |ui| {
                    ui.label(crate::tr!("export-folder"));
                    text_field(ui, "export-folder", &mut self.file.export_path);
                    ui.label(crate::tr!("export-format"));
                    segmented(ui, &mut self.file.export_format, &formats);
                    ui.checkbox(
                        &mut self.file.export_page_images,
                        crate::tr!("export-page-images"),
                    );
                    ui.weak(crate::tr!("export-help"));
                },
                |ui| {
                    export = button(ui, ButtonKind::Primary, crate::tr!("export-button")).clicked();
                    cancel = button(ui, ButtonKind::Ghost, crate::tr!("cancel")).clicked();
                },
            );
            if cancel || modal.should_close() {
                self.file.export_open = false;
            }
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
