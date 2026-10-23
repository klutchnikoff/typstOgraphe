use super::*;
use crate::engine::{
    Command as CoreCommand, FindParams, ReplaceAllParams, ReplaceCurrentParams, SearchParams,
    SearchResults, SelectTextParams,
};

impl App {
    pub(super) fn refresh_search(&mut self) {
        let guard = self.document.engine.target_guard();
        if self.search.guard == Some(guard) && self.search.engine.is_current(guard.revision) {
            return;
        }
        let query = crate::engine::Query::Find(SearchParams {
            query: self.search.engine.query.clone(),
            case_sensitive: self.search.engine.case_sensitive,
        });
        let results = self.document.engine.query_call(query.to_call());
        self.search.guard = Some(guard);
        self.search.occurrences.clear();
        let mut ranges = Vec::new();
        let failed = match results {
            Ok(value) => {
                let results: SearchResults =
                    serde_json::from_value(value).expect("typed search results");
                for occurrence in results.matches {
                    let (start, end) = self
                        .document
                        .engine
                        .resolve_target(occurrence.target)
                        .expect("current search target");
                    ranges.push(start.min(end)..start.max(end));
                    self.search.occurrences.push(occurrence);
                }
                false
            }
            Err(_) => true,
        };
        self.search
            .engine
            .cache_results(guard.revision, ranges, failed);
    }

