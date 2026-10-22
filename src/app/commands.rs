use super::*;

impl App {
    pub(super) fn snapshot(&self) -> Snapshot {
        self.document
            .engine
            .snapshot(self.document.selection.into())
    }
    pub(super) fn range(&self) -> Range<usize> {
        let range = self.document.selection.as_sorted_char_range();
        range.start.min(self.document.engine.document.view.len())
            ..range.end.min(self.document.engine.document.view.len())
    }
    pub(super) fn remember(&mut self, before: Snapshot, typing: bool) {
        let previous = before.selection;
        let result = self
            .document
            .engine
            .remember(before, self.document.selection.into(), typing);
        self.accept_edit(result, previous);
    }
    pub(super) fn accept_edit(
        &mut self,
        result: Result<crate::session::Selection, crate::document::edits::EditError>,
        previous: crate::session::Selection,
    ) -> bool {
        let adjusted = match result {
            Ok(selection) => selection,
            Err(error) => {
                self.document.selection = previous.into();
                self.document.force_selection = true;
                self.dialogs.message = Some(Self::edit_error_text(error));
                return false;
            }
        };
        self.document.force_selection |= adjusted != self.document.selection.into();
        self.document.selection = adjusted.into();
        true
    }
    pub(super) fn edit_error_text(error: crate::document::edits::EditError) -> String {
        crate::i18n::text(match error {
            crate::document::edits::EditError::ProtectedBoundary => "edit-protected-boundary",
            crate::document::edits::EditError::InvalidText => "edit-invalid-text",
            crate::document::edits::EditError::StaleDraft
            | crate::document::edits::EditError::Target(_) => "edit-stale-draft",
            _ => "edit-invalid-draft",
        })
    }
    pub(super) fn apply(&mut self, command: Command) {
        if !self.keymap.registry().contains(command)
            || (!self.document.open && !command.available_without_document())
        {
            return;
        }
        self.cancel_ime();
        if let Some(core) = command.document_command() {
            self.apply_document_command(command, core);
            return;
        }
        match command {
            Command::ToggleSpelling
            | Command::SpellingSuggestions
            | Command::NextSpelling
            | Command::PreviousSpelling
            | Command::IgnoreSpelling
            | Command::LearnSpelling
            | Command::ResetSpelling => self.apply_spelling_command(command),
            Command::Find | Command::FindReplace => {
                self.search.open = true;
                self.search.replace_open |= command == Command::FindReplace;
                self.search.focus_pending = true;
            }
            Command::FindNext | Command::FindPrevious => {
                self.search.open = true;
                self.refresh_search();
                if let Some(index) = self
                    .search
                    .engine
                    .next(self.range(), command == Command::FindPrevious)
                {
                    self.select_search_result(index);
                }
            }
            Command::ReplaceCurrent | Command::ReplaceAll => {
                self.replace_search(command == Command::ReplaceAll)
            }
            Command::RecoverDocuments => self.recovery.open = !self.recovery.entries.is_empty(),
            Command::ImportModel => self.import_model(),
            Command::InsertImage => self.choose_image(None),
            Command::Preferences => self.dialogs.preferences_open = true,
            Command::File(command) => self.file.command = Some(command),
            Command::Palette => {
                self.dialogs.palette_open = !self.dialogs.palette_open;
                self.dialogs.palette_query.clear();
                self.dialogs.palette_index = 0;
            }
            Command::Information => {
                self.dialogs.information_open = !self.dialogs.information_open;
                if !self.dialogs.information_open {
                    self.document.history_field = None;
                    self.document.force_selection = true;
                }
            }
            command @ Command::Model(_) => {
                let template = command.model_source().expect("commande de modèle");
                if self.document.engine.template != template {
                    let before = self.snapshot();
                    self.document.engine.dependencies = crate::packages::for_template(&template);
                    self.document.engine.template = template;
                    self.document.presentation = ModelPresentation::default();
                    self.document.engine.metadata.options_mut().bullet_marker = None;
                    if let Command::Model(model) = command
                        && let Some((paper, landscape)) = model.definition().page
                    {
                        self.document.engine.metadata.options_mut().paper = paper;
                        self.document.engine.metadata.options_mut().landscape = landscape;
                    }
                    self.remember(before, false);
                }
            }
            Command::Link => self.open_link(),
            _ => unreachable!("document command dispatched through Engine"),
        }
    }
    fn apply_spelling_command(&mut self, command: Command) {
        match command {
            Command::ToggleSpelling => {
                self.preferences.spelling.enabled = !self.preferences.spelling.enabled
            }
            Command::SpellingSuggestions => {
                if let Some(issue) = self.selected_spelling_issue() {
                    self.open_spelling_target(issue);
                    self.spelling.menu_pending = true;
                }
            }
            Command::NextSpelling | Command::PreviousSpelling => {
                let range = self.range();
                let issues = &self.spelling.checker.issues;
                let target = if command == Command::PreviousSpelling {
                    issues
                        .iter()
                        .rfind(|issue| issue.range.start < range.start)
                        .or_else(|| issues.last())
                } else {
                    issues
                        .iter()
                        .find(|issue| issue.range.start >= range.end)
                        .or_else(|| issues.first())
                }
                .cloned();
                if let Some(issue) = target {
                    self.focus_annotation_target(issue.range);
                }
            }
            Command::IgnoreSpelling | Command::LearnSpelling => {
                if let Some(issue) = self.selected_spelling_issue() {
                    let language = self
                        .document
                        .engine
                        .metadata
                        .options()
                        .language
                        .code()
                        .to_owned();
                    let word = crate::spelling::word_key(&issue.word);
                    if command == Command::LearnSpelling {
                        self.preferences
                            .spelling
                            .personal
                            .entry(language)
                            .or_default()
                            .insert(word);
                    } else {
                        let before = self.snapshot();
                        self.document
                            .engine
                            .metadata
                            .options_mut()
                            .spelling_ignored
                            .entry(language)
                            .or_default()
                            .insert(word);
                        self.remember(before, false);
                    }
                }
            }
            Command::ResetSpelling => {
                let before = self.snapshot();
                self.document
                    .engine
                    .metadata
                    .options_mut()
                    .spelling_ignored
                    .clear();
                self.remember(before, false);
            }
            _ => unreachable!("spelling command"),
        }
    }
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.modal_open() {
            return;
        }
        if self.search.open
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.close_search();
            return;
        }
        let focused = ctx.memory(|memory| memory.has_focus(egui::Id::new(editor::EDITOR_ID)));
        #[cfg(target_os = "macos")]
        let reserved = self
            .native_menus
            .as_ref()
            .map_or_else(Vec::new, |menus| menus.shortcuts());
        #[cfg(not(target_os = "macos"))]
        let reserved = Vec::new();
        let commands = ctx.input_mut(|input| self.keymap.consume(input, focused, &reserved));
        for command in commands {
            if self.modal_open() {
                break;
            }
            self.apply(command);
        }
        if self.document.open && focused && !self.modal_open() {
            self.text_shortcuts(ctx);
        }
    }
    pub(super) fn track_history_focus(&mut self, ctx: &egui::Context) {
        // La recherche de la palette et les champs des dialogues ne doivent pas
        // remplacer la cible de l’historique du document.
        if self.modal_open() {
            return;
        }
        if let Some(id) = ctx.memory(|memory| memory.focused()) {
            if id == egui::Id::new(editor::EDITOR_ID)
                || [SEARCH_ID, REPLACE_ID]
                    .iter()
                    .any(|search_id| id == egui::Id::new(search_id))
            {
                self.document.history_field = None;
            } else if egui::TextEdit::load_state(ctx, id).is_some() {
                self.document.history_field = Some(id);
            }
        }
        if !self.dialogs.information_open && !self.modal_open() {
            self.document.history_field = None;
        }
    }
    pub(super) fn menu_statuses(&self) -> Vec<(Command, menus::Status)> {
        self.keymap
            .registry()
            .definitions()
            .filter(|d| d.menu_position().is_some())
            .map(|d| {
                let enabled = !self.modal_open()
                    && (self.document.open || d.command.available_without_document())
                    && d.command.document_command().is_none_or(|command| {
                        matches!(
                            self.document.engine.available(&command),
                            crate::engine::Availability::Enabled
                        )
                    })
                    && match d.command {
                        Command::Clear => self.document.history_field.is_none(),
                        Command::RecoverDocuments => !self.recovery.entries.is_empty(),
                        Command::FindNext | Command::FindPrevious => {
                            !self.search.engine.matches.is_empty()
                        }
                        Command::NextSpelling | Command::PreviousSpelling => {
                            !self.spelling.checker.issues.is_empty()
                        }
                        Command::IgnoreSpelling
                        | Command::LearnSpelling
                        | Command::SpellingSuggestions => self.selected_spelling_issue().is_some(),
                        Command::ResetSpelling => !self
                            .document
                            .engine
                            .metadata
                            .options()
                            .spelling_ignored
                            .is_empty(),
                        _ => true,
                    };
                (
                    d.command,
                    menus::Status {
                        enabled,
                        checked: (d.command == Command::Information
                            && self.dialogs.information_open)
                            || (d.command == Command::ToggleSpelling
                                && self.preferences.spelling.enabled),
                    },
                )
            })
            .collect()
    }
    pub(super) fn dispatch_action(&mut self, command: Command, ctx: &egui::Context) {
        if self.modal_open() {
            return;
        }
        self.apply(command);
        ctx.request_repaint();
    }
    #[cfg(target_os = "macos")]
    pub(super) fn refresh_native_menus(&mut self, ctx: &egui::Context) {
        if self
            .native_menus
            .as_ref()
            .is_some_and(|menus| menus.document_type() != self.keymap.registry().document_type())
        {
            // Retirer les anciens menus avant d’installer ceux du type actif.
            self.native_menus.take();
            match menus::native::NativeMenus::new(ctx.clone(), &self.keymap) {
                Ok(menus) => self.native_menus = Some(menus),
                Err(error) => {
                    self.file.error =
                        Some(crate::tr!("native-menu-error", error = error.to_string()))
                }
            }
        }
        let statuses = self.menu_statuses();
        if let Some(menus) = &mut self.native_menus
            && let Err(error) = menus.update(&self.keymap, &statuses)
        {
            self.file.error = Some(crate::tr!("native-menu-error", error = error.to_string()));
            ctx.request_repaint();
        }
    }
}
