use super::*;

impl App {
    pub(super) fn snapshot(&self) -> Snapshot {
        self.session.snapshot(self.selection.into())
    }
    pub(super) fn range(&self) -> Range<usize> {
        let range = self.selection.as_sorted_char_range();
        range.start.min(self.session.document.view.len())
            ..range.end.min(self.session.document.view.len())
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(super) fn changed(&mut self) {
        let adjusted = self.session.changed(self.selection.into());
        self.force_selection |= adjusted != self.selection.into();
        self.selection = adjusted.into();
    }
    pub(super) fn remember(&mut self, before: Snapshot, typing: bool) {
        let adjusted = self.session.remember(before, self.selection.into(), typing);
        self.force_selection |= adjusted != self.selection.into();
        self.selection = adjusted.into();
    }
    pub(super) fn apply(&mut self, command: Command) {
        if !self.keymap.registry().contains(command)
            || (!self.document_open && !command.available_without_document())
        {
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
                self.search_open = true;
                self.replace_open |= command == Command::FindReplace;
                self.search_focus_pending = true;
            }
            Command::FindNext | Command::FindPrevious => {
                self.search_open = true;
                self.refresh_search();
                if let Some(index) = self
                    .search
                    .next(self.range(), command == Command::FindPrevious)
                {
                    self.select_search_result(index);
                }
            }
            Command::ReplaceCurrent | Command::ReplaceAll => {
                self.replace_search(command == Command::ReplaceAll)
            }
            Command::RecoverDocuments => self.recovery_open = !self.recovery_entries.is_empty(),
            Command::ImportModel => self.import_model(),
            Command::InsertImage => self.choose_image(None),
            Command::Preferences => self.preferences_open = true,
            Command::File(command) => self.file_command = Some(command),
            Command::Palette => {
                self.palette_open = !self.palette_open;
                self.palette_query.clear();
                self.palette_index = 0;
            }
            Command::Information => {
                self.information_open = !self.information_open;
                if !self.information_open {
                    self.history_field = None;
                    self.force_selection = true;
                }
            }
            command @ Command::Model(_) => {
                let template = command.model_source().expect("commande de modèle");
                if self.session.template != template {
                    let before = self.snapshot();
                    self.session.dependencies = crate::packages::for_template(&template);
                    self.session.template = template;
                    self.presentation = ModelPresentation::default();
                    self.session.metadata.options_mut().bullet_marker = None;
                    if let Command::Model(model) = command
                        && let Some((paper, landscape)) = model.definition().page
                    {
                        self.session.metadata.options_mut().paper = paper;
                        self.session.metadata.options_mut().landscape = landscape;
                    }
                    self.remember(before, false);
                }
            }
            Command::InsertPostscript | Command::InsertAttachment => {
                let before = self.snapshot();
                let kind = if command == Command::InsertPostscript {
                    BlockKind::Postscript
                } else {
                    BlockKind::Attachment
                };
                let at = self.session.document.view.insert_letter_note(kind);
                self.remember(before, false);
                self.focus_annotation_target(at..at);
            }
            Command::Undo | Command::Redo => self.apply_history(command == Command::Undo),
            Command::Link => self.open_link(),
            Command::RemoveLink => {
                let range = self.range();
                if range.is_empty() {
                    self.message = Some(crate::tr!("select-link-remove"));
                } else if self.session.document.view.styles[range]
                    .iter()
                    .any(|style| style.link.is_some())
                {
                    self.edit(|view, range| view.set_link(range, None));
                }
            }
            Command::Bold => self.edit(|view, range| view.toggle_mark(range, Mark::Bold)),
            Command::Italic => self.edit(|view, range| view.toggle_mark(range, Mark::Italic)),
            Command::Clear => self.edit(|view, range| view.clear_marks(range)),
            Command::Block(kind) => self.edit(|view, range| view.set_block(range, kind)),
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
                    self.spelling_menu_pending = true;
                }
            }
            Command::NextSpelling | Command::PreviousSpelling => {
                let range = self.range();
                let issues = &self.spelling.issues;
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
                    let language = self.session.metadata.options().language.code().to_owned();
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
                        self.session
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
                self.session.metadata.options_mut().spelling_ignored.clear();
                self.remember(before, false);
            }
            _ => unreachable!("spelling command"),
        }
    }
    pub(super) fn apply_history(&mut self, undo: bool) {
        let old_document = self.session.document.clone();
        let old_template = self.session.template.clone();
        if let Some(selection) = self.session.history(undo, self.selection.into()) {
            self.image_textures.clear();
            if self.session.template != old_template {
                self.presentation = ModelPresentation::default();
            }
            self.selection = selection.into();
            self.force_selection =
                self.history_field.is_none() || self.session.document != old_document;
            if self.force_selection {
                self.history_field = None;
            }
        }
    }
    pub(super) fn edit(&mut self, edit: impl FnOnce(&mut Projection, Range<usize>)) {
        self.selection = self.session.edit(self.selection.into(), edit).into();
        self.force_selection = true;
    }
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.modal_open() {
            return;
        }
        if self.search_open
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
        if self.document_open && focused && !self.modal_open() {
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
                self.history_field = None;
            } else if egui::TextEdit::load_state(ctx, id).is_some() {
                self.history_field = Some(id);
            }
        }
        if !self.information_open && !self.modal_open() {
            self.history_field = None;
        }
    }
    pub(super) fn menu_statuses(&self) -> Vec<(Command, menus::Status)> {
        self.keymap
            .registry()
            .definitions()
            .filter(|d| d.menu_position().is_some())
            .map(|d| {
                let enabled = !self.modal_open()
                    && (self.document_open || d.command.available_without_document())
                    && match d.command {
                        Command::Undo => !self.session.undo.is_empty(),
                        Command::Redo => !self.session.redo.is_empty(),
                        Command::Clear => self.history_field.is_none(),
                        Command::RecoverDocuments => !self.recovery_entries.is_empty(),
                        Command::FindNext | Command::FindPrevious => {
                            !self.search.matches.is_empty()
                        }
                        Command::NextSpelling | Command::PreviousSpelling => {
                            !self.spelling.issues.is_empty()
                        }
                        Command::IgnoreSpelling
                        | Command::LearnSpelling
                        | Command::SpellingSuggestions => self.selected_spelling_issue().is_some(),
                        Command::ResetSpelling => {
                            !self.session.metadata.options().spelling_ignored.is_empty()
                        }
                        _ => true,
                    };
                (
                    d.command,
                    menus::Status {
                        enabled,
                        checked: (d.command == Command::Information && self.information_open)
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
}
