use super::*;

impl App {
    pub fn capture_spelling(&mut self, ctx: &egui::Context) {
        self.load_demo();
        let before = self.snapshot();
        self.session.metadata.options_mut().language = DocumentLanguage::French;
        self.session.metadata.options_mut().spelling_ignored.clear();
        self.session.document = Document::from_source("Madame, Monsieur,\n\nNous vous adressons ce *courrrier* pour confirmer la _réservvation_ de la salle.\n\nLa réunion aura lieu aujourd’hui à l’école, lors du rendez-vous prévu avec les associations.\n\nNous vous remercions de votre attention.").unwrap();
        self.remember(before, false);
        self.preferences = Preferences::default();
        self.preferences.appearance = Appearance::Light;
        ctx.set_theme(egui::ThemePreference::Light);
        self.capture_spelling = true;
    }
    pub(super) fn refresh_spelling(&mut self, ctx: &egui::Context) {
        if !self.document_open {
            return;
        }
        let options = self.session.metadata.options();
        self.spelling.synchronize(
            self.session.revision,
            &self.session.document.view.text,
            options.language,
            crate::spelling::accepted(
                options.language,
                &self.preferences.spelling.personal,
                &options.spelling_ignored,
            ),
            self.preferences.spelling.enabled,
            Instant::now(),
        );
        if let Some(delay) = self.spelling.next_wake(Instant::now()) {
            ctx.request_repaint_after(delay);
        }
    }
    pub(super) fn selected_spelling_issue(&self) -> Option<crate::spelling::Issue> {
        if !self.preferences.spelling.enabled || !self.spelling.is_current(self.session.revision) {
            return None;
        }
        let range = self.range();
        self.spelling
            .issues
            .iter()
            .find(|issue| {
                issue.range.contains(&range.start)
                    || (range.is_empty() && range.start == issue.range.end)
            })
            .cloned()
    }
    pub(super) fn valid_spelling_target(
        &self,
        revision: u64,
        issue: &crate::spelling::Issue,
    ) -> bool {
        revision == self.session.revision
            && self.preferences.spelling.enabled
            && self.spelling.is_current(revision)
            && self.spelling.issues.contains(issue)
            && self
                .session
                .document
                .view
                .text
                .chars()
                .skip(issue.range.start)
                .take(issue.range.len())
                .collect::<String>()
                == issue.word
    }
    pub(super) fn correct_spelling(
        &mut self,
        revision: u64,
        issue: &crate::spelling::Issue,
        replacement: &str,
    ) {
        if !self.valid_spelling_target(revision, issue) {
            return;
        }
        self.history_field = None;
        self.selection = self
            .session
            .replace_ranges(
                self.selection.into(),
                std::slice::from_ref(&issue.range),
                replacement,
            )
            .into();
        self.force_selection = true;
        self.forward_pending = true;
        self.spelling_target = None;
    }
    pub(super) fn open_spelling_target(&mut self, issue: crate::spelling::Issue) {
        self.spelling
            .request_suggestions(self.session.metadata.options().language, &issue.word);
        self.selection = editor::cursor(issue.range.clone());
        self.force_selection = true;
        self.forward_pending = true;
        self.history_field = None;
        self.session.typing_group = false;
        self.spelling_target = Some((self.session.revision, issue));
    }
    pub(super) fn spelling_context_menu(&mut self, output: &egui::text_edit::TextEditOutput) {
        if output.response.secondary_clicked() {
            self.spelling_target = None;
            self.spelling_menu_position = None;
            if self.spelling.is_current(self.session.revision)
                && let Some(pointer) = output.response.interact_pointer_pos()
            {
                let ranges: Vec<_> = self
                    .spelling
                    .issues
                    .iter()
                    .map(|issue| issue.range.clone())
                    .collect();
                if let Some((index, _)) =
                    editor::range_rects(&output.galley, &ranges)
                        .iter()
                        .find(|(_, rect)| {
                            rect.translate(output.galley_pos.to_vec2())
                                .contains(pointer)
                        })
                {
                    let issue = self.spelling.issues[*index].clone();
                    self.open_spelling_target(issue);
                }
            }
        }
        let mut popup = egui::Popup::context_menu(&output.response);
        if self.spelling_menu_pending {
            self.spelling_menu_pending = false;
            if let Some((_, issue)) = &self.spelling_target {
                self.spelling_menu_position = Some(
                    output.galley_pos
                        + output
                            .galley
                            .pos_from_cursor(CCursor::new(issue.range.start))
                            .left_bottom()
                            .to_vec2(),
                );
                egui::Popup::open_id(&output.response.ctx, popup.get_id());
            }
        }
        if let Some(position) = self.spelling_menu_position {
            popup = popup.at_position(position);
        }
        popup.show(|ui| {
            let Some((revision, issue)) = self
                .spelling_target
                .clone()
                .filter(|(revision, issue)| self.valid_spelling_target(*revision, issue))
            else {
                ui.weak(crate::tr!("spelling-no-issue-here"));
                return;
            };
            ui.label(egui::RichText::new(&issue.word).strong());
            if let Some((_, Some(suggestions))) = &self.spelling.suggestions {
                let suggestions = suggestions.clone();
                if suggestions.is_empty() {
                    ui.weak(crate::tr!("spelling-no-suggestions"));
                }
                for suggestion in suggestions {
                    if ui.button(&suggestion).clicked() {
                        self.correct_spelling(revision, &issue, &suggestion);
                        ui.close();
                    }
                }
            } else {
                ui.weak(crate::tr!("spelling-loading-suggestions"));
            }
            ui.separator();
            for command in [Command::IgnoreSpelling, Command::LearnSpelling] {
                if ui.button(command.label()).clicked() {
                    self.apply(command);
                    self.spelling_target = None;
                    self.force_selection = true;
                    ui.close();
                }
            }
        });
    }
}
