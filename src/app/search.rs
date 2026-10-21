use super::*;

impl App {
    pub(super) fn refresh_search(&mut self) {
        self.search
            .refresh(self.session.revision, &self.session.document.view);
    }
    pub fn capture_search(&mut self, query: String) {
        self.load_demo();
        self.apply(Command::FindReplace);
        self.search.query = query;
        self.apply(Command::FindNext);
    }
    pub(super) fn close_search(&mut self) {
        self.search_open = false;
        self.search_focus_pending = false;
        self.force_selection = true;
    }
    pub(super) fn search_focus(&self, ctx: &egui::Context) -> Option<egui::Id> {
        ctx.memory(|memory| memory.focused()).filter(|id| {
            self.search_open
                && [SEARCH_ID, REPLACE_ID]
                    .iter()
                    .any(|name| *id == egui::Id::new(name))
        })
    }
    pub(super) fn select_search_result(&mut self, index: usize) {
        if let Some(range) = self.search.matches.get(index) {
            // Une seule sélection native. Les autres résultats ne sont que des fonds colorés.
            self.selection = editor::cursor(range.clone());
            self.session.document.view.typing_style =
                self.session.document.view.style_at(range.start);
            self.force_selection = true;
            self.editor_scroll_pending = true;
            self.forward_pending = true;
            self.session.typing_group = false;
        }
    }
    pub(super) fn replace_search(&mut self, all: bool) {
        self.search_open = true;
        self.replace_open = true;
        self.refresh_search();
        if !self.search.replacement_valid() {
            return;
        }
        let ranges = if all {
            self.search.matches.clone()
        } else {
            self.search
                .selected(&self.range())
                .map(|index| vec![self.search.matches[index].clone()])
                .unwrap_or_default()
        };
        if ranges.is_empty() {
            return;
        }
        self.selection = self
            .session
            .replace_ranges(self.selection.into(), &ranges, &self.search.replacement)
            .into();
        self.force_selection = true;
        self.forward_pending = true;
        self.refresh_search();
        if !all && let Some(index) = self.search.next(self.range(), false) {
            self.select_search_result(index);
        }
        self.message = Some(crate::tr!("search-replaced", count = ranges.len()));
    }
    pub(super) fn search_ui(&mut self, ui: &mut egui::Ui) {
        if !self.search_open {
            return;
        }
        self.refresh_search();
        // Consommer Entrée avant le TextEdit singleline évite de perdre le focus.
        let enter = self
            .search_focus(ui.ctx())
            .filter(|_| ui.is_enabled())
            .and_then(|id| {
                if ui.input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter))
                {
                    Some((id, true))
                } else if ui
                    .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
                {
                    Some((id, false))
                } else {
                    None
                }
            });
        let mut action = None;
        let mut close = false;
        let chevron = |ui: &egui::Ui, rotation| {
            egui::Button::new(
                crate::icons::Icon::ChevronDown
                    .image(ui.ctx(), 12.0)
                    .rotate(rotation, egui::vec2(0.5, 0.5)),
            )
            .image_tint_follows_text_color(true)
            .small()
        };
        ui.horizontal_wrapped(|ui| {
            let toggle = ui
                .add(chevron(
                    ui,
                    if self.replace_open {
                        0.0
                    } else {
                        -std::f32::consts::FRAC_PI_2
                    },
                ))
                .on_hover_text(crate::tr!("search-toggle-replace"));
            toggle.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    ui.is_enabled(),
                    crate::tr!("search-toggle-replace"),
                )
            });
            if toggle.clicked() {
                self.replace_open = !self.replace_open;
            }
            let width = (ui.available_width() - 260.0).max(60.0);
            let mut output = egui::TextEdit::singleline(&mut self.search.query)
                .id(egui::Id::new(SEARCH_ID))
                .hint_text(crate::tr!("search-placeholder"))
                .desired_width(width)
                .show(ui);
            if self.search_focus_pending && ui.is_enabled() {
                output.response.request_focus();
                output
                    .state
                    .cursor
                    .set_char_range(Some(editor::cursor(0..self.search.query.chars().count())));
                output.state.store(ui.ctx(), output.response.id);
                self.search_focus_pending = false;
            }
            self.refresh_search();
            let counter = match self.search.selected(&self.range()) {
                Some(index) => crate::tr!(
                    "search-active-result",
                    current = index + 1,
                    total = self.search.matches.len()
                ),
                None => crate::tr!("search-results", count = self.search.matches.len()),
            };
            // Largeur stable : choisir un résultat ne doit pas déplacer le texte
            // sous le pointeur en repliant la barre sur une ligne supplémentaire.
            ui.add_sized(
                [90.0, ui.spacing().interact_size.y],
                egui::Label::new(counter).truncate(),
            );
            let has_matches = !self.search.matches.is_empty();
            for (rotation, command) in [
                (std::f32::consts::PI, Command::FindPrevious),
                (0.0, Command::FindNext),
            ] {
                let response = ui
                    .add_enabled(has_matches, chevron(ui, rotation))
                    .on_hover_text(self.keymap.hint(ui.ctx(), command));
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        has_matches && ui.is_enabled(),
                        command.label(),
                    )
                });
                if response.clicked() {
                    action = Some(command);
                }
            }
            let case = ui
                .selectable_label(self.search.case_sensitive, "Aa")
                .on_hover_text(crate::tr!("search-match-case"));
            case.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Checkbox,
                    ui.is_enabled(),
                    self.search.case_sensitive,
                    crate::tr!("search-match-case"),
                )
            });
            if case.clicked() {
                self.search.case_sensitive = !self.search.case_sensitive;
                self.refresh_search();
            }
            close = ui
                .small_button("×")
                .on_hover_text(crate::tr!("search-close"))
                .clicked();
        });
        if self.replace_open {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(20.0);
                let width = (ui.available_width() - 230.0).max(80.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search.replacement)
                        .id(egui::Id::new(REPLACE_ID))
                        .hint_text(crate::tr!("search-replacement-placeholder"))
                        .desired_width(width),
                );
                let valid = self.search.replacement_valid();
                for (command, enabled) in [
                    (
                        Command::ReplaceCurrent,
                        self.search.selected(&self.range()).is_some(),
                    ),
                    (Command::ReplaceAll, !self.search.matches.is_empty()),
                ] {
                    if ui
                        .add_enabled(valid && enabled, egui::Button::new(command.label()).small())
                        .clicked()
                    {
                        action = Some(command);
                    }
                }
            });
            if !self.search.replacement_valid() {
                ui.colored_label(ui.visuals().error_fg_color, crate::tr!("search-one-line"));
            }
        }
        if self.search.failed {
            ui.colored_label(ui.visuals().error_fg_color, crate::tr!("search-too-large"));
        }
        if let Some((id, backward)) = enter {
            action = Some(if id == egui::Id::new(REPLACE_ID) && !backward {
                Command::ReplaceCurrent
            } else if backward {
                Command::FindPrevious
            } else {
                Command::FindNext
            });
        }
        if let Some(command) = action {
            self.apply(command);
            let id = if matches!(command, Command::ReplaceCurrent | Command::ReplaceAll) {
                REPLACE_ID
            } else {
                SEARCH_ID
            };
            ui.memory_mut(|memory| memory.request_focus(egui::Id::new(id)));
            ui.ctx().request_repaint();
        }
        if close {
            self.close_search();
        }
        ui.separator();
    }
}
