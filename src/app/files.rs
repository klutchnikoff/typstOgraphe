use super::*;

impl App {
    pub(super) fn load_demo(&mut self) {
        let kind = self.new_document_type;
        let stored = StoredDocument::demo(kind);
        let document = Document::from_source(&stored.source).expect("démonstration valide");
        self.install_document(stored, document, None)
            .expect("registre valide");
    }
    pub(super) fn stored_document(&self) -> StoredDocument {
        self.session.stored_document()
    }
    pub(super) fn is_dirty(&self) -> bool {
        self.document_open && self.session.is_dirty()
    }
    pub(super) fn save_path(&mut self, path: PathBuf) -> bool {
        let letter = self.stored_document();
        match storage::save(&path, &letter) {
            Ok(()) => {
                self.message = Some(crate::tr!(
                    "letter-saved",
                    path = path.display().to_string()
                ));
                self.session.file_path = Some(path);
                // Le fichier est nettoyé par storage::save ; ce repère en mémoire
                // garde les mêmes ressources que l’historique pour is_dirty.
                self.session.saved = letter;
                self.session.recovered = false;
                self.recovery.recovered_name = None;
                self.clear_recovery();
                self.file.error = None;
                true
            }
            Err(error) => {
                self.file.error = Some(error);
                false
            }
        }
    }
    pub(super) fn save_document(&mut self, save_as: bool) -> bool {
        if !self.document_open {
            return false;
        }
        if !save_as && let Some(path) = self.session.file_path.clone() {
            return self.save_path(path);
        }
        let mut dialog = rfd::FileDialog::new()
            .set_title(crate::tr!("save-dialog"))
            .add_filter(crate::tr!("letter-filter"), &[storage::EXTENSION])
            .set_file_name(
                self.session
                    .file_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| {
                        self.recovered_filename()
                            .unwrap_or_else(|| crate::tr!("default-filename"))
                    }),
            );
        if let Some(parent) = self
            .session
            .file_path
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
            self.save_path(path)
        } else {
            false
        }
    }
    pub(super) fn open_path(&mut self, path: PathBuf) -> bool {
        match storage::read(&path).and_then(|letter| {
            Document::from_source(&letter.source).map(|document| (letter, document))
        }) {
            Ok((letter, document)) => {
                if let Err(error) = self.install_document(letter, document, Some(path)) {
                    self.file.error = Some(error);
                    return false;
                }
                self.message = Some(crate::tr!("letter-opened"));
                true
            }
            Err(error) => {
                self.file.error = Some(error);
                false
            }
        }
    }
    pub(super) fn install_document(
        &mut self,
        letter: StoredDocument,
        document: Document,
        path: Option<PathBuf>,
    ) -> Result<(), String> {
        let keymap = self.keymap.for_document(letter.metadata.document_type())?;
        self.clear_recovery();
        self.recovery.recovered_name = None;
        self.search.engine = Default::default();
        self.search.open = false;
        self.search.replace_open = false;
        self.search.focus_pending = false;
        self.spelling.target = None;
        self.spelling.menu_pending = false;
        self.spelling.menu_position = None;
        self.keymap = keymap;
        self.history_field = None;
        self.session = Session::new(letter, document, path, self.session.revision + 1);
        self.document_open = true;
        self.image_textures.clear();
        self.image_info = None;
        self.information_group = InformationGroup::General;
        self.presentation = ModelPresentation::default();
        self.selection = CCursorRange::one(CCursor::new(0));
        self.force_selection = true;
        self.preview.editor_scroll_pending = true;
        self.preview.forward_pending = true;
        self.preview.pages.clear();
        self.preview.sync = None;
        self.preview.error = None;
        self.preview.revision = None;
        self.preview.elapsed = None;
        self.file.error = None;
        self.message = None;
        Ok(())
    }
    pub(super) fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        if self.is_dirty() {
            self.file.pending_action = Some(action);
        } else {
            self.perform_action(action, ctx);
        }
    }
    pub(super) fn close_document(&mut self) {
        let kind = self.new_document_type;
        let letter = StoredDocument::blank(kind);
        let document = Document::from_source("").expect("document vide valide");
        // install_document réinitialise aussi l'historique, les dialogues liés
        // au document et les résultats de rendu. Les préférences restent globales.
        self.install_document(letter, document, None)
            .expect("registre valide");
        self.document_open = false;
        self.information_open = false;
        self.link_open = false;
        self.file.export_open = false;
        self.history_field = None;
    }
    pub(super) fn perform_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        match action {
            PendingAction::New => {
                self.new_document_type = DocumentType::ALL
                    .into_iter()
                    .find(|kind| {
                        kind.can_create()
                            && self.document_open
                            && *kind == self.session.metadata.document_type()
                    })
                    .or_else(|| DocumentType::ALL.into_iter().find(|kind| kind.can_create()))
                    .unwrap_or(self.new_document_type);
                self.new_document_model = self
                    .new_document_type
                    .models()
                    .next()
                    .map(|entry| entry.model);
                self.new_document_open = true;
            }
            PendingAction::Open => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(crate::tr!("open-dialog"))
                    .add_filter(crate::tr!("letter-filter"), &[storage::EXTENSION])
                    .add_filter(crate::tr!("all-files"), &["*"]);
                if let Some(parent) = self
                    .session
                    .file_path
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
            .new_document_model
            .filter(|model| model.definition().kind == kind)
            .or_else(|| kind.models().next().map(|entry| entry.model))
        else {
            self.file.error = Some(crate::tr!("type-no-model"));
            return;
        };
        let mut letter = match self
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
        letter.template = model.source();
        let document = Document::from_source(&letter.source).expect("document vide valide");
        if let Err(error) = self.install_document(letter, document, None) {
            self.file.error = Some(error);
        } else {
            self.new_document_open = false;
            self.file.export_path = std::env::current_dir()
                .unwrap_or_default()
                .join("output")
                .join(kind.file_stem())
                .display()
                .to_string();
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
            FileCommand::Export => self.file.export_open = self.document_open,
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
                    self.force_selection = true;
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
                self.force_selection = true;
            }
        }
    }
}
