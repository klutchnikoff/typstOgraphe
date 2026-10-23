use super::*;
use crate::engine::{
    Availability, Command as Core, SelectTextParams, SpellingCorrectionParams, SpellingIssue,
    SpellingIssueParams,
};

impl App {
    pub fn capture_spelling(&mut self, ctx: &egui::Context) {
        self.load_demo();
        let before = self.snapshot();
        self.document.engine.metadata.options_mut().language = DocumentLanguage::French;
        self.document
            .engine
            .metadata
            .options_mut()
            .spelling_ignored
            .clear();
        self.document.engine.document = Document::from_source("Madame, Monsieur,\n\nNous vous adressons ce *courrrier* pour confirmer la _réservvation_ de la salle.\n\nLa réunion aura lieu aujourd’hui à l’école, lors du rendez-vous prévu avec les associations.\n\nNous vous remercions de votre attention.").unwrap();
        self.remember(before, false);
        self.preferences = Preferences::default();
        self.preferences.appearance = Appearance::Light;
        ctx.set_theme(egui::ThemePreference::Light);
        self.spelling.capture_active = true;
    }
    pub(super) fn refresh_spelling(&mut self, ctx: &egui::Context) {
        if !self.document.open {
            return;
        }
        self.document
            .engine
            .configure_spelling(&self.preferences.spelling.personal);
        let options = self.document.engine.metadata.options();
        self.spelling.checker.synchronize(
            self.document.engine.revision,
            &self.document.engine.document.view.text,
            options.language,
            crate::spelling::accepted(
                options.language,
                &self.preferences.spelling.personal,
                &options.spelling_ignored,
            ),
            self.preferences.spelling.enabled,
            Instant::now(),
        );
        if let Some(delay) = self.spelling.checker.next_wake(Instant::now()) {
            ctx.request_repaint_after(delay);
        }
    }
    pub(super) fn selected_spelling_issue(&self) -> Option<crate::spelling::Issue> {
        if !self.preferences.spelling.enabled
            || !self
                .spelling
                .checker
                .is_current(self.document.engine.revision)
        {
            return None;
        }
        let range = self.range();
        self.spelling
            .checker
            .issues
            .iter()
            .find(|issue| {
                issue.range.contains(&range.start)
                    || (range.is_empty() && range.start == issue.range.end)
            })
            .cloned()
    }
    pub(super) fn spelling_reference(
        &self,
        issue: &crate::spelling::Issue,
    ) -> Option<SpellingIssue> {
        if !self.preferences.spelling.enabled
            || !self
                .spelling
                .checker
                .is_current(self.document.engine.revision)
        {
            return None;
        }
        let target = self
            .document
            .engine
            .selection_target(editor::cursor(issue.range.clone()).into())
            .ok()?;
        let reference = SpellingIssue {
            target,
            language: self.document.engine.metadata.options().language,
            word: issue.word.clone(),
        };
        self.valid_spelling_target(&reference).then_some(reference)
    }
    pub(super) fn valid_spelling_target(&self, issue: &SpellingIssue) -> bool {
        self.preferences.spelling.enabled
            && self
                .spelling
                .checker
                .is_current(issue.target.guard.revision)
            && matches!(
                self.document
                    .engine
                    .available(&Core::IgnoreSpelling(SpellingIssueParams {
                        issue: issue.clone()
                    })),
                Availability::Enabled
            )
    }
    pub(super) fn correct_spelling(&mut self, issue: &SpellingIssue, replacement: &str) {
        self.document.history_field = None;
        if self.run_document_command(Core::CorrectSpelling(SpellingCorrectionParams {
            issue: issue.clone(),
            replacement: replacement.into(),
        })) {
            self.spelling.target = None;
        }
    }
    /// Le menu capture une cible ; le raccourci utilise la faute sélectionnée.
    pub(super) fn apply_spelling_issue(&mut self, command: Command, issue: SpellingIssue) {
        match command {
            Command::IgnoreSpelling => {
                if self.run_document_command(Core::IgnoreSpelling(SpellingIssueParams { issue })) {
                    self.spelling.target = None;
                }
            }
            Command::LearnSpelling if self.valid_spelling_target(&issue) => {
                self.preferences
                    .spelling
                    .personal
                    .entry(issue.language.code().into())
                    .or_default()
                    .insert(crate::spelling::word_key(&issue.word));
                self.document
                    .engine
                    .configure_spelling(&self.preferences.spelling.personal);
            }
            Command::LearnSpelling => {}
            _ => unreachable!("spelling issue action"),
        }
    }
    pub(super) fn open_spelling_target(&mut self, issue: crate::spelling::Issue) {
        let Some(reference) = self.spelling_reference(&issue) else {
            return;
        };
        if !self.run_document_command(Core::SelectText(SelectTextParams {
            target: reference.target.into(),
            part: None,
        })) {
            return;
        }
        // Le worker partage le calcul pur de spelling.suggestions ; il évite
        // de bloquer la frame egui. La cible capturée sera revalidée au clic.
        self.spelling
            .checker
            .request_suggestions(reference.language, &reference.word);
        self.document.history_field = None;
        self.spelling.target = Some(reference);
    }
    pub(super) fn spelling_context_menu(&mut self, output: &egui::text_edit::TextEditOutput) {
        if output.response.secondary_clicked() {
            self.spelling.target = None;
            self.spelling.menu_position = None;
            if self
                .spelling
                .checker
                .is_current(self.document.engine.revision)
                && let Some(pointer) = output.response.interact_pointer_pos()
            {
                let ranges: Vec<_> = self
                    .spelling
                    .checker
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
                    let issue = self.spelling.checker.issues[*index].clone();
                    self.open_spelling_target(issue);
                }
            }
        }
        let mut popup = egui::Popup::context_menu(&output.response);
        if self.spelling.menu_pending {
            self.spelling.menu_pending = false;
            if let Some(issue) = &self.spelling.target {
                let Ok((start, _)) = self.document.engine.resolve_target(issue.target) else {
                    return;
                };
                self.spelling.menu_position = Some(
                    output.galley_pos
                        + output
                            .galley
                            .pos_from_cursor(CCursor::new(start))
                            .left_bottom()
                            .to_vec2(),
                );
                egui::Popup::open_id(&output.response.ctx, popup.get_id());
            }
        }
        if let Some(position) = self.spelling.menu_position {
            popup = popup.at_position(position);
        }
        popup.show(|ui| {
            let Some(issue) = self
                .spelling
                .target
                .clone()
                .filter(|issue| self.valid_spelling_target(issue))
            else {
                ui.weak(crate::tr!("spelling-no-issue-here"));
                return;
            };
            ui.label(egui::RichText::new(&issue.word).strong());
            if let Some((_, Some(suggestions))) = &self.spelling.checker.suggestions {
                let suggestions = suggestions.clone();
                if suggestions.is_empty() {
                    ui.weak(crate::tr!("spelling-no-suggestions"));
                }
                for suggestion in suggestions {
                    if ui.button(&suggestion).clicked() {
                        self.correct_spelling(&issue, &suggestion);
                        ui.close();
                    }
                }
            } else {
                ui.weak(crate::tr!("spelling-loading-suggestions"));
            }
            ui.separator();
            for command in [Command::IgnoreSpelling, Command::LearnSpelling] {
                if ui.button(command.label()).clicked() {
                    self.apply_spelling_issue(command, issue.clone());
                    self.spelling.target = None;
                    self.document.force_selection = true;
                    ui.close();
                }
            }
        });
    }
}
