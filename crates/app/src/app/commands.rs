use super::*;

impl App {
    pub(super) fn snapshot(&self) -> crate::engine::NativeEdit {
        self.document
            .engine
            .begin_native_edit(editor::to_core(self.document.selection))
    }
    pub(super) fn range(&self) -> Range<usize> {
        let range = self.document.selection.as_sorted_char_range();
        range.start.min(self.document.engine.document().view.len())
            ..range.end.min(self.document.engine.document().view.len())
    }
    pub(super) fn accept_edit(
        &mut self,
        result: Result<crate::engine::Selection, crate::document::edits::EditError>,
        previous: crate::engine::Selection,
    ) -> bool {
        let adjusted = match result {
            Ok(selection) => selection,
            Err(error) => {
                self.document.selection = editor::to_editor(previous);
                self.document.force_selection = true;
                self.dialogs.message = Some(Self::edit_error_text(error));
                return false;
            }
        };
        self.document.force_selection |= adjusted != editor::to_core(self.document.selection);
        self.document.selection = editor::to_editor(adjusted);
        self.publish_view_selection();
        true
    }
    pub(super) fn edit_error_text(error: crate::document::edits::EditError) -> String {
        crate::i18n::text(match error {
            crate::document::edits::EditError::ReadOnly => "file-read-only",
            crate::document::edits::EditError::ProtectedBoundary => "edit-protected-boundary",
            crate::document::edits::EditError::InvalidText => "edit-invalid-text",
            crate::document::edits::EditError::StaleDraft
            | crate::document::edits::EditError::Target(_) => "edit-stale-draft",
            _ => "edit-invalid-draft",
        })
    }
    pub(super) fn apply(&mut self, command: Command) {
        if !self.command_enabled(command) {
            return;
        }
        self.cancel_ime();
        if let Some(core) = self
            .ui_text_target()
            .and_then(|target| command.document_command_at(target))
        {
            self.apply_document_command(command, core);
            return;
        }
        match command {
            Command::ReviewProposal
            | Command::CloseReview
            | Command::AcceptProposal
            | Command::RejectProposal
            | Command::RevealAgentChange => self.apply_review_command(command),
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
                self.navigate_search(command == Command::FindPrevious);
            }
            Command::ReplaceCurrent | Command::ReplaceAll => {
                self.replace_search(command == Command::ReplaceAll)
            }
            Command::RecoverDocuments => self.recovery.open = !self.recovery.entries.is_empty(),
            Command::ImportModel => self.import_model(),
            Command::InsertImage => self.choose_image(None),
            Command::RemoveWidget => {
                if let Some(core) = self.current_widget_removal()
                    && self.run_document_command(core)
                {
                    self.bands.open = None;
                }
            }
            Command::ContextualAction => self.activate_contextual_action(),
            Command::Preferences => self.dialogs.preferences_open = true,
            Command::Assistance => self.dialogs.assistance_open = true,
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
            Command::Model(model) => self.select_model(model),
            Command::Link => self.open_link(),
            _ => unreachable!("document command dispatched through Engine"),
        }
    }
    fn apply_spelling_command(&mut self, command: Command) {
        self.document
            .engine
            .configure_spelling(&self.preferences.spelling.personal);
        match command {
            Command::ToggleSpelling => {
                self.preferences.spelling.enabled = !self.preferences.spelling.enabled;
            }
            Command::SpellingSuggestions => {
                if let Some(issue) = self.current_spelling_reference() {
                    self.open_spelling_reference(issue);
                }
            }
            Command::NextSpelling | Command::PreviousSpelling => {
                self.document.history_field = None;
                if let Some(core) = self.context_document_command(command) {
                    self.run_document_command(core);
                }
            }
            Command::IgnoreSpelling | Command::LearnSpelling => {
                if let Some(issue) = self.current_spelling_reference() {
                    self.apply_spelling_issue(command, issue);
                }
            }
            Command::ResetSpelling => {
                if let Some(core) = self.context_document_command(command) {
                    self.run_document_command(core);
                }
            }
            _ => unreachable!("spelling command"),
        }
    }
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.modal_open() {
            return;
        }
        if self.band_shortcuts(ctx) {
            return;
        }
        let band_field = self.bands.open.is_some()
            && ctx.memory(|m| m.has_focus(egui::Id::new("editor-band-input")));
        if self.search.open
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.close_search();
            return;
        }
        let focused = ctx.memory(|memory| memory.has_focus(egui::Id::new(editor::EDITOR_ID)));
        let composing =
            self.composing_input(ctx) || self.document.ime.active() || self.bands.parameter_ime;
        #[cfg(target_os = "macos")]
        let reserved = self
            .native_menus
            .as_ref()
            .map_or_else(Vec::new, |menus| menus.shortcuts());
        #[cfg(not(target_os = "macos"))]
        let reserved = Vec::new();
        let commands = ctx.input_mut(|input| {
            self.keymap.consume(input, focused, &reserved, |c| {
                if c == Command::ContextualAction {
                    // Reserve the binding in the editor even without an action:
                    // a contextual Enter must never become an ordinary newline.
                    !composing && (focused || self.bands.open.is_some())
                } else {
                    !band_field && self.command_enabled(c)
                }
            })
        });
        for command in commands {
            if self.modal_open() {
                break;
            }
            self.apply(command);
        }
        if self.document.open && focused && !band_field && !self.modal_open() {
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
                let enabled = !self.modal_open() && self.command_enabled(d.command);
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
        if self.modal_open() || (command == Command::ContextualAction && self.composing_input(ctx))
        {
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
