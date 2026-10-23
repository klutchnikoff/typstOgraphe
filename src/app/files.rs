use super::*;

impl App {
    pub(super) fn load_demo(&mut self) {
        let kind = self.dialogs.new_document_type;
        let stored = StoredDocument::demo(kind);
        let document = Document::from_source(&stored.source).expect("démonstration valide");
        self.install_document(stored, document, None)
            .expect("registre valide");
    }
    #[cfg(test)]
    pub(super) fn stored_document(&self) -> StoredDocument {
        self.document.engine.stored_document()
    }
    pub(super) fn is_dirty(&self) -> bool {
        self.document.open && self.document.engine.is_dirty()
    }
    pub(super) fn save_path(&mut self, path: PathBuf) -> bool {
        let command = crate::engine::Command::SaveDocument(crate::engine::SaveDocumentParams {
            guard: self.document.engine.target_guard(),
            path: Some(path),
        });
        self.save_file_command(command)
    }
    fn save_file_command(&mut self, command: crate::engine::Command) -> bool {
        match self.execute_document_command(command) {
            Ok(outcome) => {
                let path = outcome
                    .value
                    .as_ref()
                    .and_then(|v| v["path"].as_str())
                    .unwrap_or_default();
                self.dialogs.message = Some(crate::tr!("letter-saved", path = path));
                self.file.error = None;
                true
            }
            Err(error) => {
                self.file.error = Some(Self::command_error_text(&error));
                false
            }
        }
    }
    fn save_document(&mut self, save_as: bool) -> bool {
        if !self.document.open {
            return false;
        }
        if !save_as && let Some(path) = self.document.engine.file_path().map(PathBuf::from) {
            return self.save_path(path);
        }
        let mut dialog = rfd::FileDialog::new()
            .set_title(crate::tr!("save-dialog"))
            .add_filter(crate::tr!("letter-filter"), &[storage::EXTENSION])
            .set_file_name(
                self.document
                    .engine
                    .file_path()
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| {
                        self.recovered_filename()
                            .unwrap_or_else(|| crate::tr!("default-filename"))
                    }),
            );
        if let Some(parent) = self
            .document
            .engine
            .file_path()
            .as_ref()
            .and_then(|path| path.parent())
        {
            dialog = dialog.set_directory(parent);
        }
        if let Some(mut path) = dialog.save_file() {
            if path.extension().is_none() {
                path.set_extension(storage::EXTENSION);
                if path.exists()
                    && rfd::MessageDialog::new()
                        .set_title(crate::tr!("replace-title"))
                        .set_description(crate::tr!(
                            "replace-message",
                            path = path.display().to_string()
                        ))
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .show()
                        != rfd::MessageDialogResult::Yes
                {
                    return false;
                }
            }
            self.save_file_command(crate::engine::Command::SaveAs(
                crate::engine::SaveAsParams {
                    guard: self.document.engine.target_guard(),
                    path,
                    overwrite: true,
                },
            ))
        } else {
            false
        }
    }
    pub(super) fn open_path(&mut self, path: PathBuf) -> bool {
        // Le parcours UI a déjà traité le brouillon dans request_action/file_dialogs.
        match self.execute_document_command(crate::engine::Command::OpenDocument(
            crate::engine::OpenDocumentParams {
                guard: self.document.engine.target_guard(),
                path,
                discard_changes: true,
            },
        )) {
            Ok(_) => {
                self.dialogs.message = Some(crate::tr!("letter-opened"));
                true
            }
            Err(error) => {
                self.file.error = Some(Self::command_error_text(&error));
                false
            }
        }
    }
    pub(super) fn install_document(
        &mut self,
        letter: StoredDocument,
        _document: Document,
        path: Option<PathBuf>,
    ) -> Result<(), String> {
        self.keymap.for_document(letter.metadata.document_type())?;
        self.document
            .engine
            .install_stored(letter, path, false)
            .map_err(|e| Self::command_error_text(&e))?;
        self.after_document_replaced();
        Ok(())
    }
    pub(super) fn after_document_replaced(&mut self) {
        let keymap = self
            .keymap
            .for_document(self.document.engine.metadata().document_type())
            .expect("registre intégré valide");
        self.cancel_ime();
        self.clear_recovery();
        self.recovery.recovered_name = None;
        self.search = SearchState::default();
        self.dialogs.marker_draft = None;
        self.spelling.target = None;
        self.spelling.menu_pending = false;
        self.spelling.menu_position = None;
        self.keymap = keymap;
        self.document.history_field = None;
        self.document.open = self.document.engine.is_open();
        self.image_textures.clear();
        self.dialogs.information_open = false;
        self.dialogs.link_open = false;
        self.file.export_open = false;
        self.dialogs.image_info = None;
        self.dialogs.image_target = None;
        self.dialogs.link_target = None;
        self.dialogs.information_group = InformationGroup::General;
        self.document.presentation = ModelPresentation::default();
        self.document.selection = CCursorRange::one(CCursor::new(0));
        self.document.force_selection = true;
        self.preview.editor_scroll_pending = true;
        self.preview.forward_pending = true;
        self.preview.pages.clear();
        self.preview.sync = None;
        self.preview.error = None;
        self.preview.revision = None;
        self.preview.elapsed = None;
        self.file.error = None;
        self.dialogs.message = None;
    }
    pub(super) fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        if self.is_dirty() {
            self.file.pending_action = Some(action);
        } else {
            self.perform_action(action, ctx);
        }
    }
    pub(super) fn close_document(&mut self) {
        if let Err(error) = self.execute_document_command(crate::engine::Command::CloseDocument(
            crate::engine::CloseDocumentParams {
                guard: self.document.engine.target_guard(),
                discard_changes: true,
            },
        )) {
            self.file.error = Some(Self::command_error_text(&error));
        }
    }
    pub(super) fn perform_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        match action {
            PendingAction::New => {
                self.dialogs.new_document_type = DocumentType::ALL
                    .into_iter()
                    .find(|kind| {
                        kind.can_create()
                            && self.document.open
                            && *kind == self.document.engine.metadata().document_type()
                    })
                    .or_else(|| DocumentType::ALL.into_iter().find(|kind| kind.can_create()))
                    .unwrap_or(self.dialogs.new_document_type);
                self.dialogs.new_document_model = self
                    .dialogs
                    .new_document_type
                    .models()
                    .next()
                    .map(|entry| entry.model);
                self.dialogs.new_document_open = true;
            }
            PendingAction::Open => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(crate::tr!("open-dialog"))
                    .add_filter(crate::tr!("letter-filter"), &[storage::EXTENSION])
                    .add_filter(crate::tr!("all-files"), &["*"]);
                if let Some(parent) = self
                    .document
                    .engine
                    .file_path()
                    .as_ref()
                    .and_then(|path| path.parent())
                {
                    dialog = dialog.set_directory(parent);
                }
                if let Some(path) = dialog.pick_file() {
                    self.open_path(path);
                }
            }
            PendingAction::Close => {
                self.file.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            PendingAction::CloseDocument => self.close_document(),
            PendingAction::Recover(index) => self.recover_document(index),
        }
        ctx.request_repaint();
    }
    pub(super) fn create_document(&mut self, kind: DocumentType) {
        let Some(model) = self
            .dialogs
            .new_document_model
            .filter(|model| model.definition().kind == kind)
            .or_else(|| kind.models().next().map(|entry| entry.model))
        else {
            self.file.error = Some(crate::tr!("type-no-model"));
            return;
        };
        let letter = match self
            .preferences
            .profile
            .document(kind, self.profile_signature.as_ref())
        {
            Ok(letter) => letter,
            Err(error) => {
                self.file.error = Some(error);
                return;
            }
        };
        let signature = letter
            .metadata
            .signature_path()
            .and(self.profile_signature.as_ref())
            .map(crate::engine::ImageInput::from);
        let initial_values = letter
            .metadata
            .field_values()
            .into_iter()
            .filter(|f| !f.value.is_empty())
            .map(|f| crate::engine::InitialValue {
                field: f.field,
                value: f.value,
            })
            .collect();
        let command = crate::engine::Command::NewDocument(crate::engine::NewDocumentParams {
            guard: self.document.engine.target_guard(),
            document_type: kind,
            model: model.definition().command.id.into(),
            initial_values,
            signature,
            discard_changes: true,
        });
        match self.execute_document_command(command) {
            Err(error) => self.file.error = Some(Self::command_error_text(&error)),
            Ok(_) => {
                self.dialogs.new_document_open = false;
                self.file.export_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("output")
                    .join(kind.file_stem())
                    .display()
                    .to_string();
            }
        }
    }
    pub(super) fn handle_file_command(&mut self, command: FileCommand, ctx: &egui::Context) {
        match command {
            FileCommand::New => self.request_action(PendingAction::New, ctx),
            FileCommand::Open => self.request_action(PendingAction::Open, ctx),
            FileCommand::Close => self.request_action(PendingAction::CloseDocument, ctx),
            FileCommand::Quit => self.request_action(PendingAction::Close, ctx),
            FileCommand::Save => {
                self.save_document(false);
            }
            FileCommand::SaveAs => {
                self.save_document(true);
            }
            FileCommand::Export => {
                self.file.export_open = self.document.open && self.file.export_task.is_none();
                self.file.export_guard = Some(self.document.engine.target_guard());
            }
        }
        ctx.request_repaint();
    }
    pub(super) fn file_dialogs(&mut self, ctx: &egui::Context) {
        if self.file.pending_action.is_some() {
            let mut choice = None;
            egui::Window::new(crate::tr!("unsaved-title"))
                .id(egui::Id::new("unsaved-letter"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(crate::tr!("unsaved-message"));
                    if let Some(error) = &self.file.error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button(crate::tr!("save")).clicked() {
                            choice = Some(0);
                        }
                        if ui.button(crate::tr!("discard")).clicked() {
                            choice = Some(1);
                        }
                        if ui.button(crate::tr!("cancel")).clicked() {
                            choice = Some(2);
                        }
                    });
                });
            match choice {
                Some(0) if self.save_document(false) => {
                    if let Some(action) = self.file.pending_action.take() {
                        self.perform_action(action, ctx);
                    }
                }
                Some(1) => {
                    self.clear_recovery();
                    self.file.error = None;
                    if let Some(action) = self.file.pending_action.take() {
                        self.perform_action(action, ctx);
                    }
                }
                Some(2) => {
                    self.file.pending_action = None;
                    self.file.error = None;
                    self.document.force_selection = true;
                }
                _ => {}
            }
        } else if self.file.error.is_some() {
            let mut open = true;
            let mut close = false;
            egui::Window::new(crate::tr!("file-error-title"))
                .open(&mut open)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(self.file.error.as_deref().unwrap_or_default());
                    close = ui.button(crate::tr!("close")).clicked();
                });
            if close || !open {
                self.file.error = None;
                self.document.force_selection = true;
            }
        }
    }
}