    /// Diagnostic de présentation utilisant les mêmes commandes que l’interface.
    pub fn capture_search(&mut self, query: String) {
        self.load_demo();
        self.apply(Command::FindReplace);
        self.search.engine.query = query;
        self.apply(Command::FindNext);
    }
    pub(super) fn close_search(&mut self) {
        self.search.open = false;
        self.search.focus_pending = false;
        self.document.force_selection = true;
    }
    pub(super) fn search_focus(&self, ctx: &egui::Context) -> Option<egui::Id> {
        ctx.memory(|memory| memory.focused()).filter(|id| {
            self.search.open
                && [SEARCH_ID, REPLACE_ID]
                    .iter()
                    .any(|name| *id == egui::Id::new(name))
        })
    }
    pub(super) fn select_search_result(&mut self, index: usize) {
        if let Some(occurrence) = self.search.occurrences.get(index).cloned()
            && self.run_document_command(CoreCommand::SelectText(SelectTextParams {
                target: occurrence.target.into(),
                part: None,
            }))
        {
            self.preview.editor_scroll_pending = true;
        }
    }
    pub(super) fn navigate_search(&mut self, backward: bool) {
        self.search.open = true;
        self.refresh_search();
        if self.search.engine.matches.is_empty() {
            return;
        }
        let params = FindParams {
            query: self.search.engine.query.clone(),
            case_sensitive: self.search.engine.case_sensitive,
            target: Default::default(),
        };
        self.run_document_command(if backward {
            CoreCommand::FindPrevious(params)
        } else {
            CoreCommand::FindNext(params)
        });
    }
    pub(super) fn replace_search(&mut self, all: bool) {
        self.search.open = true;
        self.search.replace_open = true;
        self.refresh_search();
        if !self.search.engine.replacement_valid() || self.search.engine.matches.is_empty() {
            return;
        }
        let query = self.search.engine.query.clone();
        let case_sensitive = self.search.engine.case_sensitive;
        let replacement = self.search.engine.replacement.clone();
        let command = if all {
            CoreCommand::ReplaceAll(ReplaceAllParams {
                guard: self.search.guard.expect("refreshed search"),
                query,
                case_sensitive,
                replacement,
            })
        } else {
            let Some(index) = self.search.engine.selected(&self.range()) else {
                return;
            };
            CoreCommand::ReplaceCurrent(ReplaceCurrentParams {
                query,
                case_sensitive,
                replacement,
                occurrence: self.search.occurrences[index].clone(),
            })
        };
        match self.execute_document_command(command) {
            Ok(outcome) => {
                let count = outcome.value.expect("replacement result")["count"]
                    .as_u64()
                    .unwrap();
                self.dialogs.message = Some(crate::tr!("search-replaced", count = count));
            }
            Err(error) => {
                self.document.force_selection = true;
                self.dialogs.message = Some(Self::command_error_text(&error));
            }
        }
    }
    pub(super) fn search_ui(&mut self, ui: &mut egui::Ui) {
        if !self.search.open {
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
        let (mut action, close) = self.search_query_row(ui);
        if let Some(command) = self.search_replacement_row(ui) {
            action = Some(command);
        }
        if self.search.engine.failed {
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
    fn search_query_row(&mut self, ui: &mut egui::Ui) -> (Option<Command>, bool) {
        let mut action = None;
        let mut close = false;
        ui.horizontal_wrapped(|ui| {
            let toggle = ui
                .add(search_chevron(
                    ui,
                    if self.search.replace_open {
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
                self.search.replace_open = !self.search.replace_open;
            }
            let width = (ui.available_width() - 260.0).max(60.0);
            let mut output = egui::TextEdit::singleline(&mut self.search.engine.query)
                .id(egui::Id::new(SEARCH_ID))
                .hint_text(crate::tr!("search-placeholder"))
                .desired_width(width)
                .show(ui);
            if self.search.focus_pending && ui.is_enabled() {
                output.response.request_focus();
                output.state.cursor.set_char_range(Some(editor::cursor(
                    0..self.search.engine.query.chars().count(),
                )));
                output.state.store(ui.ctx(), output.response.id);
                self.search.focus_pending = false;
            }
            self.refresh_search();
            self.search_result_controls(ui, &mut action);
            close = ui
                .small_button("×")
                .on_hover_text(crate::tr!("search-close"))
                .clicked();
        });
        (action, close)
    }
    fn search_result_controls(&mut self, ui: &mut egui::Ui, action: &mut Option<Command>) {
        let counter = match self.search.engine.selected(&self.range()) {
            Some(index) => crate::tr!(
                "search-active-result",
                current = index + 1,
                total = self.search.engine.matches.len()
            ),
            None => crate::tr!("search-results", count = self.search.engine.matches.len()),
        };
        // Largeur stable : choisir un résultat ne doit pas déplacer le texte
        // sous le pointeur en repliant la barre sur une ligne supplémentaire.
        ui.add_sized(
            [90.0, ui.spacing().interact_size.y],
            egui::Label::new(counter).truncate(),
        );
        let has_matches = !self.search.engine.matches.is_empty();
        for (rotation, command) in [
            (std::f32::consts::PI, Command::FindPrevious),
            (0.0, Command::FindNext),
        ] {
            let response = ui
                .add_enabled(has_matches, search_chevron(ui, rotation))
                .on_hover_text(self.keymap.hint(ui.ctx(), command));
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    has_matches && ui.is_enabled(),
                    command.label(),
                )
            });
            if response.clicked() {
                *action = Some(command);
            }
        }
        let case = ui
            .selectable_label(self.search.engine.case_sensitive, "Aa")
            .on_hover_text(crate::tr!("search-match-case"));
        case.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                ui.is_enabled(),
                self.search.engine.case_sensitive,
                crate::tr!("search-match-case"),
            )
        });
        if case.clicked() {
            self.search.engine.case_sensitive = !self.search.engine.case_sensitive;
            self.refresh_search();
        }
    }
    fn search_replacement_row(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let mut action = None;
        if self.search.replace_open {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(20.0);
                let width = (ui.available_width() - 230.0).max(80.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search.engine.replacement)
                        .id(egui::Id::new(REPLACE_ID))
                        .hint_text(crate::tr!("search-replacement-placeholder"))
                        .desired_width(width),
                );
                let valid = self.search.engine.replacement_valid();
                for (command, enabled) in [
                    (
                        Command::ReplaceCurrent,
                        self.search.engine.selected(&self.range()).is_some(),
                    ),
                    (Command::ReplaceAll, !self.search.engine.matches.is_empty()),
                ] {
                    if ui
                        .add_enabled(valid && enabled, egui::Button::new(command.label()).small())
                        .clicked()
                    {
                        action = Some(command);
                    }
                }
            });
            if !self.search.engine.replacement_valid() {
                ui.colored_label(ui.visuals().error_fg_color, crate::tr!("search-one-line"));
            }
        }
        action
    }
}

fn search_chevron(ui: &egui::Ui, rotation: f32) -> egui::Button<'static> {
    egui::Button::new(
        crate::icons::Icon::ChevronDown
            .image(ui.ctx(), 12.0)
            .rotate(rotation, egui::vec2(0.5, 0.5)),
    )
    .image_tint_follows_text_color(true)
    .small()
}
