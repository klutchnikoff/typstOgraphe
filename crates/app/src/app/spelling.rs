use super::*;
use crate::engine::{
    Availability, Command as Core, SelectTextParams, SpellingIssue, SpellingIssueParams,
};

impl App {
    pub fn capture_spelling(&mut self, ctx: &egui::Context) {
        // This is a complete diagnostic fixture, not native keyboard input.
        // Install it through the document loader rather than replacing a guarded
        // NativeEdit with an unrelated projection that has no recorded splices.
        let mut stored = StoredDocument::demo(self.dialogs.new_document_type);
        stored.source = "Madame, Monsieur,\n\nNous vous adressons ce *courrrier* pour confirmer la _réservvation_ de la salle.\n\nLa réunion aura lieu aujourd’hui à l’école, lors du rendez-vous prévu avec les associations.\n\nNous vous remercions de votre attention.".into();
        self.install_document(stored, None)
            .expect("valid capture fixture");
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
        let options = self.document.engine.metadata().options();
        self.spelling.checker.synchronize(
            self.document.engine.revision(),
            &self.document.engine.document().view.text,
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
    pub(super) fn spelling_reference(
        &self,
        issue: &crate::spelling::Issue,
    ) -> Option<SpellingIssue> {
        if !self.preferences.spelling.enabled
            || !self
                .spelling
                .checker
                .is_current(self.document.engine.revision())
        {
            return None;
        }
        let target = self
            .document
            .engine
            .selection_target(editor::to_core(editor::cursor(issue.range.clone())))
            .ok()?;
        let reference = SpellingIssue {
            target,
            language: self.document.engine.metadata().options().language,
            word: issue.word.clone(),
        };
        self.valid_spelling_target(&reference).then_some(reference)
    }
    pub(super) fn valid_spelling_target(&self, issue: &SpellingIssue) -> bool {
        self.preferences.spelling.enabled
            && matches!(
                self.document
                    .engine
                    .available(&Core::IgnoreSpelling(SpellingIssueParams {
                        issue: issue.clone()
                    })),
                Availability::Enabled
            )
    }
    #[cfg(test)]
    pub(super) fn correct_spelling(&mut self, issue: &SpellingIssue, replacement: &str) {
        self.document.history_field = None;
        if self.run_document_command(Core::CorrectSpelling(
            crate::engine::SpellingCorrectionParams {
                issue: issue.clone(),
                replacement: replacement.into(),
            },
        )) {
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
        self.open_spelling_reference(reference);
    }
    pub(super) fn open_spelling_reference(&mut self, reference: SpellingIssue) {
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
        self.spelling.target = Some(reference.clone());
        self.show_band(
            reference.target,
            super::bands::BandContent::Spelling { issue: reference },
        );
    }
    pub(super) fn spelling_context_menu(&mut self, output: &egui::text_edit::TextEditOutput) {
        if (output.response.secondary_clicked() || output.response.clicked())
            && self
                .spelling
                .checker
                .is_current(self.document.engine.revision())
            && let Some(pointer) = output.response.interact_pointer_pos()
        {
            let ranges: Vec<_> = self
                .spelling
                .checker
                .issues
                .iter()
                .map(|i| i.range.clone())
                .collect();
            if let Some((index, _)) =
                editor::range_rects(&output.galley, &ranges)
                    .iter()
                    .find(|(_, rect)| {
                        rect.translate(output.galley_pos.to_vec2())
                            .contains(pointer)
                    })
            {
                self.open_spelling_target(self.spelling.checker.issues[*index].clone());
            }
        }
    }
}
