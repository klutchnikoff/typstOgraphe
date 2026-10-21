use crate::{
    annotations::OptionalAnnotation,
    commands::{Command, FileCommand, Keymap, Registry, ShortcutOverrides},
    document::{
        BlockKind, Document, DocumentLanguage, ImageBlock, InlineStyle, Mark, PaperFormat,
        Projection,
    },
    document_types::{DocumentType, ModelPresentation, models::Model},
    editor, menus,
    navigation::{PagePoint, RenderSync},
    preferences::{Appearance, EditorFont, Preferences},
    rendering::{RenderRequest, RenderWorker},
    session::{EditGroup, Session, Snapshot},
    storage::{self, StoredDocument},
};
use eframe::egui::{
    self, Color32, RichText,
    text::{CCursor, CCursorRange},
};
use std::{
    ops::Range,
    path::PathBuf,
    time::{Duration, Instant},
};

const ACCENT: Color32 = Color32::from_rgb(40, 92, 119);
const PREVIEW_SELECTION: Color32 = Color32::from_rgba_premultiplied(16, 33, 59, 65);
const SEARCH_ID: &str = "document-search";
const REPLACE_ID: &str = "document-replacement";
#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
    Close,
    CloseDocument,
    Recover(usize),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum InformationGroup {
    #[default]
    General,
    Fields(&'static str),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PreferencesPage {
    Profile,
    #[default]
    Appearance,
    Spelling,
}

pub struct App {
    // La session vide sert uniquement de tampon interne tant qu'aucun document
    // n'est ouvert : pas de rendu, d'édition ni de copie de récupération.
    document_open: bool,
    session: Session,
    spelling: crate::spelling::Checker,
    spelling_target: Option<(u64, crate::spelling::Issue)>,
    spelling_menu_pending: bool,
    spelling_menu_position: Option<egui::Pos2>,
    capture_spelling: bool,
    recovery: Option<crate::recovery::Recovery>,
    recovery_entries: Vec<crate::recovery::Entry>,
    recovery_open: bool,
    recovery_error: Option<String>,
    recovered_name: Option<String>,
    search: crate::search::Search,
    search_open: bool,
    replace_open: bool,
    search_focus_pending: bool,
    preferences: Preferences,
    preferences_tracker: crate::preferences::SaveTracker,
    preferences_directory: Option<PathBuf>,
    preferences_page: PreferencesPage,
    profile_signature: Option<crate::assets::ImageData>,
    profile_error: Option<String>,
    preferences_open: bool,
    new_document_open: bool,
    new_document_type: DocumentType,
    new_document_model: Option<Model>,
    keymap: Keymap,
    #[cfg(target_os = "macos")]
    native_menus: Option<menus::native::NativeMenus>,
    history_field: Option<egui::Id>,
    presentation: ModelPresentation,
    file_command: Option<FileCommand>,
    pending_action: Option<PendingAction>,
    file_error: Option<String>,
    allow_close: bool,
    window_title: String,
    selection: CCursorRange,
    force_selection: bool,
    worker: RenderWorker,
    statistics: crate::statistics::Counter,
    submitted: u64,
    pages: Vec<egui::TextureHandle>,
    rendering_time: Option<Duration>,
    rendering_error: Option<String>,
    rendered_revision: Option<u64>,
    sync: Option<RenderSync>,
    forward_pending: bool,
    editor_scroll_pending: bool,
    information_open: bool,
    information_group: InformationGroup,
    palette_open: bool,
    palette_query: String,
    palette_index: usize,
    image_info: Option<u64>,
    image_alt: String,
    image_textures: std::collections::HashMap<String, egui::TextureHandle>,
    link_open: bool,
    link_url: String,
    link_error: Option<String>,
    link_range: Range<usize>,
    export_open: bool,
    export_path: String,
    message: Option<String>,
    capture: Option<PathBuf>,
    screenshot_requested: bool,
    started: Instant,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        capture: Option<PathBuf>,
        path: Option<PathBuf>,
        capture_recovery: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if capture.is_some() {
            cc.egui_ctx
                .memory_mut(|memory| *memory = Default::default());
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1240.0, 900.0)));
        }
        let mut app = Self::with_context(cc.egui_ctx.clone(), capture);
        if app.capture.is_none() {
            app.preferences = Preferences::load(cc.storage);
            app.preferences_directory = eframe::storage_dir("typstOgraphe");
            if let Some(directory) = &app.preferences_directory {
                match app.preferences.profile.load_signature(directory) {
                    Ok(signature) => app.profile_signature = signature,
                    Err(error) => {
                        app.profile_error =
                            Some(crate::tr!("profile-signature-error", error = error))
                    }
                }
            }
        }
        app.preferences_tracker = crate::preferences::SaveTracker::new(&app.preferences);
        cc.egui_ctx.set_theme(app.preferences.appearance.theme());
        if let Some(path) = path {
            app.open_path(path);
        }
        // Les captures et les tests restent isolés des données réelles.
        if let Some(directory) = capture_recovery {
            app.start_recovery(directory, cc.egui_ctx.clone());
        } else if app.capture.is_none() {
            match eframe::storage_dir("typstOgraphe") {
                Some(directory) => {
                    app.start_recovery(directory.join("recovery"), cc.egui_ctx.clone())
                }
                None => app.recovery_error = Some(crate::tr!("recovery-no-directory")),
            }
        }
        #[cfg(target_os = "macos")]
        {
            app.native_menus = Some(menus::native::NativeMenus::new(
                cc.egui_ctx.clone(),
                &app.keymap,
            )?);
        }
        Ok(app)
    }
    pub fn with_context(ctx: egui::Context, capture: Option<PathBuf>) -> Self {
        editor::install_fonts(&ctx);
        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.visuals.selection.bg_fill = if style.visuals.dark_mode {
                Color32::from_rgb(40, 80, 108)
            } else {
                Color32::from_rgb(174, 211, 235)
            };
            style.visuals.selection.stroke = egui::Stroke::new(
                1.0_f32,
                if style.visuals.dark_mode {
                    Color32::LIGHT_BLUE
                } else {
                    ACCENT
                },
            );
        });
        let initial_type = DocumentType::ALL
            .into_iter()
            .find(|kind| kind.can_create())
            .expect("au moins un type possède un modèle fourni");
        let saved = StoredDocument::blank(initial_type);
        let keymap = Keymap::new(
            Registry::new(saved.metadata.document_type()),
            ctx.os().is_mac(),
            &ShortcutOverrides::default(),
        )
        .expect("raccourcis par défaut cohérents");
        let mut preferences = Preferences::default();
        if capture.is_some() {
            // Diagnostic reproductible, isolé des préférences et de l’apparence du système.
            preferences.appearance = Appearance::Light;
            ctx.set_theme(egui::ThemePreference::Light);
            ctx.all_styles_mut(|style| {
                style.visuals.text_cursor.blink = false;
                style.animation_time = 0.0;
            });
        }
        let preferences_tracker = crate::preferences::SaveTracker::new(&preferences);
        let spelling_ctx = ctx.clone();
        let spelling = crate::spelling::Checker::new(move || spelling_ctx.request_repaint());
        let worker = RenderWorker::spawn(ctx);
        let document =
            Document::from_source(&saved.source).expect("document de démonstration valide");
        Self {
            document_open: false,
            session: Session::new(saved, document, None, 1),
            spelling,
            spelling_target: None,
            spelling_menu_pending: false,
            spelling_menu_position: None,
            capture_spelling: false,
            recovery: None,
            recovery_entries: vec![],
            recovery_open: false,
            recovery_error: None,
            recovered_name: None,
            search: Default::default(),
            search_open: false,
            replace_open: false,
            search_focus_pending: false,
            preferences,
            preferences_tracker,
            preferences_directory: None,
            preferences_page: PreferencesPage::default(),
            profile_signature: None,
            profile_error: None,
            preferences_open: false,
            new_document_open: false,
            new_document_type: initial_type,
            new_document_model: initial_type.models().next().map(|entry| entry.model),
            keymap,
            #[cfg(target_os = "macos")]
            native_menus: None,
            history_field: None,
            presentation: ModelPresentation::default(),
            file_command: None,
            pending_action: None,
            file_error: None,
            allow_close: false,
            window_title: String::new(),
            selection: CCursorRange::one(CCursor::new(0)),
            force_selection: false,
            worker,
            statistics: Default::default(),
            submitted: 0,
            pages: vec![],
            rendering_time: None,
            rendering_error: None,
            rendered_revision: None,
            sync: None,
            forward_pending: true,
            editor_scroll_pending: false,
            information_open: false,
            information_group: InformationGroup::General,
            palette_open: false,
            palette_query: String::new(),
            palette_index: 0,
            image_info: None,
            image_alt: String::new(),
            image_textures: Default::default(),
            link_open: false,
            link_url: String::new(),
            link_error: None,
            link_range: 0..0,
            export_open: false,
            export_path: std::env::current_dir()
                .unwrap_or_default()
                .join("output")
                .join("courrier")
                .display()
                .to_string(),
            message: None,
            capture,
            screenshot_requested: false,
            started: Instant::now(),
        }
    }
    fn snapshot(&self) -> Snapshot {
        self.session.snapshot(self.selection.into())
    }
    fn load_demo(&mut self) {
        let kind = self.new_document_type;
        let stored = StoredDocument::demo(kind);
        let document = Document::from_source(&stored.source).expect("démonstration valide");
        self.install_document(stored, document, None)
            .expect("registre valide");
    }
    #[cfg(test)]
    fn with_demo_context(ctx: egui::Context, capture: Option<PathBuf>) -> Self {
        let mut app = Self::with_context(ctx, capture);
        app.load_demo();
        app
    }
    fn stored_document(&self) -> StoredDocument {
        self.session.stored_document()
    }
    fn is_dirty(&self) -> bool {
        self.document_open && self.session.is_dirty()
    }
    fn modal_open(&self) -> bool {
        self.recovery_open
            || self.new_document_open
            || self.image_info.is_some()
            || self.preferences_open
            || self.palette_open
            || self.link_open
            || self.export_open
            || self.pending_action.is_some()
            || self.file_error.is_some()
    }
    fn start_recovery(&mut self, directory: PathBuf, ctx: egui::Context) {
        match crate::recovery::Recovery::start(&directory, ctx) {
            Ok((recovery, entries)) => {
                self.recovery = Some(recovery);
                self.recovery_open = !entries.is_empty();
                self.recovery_entries = entries;
            }
            Err(error) => self.recovery_error = Some(error),
        }
    }
    fn clear_recovery(&mut self) {
        if let Some(recovery) = &mut self.recovery
            && let Err(error) = recovery.clear()
        {
            self.recovery_error = Some(error);
        }
    }
    fn checkpoint_recovery(&mut self) {
        if !self.document_open {
            return;
        }
        let dirty = self.is_dirty();
        if let Some(recovery) = &mut self.recovery {
            for result in recovery.poll() {
                self.recovery_error = result.err();
            }
            let session = &self.session;
            let suggested = &self.recovered_name;
            if let Err(error) = recovery.checkpoint(session.revision, dirty, || {
                let name = session
                    .file_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .or_else(|| suggested.clone());
                (session.stored_document(), name)
            }) {
                self.recovery_error = Some(error);
            }
        }
    }
    fn recovered_filename(&self) -> Option<String> {
        self.recovered_name.as_ref().map(|name| {
            let stem = std::path::Path::new(name)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            format!("{stem}-{}.typsto", crate::tr!("recovery-filename-suffix"))
        })
    }
    fn recover_document(&mut self, index: usize) {
        let Some(entry) = self.recovery_entries.get(index) else {
            return;
        };
        let name = entry.name();
        let restored = entry.load().and_then(|stored| {
            Document::from_source(&stored.source).map(|document| (stored, document))
        });
        match restored.and_then(|(stored, document)| self.install_document(stored, document, None))
        {
            Ok(()) => {
                let entry = self.recovery_entries.remove(index);
                self.session.recovered = true;
                self.recovered_name = Some(name);
                if let Some(recovery) = &mut self.recovery
                    && let Err(error) = recovery.adopt(entry)
                {
                    self.recovery_error = Some(error);
                }
                self.recovery_open = false;
                self.message = Some(crate::tr!("recovery-restored"));
            }
            Err(error) => self.file_error = Some(crate::tr!("recovery-load-failed", error = error)),
        }
    }
    fn recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery_open || self.pending_action.is_some() || self.file_error.is_some() {
            return;
        }
        let mut open = true;
        let mut choice = None;
        let mut later = false;
        egui::Window::new(crate::tr!("recovery-title"))
            .id(egui::Id::new("recovery-documents"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.label(crate::tr!("recovery-help"));
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        for (index, entry) in self.recovery_entries.iter().enumerate() {
                            ui.separator();
                            ui.strong(entry.name());
                            match &entry.info {
                                Ok(info) => {
                                    ui.weak(crate::tr!("recovery-date", date = info.date()));
                                }
                                Err(error) => {
                                    ui.colored_label(
                                        Color32::LIGHT_RED,
                                        crate::tr!("recovery-load-failed", error = error.as_str()),
                                    );
                                }
                            }
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(
                                        entry.info.is_ok(),
                                        egui::Button::new(crate::tr!("recovery-restore")),
                                    )
                                    .clicked()
                                {
                                    choice = Some((index, true));
                                }
                                if ui.button(crate::tr!("recovery-delete")).clicked() {
                                    choice = Some((index, false));
                                }
                            });
                        }
                    });
                ui.separator();
                later = ui.button(crate::tr!("recovery-later")).clicked();
            });
        self.recovery_open = open && !later;
        if let Some((index, restore)) = choice {
            if restore {
                self.request_action(PendingAction::Recover(index), ctx);
            } else {
                match self.recovery_entries[index].delete() {
                    Ok(()) => {
                        self.recovery_entries.remove(index);
                    }
                    Err(error) => {
                        self.file_error = Some(crate::tr!("recovery-delete-failed", error = error))
                    }
                }
                self.recovery_open &= !self.recovery_entries.is_empty();
            }
        }
    }
    fn save_path(&mut self, path: PathBuf) -> bool {
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
                self.recovered_name = None;
                self.clear_recovery();
                self.file_error = None;
                true
            }
            Err(error) => {
                self.file_error = Some(error);
                false
            }
        }
    }
    fn save_document(&mut self, save_as: bool) -> bool {
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
    fn open_path(&mut self, path: PathBuf) -> bool {
        match storage::read(&path).and_then(|letter| {
            Document::from_source(&letter.source).map(|document| (letter, document))
        }) {
            Ok((letter, document)) => {
                if let Err(error) = self.install_document(letter, document, Some(path)) {
                    self.file_error = Some(error);
                    return false;
                }
                self.message = Some(crate::tr!("letter-opened"));
                true
            }
            Err(error) => {
                self.file_error = Some(error);
                false
            }
        }
    }
    fn install_document(
        &mut self,
        letter: StoredDocument,
        document: Document,
        path: Option<PathBuf>,
    ) -> Result<(), String> {
        let keymap = self.keymap.for_document(letter.metadata.document_type())?;
        self.clear_recovery();
        self.recovered_name = None;
        self.search = Default::default();
        self.search_open = false;
        self.replace_open = false;
        self.search_focus_pending = false;
        self.spelling_target = None;
        self.spelling_menu_pending = false;
        self.spelling_menu_position = None;
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
        self.editor_scroll_pending = true;
        self.forward_pending = true;
        self.pages.clear();
        self.sync = None;
        self.rendering_error = None;
        self.rendered_revision = None;
        self.rendering_time = None;
        self.file_error = None;
        self.message = None;
        Ok(())
    }
    fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        if self.is_dirty() {
            self.pending_action = Some(action);
        } else {
            self.perform_action(action, ctx);
        }
    }
    fn close_document(&mut self) {
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
        self.export_open = false;
        self.history_field = None;
    }
    fn welcome_ui(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        ui.add_space((height * 0.3).max(20.0));
        ui.vertical_centered(|ui| {
            ui.heading("typstOgraphe");
            ui.add_space(8.0);
            ui.label(crate::tr!("welcome-help"));
            ui.add_space(16.0);
            for command in [
                Command::File(FileCommand::New),
                Command::File(FileCommand::Open),
                Command::Preferences,
            ] {
                let response = ui.add_sized([240.0, 32.0], egui::Button::new(command.label()));
                if response.clicked() {
                    self.apply(command);
                }
            }
        });
    }
    fn perform_action(&mut self, action: PendingAction, ctx: &egui::Context) {
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
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            PendingAction::CloseDocument => self.close_document(),
            PendingAction::Recover(index) => self.recover_document(index),
        }
        ctx.request_repaint();
    }
    fn create_document(&mut self, kind: DocumentType) {
        let Some(model) = self
            .new_document_model
            .filter(|model| model.definition().kind == kind)
            .or_else(|| kind.models().next().map(|entry| entry.model))
        else {
            self.file_error = Some(crate::tr!("type-no-model"));
            return;
        };
        let mut letter = match self
            .preferences
            .profile
            .document(kind, self.profile_signature.as_ref())
        {
            Ok(letter) => letter,
            Err(error) => {
                self.file_error = Some(error);
                return;
            }
        };
        letter.template = model.source();
        let document = Document::from_source(&letter.source).expect("document vide valide");
        if let Err(error) = self.install_document(letter, document, None) {
            self.file_error = Some(error);
        } else {
            self.new_document_open = false;
            self.export_path = std::env::current_dir()
                .unwrap_or_default()
                .join("output")
                .join(kind.file_stem())
                .display()
                .to_string();
        }
    }
    fn handle_file_command(&mut self, command: FileCommand, ctx: &egui::Context) {
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
            FileCommand::Export => self.export_open = self.document_open,
        }
        ctx.request_repaint();
    }
    fn range(&self) -> Range<usize> {
        let range = self.selection.as_sorted_char_range();
        range.start.min(self.session.document.view.len())
            ..range.end.min(self.session.document.view.len())
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn changed(&mut self) {
        let adjusted = self.session.changed(self.selection.into());
        self.force_selection |= adjusted != self.selection.into();
        self.selection = adjusted.into();
    }
    fn remember(&mut self, before: Snapshot, typing: bool) {
        let adjusted = self.session.remember(before, self.selection.into(), typing);
        self.force_selection |= adjusted != self.selection.into();
        self.selection = adjusted.into();
    }
    fn apply(&mut self, command: Command) {
        if !self.keymap.registry().contains(command)
            || (!self.document_open && !command.available_without_document())
        {
            return;
        }
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
    fn apply_history(&mut self, undo: bool) {
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
    fn edit(&mut self, edit: impl FnOnce(&mut Projection, Range<usize>)) {
        self.selection = self.session.edit(self.selection.into(), edit).into();
        self.force_selection = true;
    }
    fn install_model(&mut self, template: String) -> Result<(), String> {
        if template.len() > 1024 * 1024 {
            return Err(crate::tr!("model-too-large"));
        }
        let mut metadata = self.session.metadata.clone();
        metadata.options_mut().bullet_marker = None;
        let mut world = crate::rendering::TypstWorld::default();
        let dependencies = crate::packages::with_existing(&template, &self.session.dependencies);
        world.set_dependencies(&dependencies);
        world.set_assets(&self.session.document.assets);
        let presentation = world
            .validate_model(self.session.document.source(), &metadata, &template)
            .map_err(|error| format!("{}\n{}", metadata.model_signature(), error))?;
        let before = self.snapshot();
        self.session.dependencies = dependencies;
        self.session.template = template;
        self.session.metadata = metadata;
        self.presentation = presentation;
        self.remember(before, false);
        Ok(())
    }
    fn import_model(&mut self) {
        use std::io::Read;
        let Some(path) = rfd::FileDialog::new()
            .set_title(Command::ImportModel.label())
            .add_filter(crate::tr!("model-file-filter"), &["typ"])
            .pick_file()
        else {
            return;
        };
        let result = std::fs::File::open(path)
            .map_err(|error| error.to_string())
            .and_then(|file| {
                let mut source = String::new();
                file.take(1024 * 1024 + 1)
                    .read_to_string(&mut source)
                    .map_err(|error| error.to_string())?;
                self.install_model(source)
            });
        if let Err(error) = result {
            self.file_error = Some(crate::tr!("model-import-error", error = error));
        }
    }
    fn choose_image(&mut self, replace: Option<u64>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(crate::tr!("image-files"), &["png", "jpg", "jpeg", "svg"])
            .pick_file()
            && let Err(error) = self.import_image(&path, replace)
        {
            self.file_error = Some(error);
        }
    }
    fn import_image(&mut self, path: &std::path::Path, replace: Option<u64>) -> Result<(), String> {
        let image = crate::assets::ImageData::read(path)?;
        let before = self.snapshot();
        let asset_path = image.insert(&mut self.session.document.assets, "image");
        let at = if let Some(id) = replace {
            let Some(image) = self.session.document.view.images.get_mut(&id) else {
                self.session.document = before.document;
                return Ok(());
            };
            image.path = asset_path;
            self.session.document.view.image_range(id).unwrap().start
        } else {
            let range = self.range();
            self.session.document.view.delete_char_range(range.clone());
            self.session.document.view.insert_image(
                range.start,
                ImageBlock {
                    path: asset_path,
                    alt: String::new(),
                },
            )
        };
        if let Err(error) = self.check_asset_budget() {
            self.session.document = before.document;
            return Err(error);
        }
        self.focus_annotation_target(at + 1..at + 1);
        self.remember(before, false);
        Ok(())
    }
    fn check_asset_budget(&self) -> Result<(), String> {
        let dependency_bytes = self
            .session
            .dependencies
            .values()
            .map(|v| v.len())
            .sum::<usize>();
        if self
            .session
            .document
            .assets
            .values()
            .map(|v| v.len())
            .sum::<usize>()
            + dependency_bytes
            <= crate::assets::MAX_DOCUMENT_BYTES as usize
        {
            return Ok(());
        }
        let mut document = self.session.document.clone();
        document.sync_source();
        let retained = crate::rendering::retained_assets(
            document.source(),
            &self.session.metadata,
            &self.session.template,
            &document.assets,
            &self.session.dependencies,
        )?;
        if retained.values().map(|v| v.len()).sum::<usize>() + dependency_bytes
            > crate::assets::MAX_DOCUMENT_BYTES as usize
        {
            return Err(crate::tr!("storage-letter-too-large"));
        }
        Ok(())
    }
    fn set_document_signature(
        &mut self,
        image: Option<&crate::assets::ImageData>,
    ) -> Result<(), String> {
        if !matches!(
            self.session.metadata,
            crate::document_types::Metadata::Courrier(_)
        ) {
            return Ok(());
        }
        let current = self.session.metadata.signature_path();
        if match (image, current) {
            (None, None) => true,
            (Some(image), Some(path)) => self
                .session
                .document
                .assets
                .get(path)
                .is_some_and(|data| data == &image.data),
            _ => false,
        } {
            return Ok(());
        }
        let before = self.snapshot();
        let path = image.map(|image| image.insert(&mut self.session.document.assets, "signature"));
        if let crate::document_types::Metadata::Courrier(meta) = &mut self.session.metadata {
            meta.signature = path;
        }
        if image.is_some()
            && let Err(error) = self.check_asset_budget()
        {
            self.session.document = before.document;
            self.session.metadata = before.metadata;
            return Err(error);
        }
        self.remember(before, false);
        Ok(())
    }
    fn use_profile_signature(&mut self) -> Result<(), String> {
        let image = self
            .profile_signature
            .clone()
            .filter(|_| self.document_open && self.preferences.profile.signature.is_some())
            .ok_or_else(|| crate::tr!("signature-profile-unavailable"))?;
        self.set_document_signature(Some(&image))
    }
    fn choose_signature(&mut self, profile: bool) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(crate::tr!("signature-choose"))
            .add_filter(crate::tr!("image-files"), &["png", "jpg", "jpeg", "svg"])
            .pick_file()
        else {
            return;
        };
        if profile {
            let result = self
                .preferences_directory
                .as_deref()
                .ok_or_else(|| crate::tr!("profile-no-directory"))
                .and_then(|directory| crate::profile::Profile::import_signature(directory, &path));
            match result {
                Ok((name, image)) => {
                    self.preferences.profile.signature = Some(name);
                    self.profile_signature = Some(image);
                    self.profile_error = None;
                }
                Err(error) => {
                    self.profile_error = Some(crate::tr!("profile-signature-error", error = error))
                }
            }
        } else {
            let result = crate::assets::ImageData::read(&path)
                .and_then(|image| self.set_document_signature(Some(&image)));
            if let Err(error) = result {
                self.file_error = Some(error);
            }
        }
    }
    fn signature_preview(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        image: &crate::assets::ImageData,
    ) {
        if !self.image_textures.contains_key(key)
            && let Ok(thumbnail) = crate::assets::thumbnail(&image.data, &image.extension)
        {
            self.image_textures.insert(
                key.into(),
                ui.ctx().load_texture(key, thumbnail, Default::default()),
            );
        }
        if let Some(texture) = self.image_textures.get(key) {
            let size = texture.size_vec2();
            let scale = (220.0 / size.x).min(72.0 / size.y).min(1.0);
            egui::Frame::new()
                .fill(Color32::WHITE)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.add(egui::Image::new(texture).fit_to_exact_size(size * scale));
                });
        }
    }
    fn document_signature_ui(&mut self, ui: &mut egui::Ui) {
        let path = self.session.metadata.signature_path().map(str::to_owned);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::tr!("signature-label"));
            if ui
                .button(if path.is_some() {
                    crate::tr!("image-replace")
                } else {
                    crate::tr!("signature-choose")
                })
                .clicked()
            {
                self.choose_signature(false);
            }
            if path.is_some() && ui.button(crate::tr!("image-remove")).clicked() {
                let _ = self.set_document_signature(None);
            }
            let available =
                self.preferences.profile.signature.is_some() && self.profile_signature.is_some();
            if ui
                .add_enabled(
                    available,
                    egui::Button::new(crate::tr!("signature-from-profile")),
                )
                .on_hover_text(crate::tr!("signature-from-profile-help"))
                .on_disabled_hover_text(crate::tr!("signature-profile-unavailable"))
                .clicked()
                && let Err(error) = self.use_profile_signature()
            {
                self.file_error = Some(error);
            }
        });
        if let Some(path) = self.session.metadata.signature_path().map(str::to_owned)
            && let Some(data) = self.session.document.assets.get(&path).cloned()
        {
            let image = crate::assets::ImageData {
                extension: path.rsplit('.').next().unwrap().into(),
                data,
            };
            self.signature_preview(ui, &path, &image);
        }
        ui.weak(crate::tr!("signature-document-help"));
    }
    fn remove_image(&mut self, id: u64) {
        if let Some(range) = self.session.document.view.image_range(id) {
            let before = self.snapshot();
            let end = (range.end + 1).min(self.session.document.view.len());
            self.session
                .document
                .view
                .delete_char_range(range.start..end);
            self.selection = editor::cursor(range.start..range.start);
            self.force_selection = true;
            self.remember(before, false);
        }
    }
    fn open_link(&mut self) {
        let range = self.range();
        if range.is_empty() {
            self.message = Some(crate::tr!("select-link"));
            return;
        }
        if !self
            .session
            .document
            .view
            .is_single_paragraph(range.clone())
        {
            self.message = Some(crate::tr!("link-one-paragraph"));
            return;
        }
        self.link_url = self
            .session
            .document
            .view
            .styles
            .get(range.start)
            .and_then(|s| s.link.clone())
            .unwrap_or_default();
        self.link_range = range;
        self.link_error = None;
        self.link_open = true;
    }
    fn request(&self, export_dir: Option<PathBuf>) -> RenderRequest {
        RenderRequest {
            revision: self.session.revision,
            source: self.session.document.source().to_owned(),
            source_map: self.session.document.source_map.clone(),
            metadata: self.session.metadata.clone(),
            template: self.session.template.clone(),
            assets: self.session.document.assets.clone(),
            dependencies: self.session.dependencies.clone(),
            export_dir,
        }
    }
    fn rendering(&mut self, ctx: &egui::Context) {
        if !self.document_open {
            while self.worker.receiver.try_recv().is_ok() {}
            return;
        }
        while let Ok(response) = self.worker.receiver.try_recv() {
            match response.result {
                Ok(rendered) => {
                    if response.is_export {
                        self.message = Some(crate::tr!(
                            "letter-exported",
                            path = rendered
                                .export_dir
                                .as_ref()
                                .map_or(String::new(), |p| p.display().to_string())
                        ));
                    }
                    if response.revision == self.session.revision {
                        self.pages = rendered
                            .pages
                            .into_iter()
                            .enumerate()
                            .map(|(index, page)| {
                                ctx.load_texture(
                                    format!("typst-{}-{index}", response.revision),
                                    egui::ColorImage::from_rgba_premultiplied(
                                        page.size, &page.rgba,
                                    ),
                                    egui::TextureOptions::LINEAR,
                                )
                            })
                            .collect();
                        self.rendering_time = Some(rendered.elapsed);
                        self.presentation = rendered.presentation;
                        self.rendering_error = None;
                        self.rendered_revision = Some(response.revision);
                        self.sync = Some(rendered.sync);
                        self.forward_pending = true;
                    }
                }
                Err(error) => {
                    if response.revision == self.session.revision || response.is_export {
                        self.rendering_error = Some(error);
                        self.rendered_revision = Some(response.revision);
                    }
                }
            }
        }
        if self.submitted != self.session.revision
            && self.session.changed_at.elapsed() > Duration::from_millis(220)
        {
            match self.worker.sender.send(self.request(None)) {
                Ok(()) => self.submitted = self.session.revision,
                Err(_) => self.rendering_error = Some(crate::tr!("renderer-stopped")),
            }
        }
        if self.rendered_revision != Some(self.session.revision) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
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
    fn track_history_focus(&mut self, ctx: &egui::Context) {
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
    fn menu_statuses(&self) -> Vec<(Command, menus::Status)> {
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
    fn dispatch_action(&mut self, command: Command, ctx: &egui::Context) {
        if self.modal_open() {
            return;
        }
        self.apply(command);
        ctx.request_repaint();
    }
    /// Une seule voie pour rendre visible et focaliser une cible d’annotation.
    fn focus_annotation_target(&mut self, range: Range<usize>) {
        self.selection = editor::cursor(range);
        if self.selection.is_empty() {
            self.selection.primary.prefer_next_row = true;
            self.selection.secondary.prefer_next_row = true;
        }
        self.force_selection = true;
        self.editor_scroll_pending = true;
        self.forward_pending = true;
        self.session.typing_group = false;
    }
    fn leave_annotation(&mut self, annotation: &OptionalAnnotation) {
        let before = self.snapshot();
        let at = self.session.document.view.leave_annotation(annotation);
        self.remember(before, false);
        self.focus_annotation_target(at..at);
    }
    fn annotation_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        let range = self.range();
        let Some(annotation) = self.session.document.view.annotation_at(range.clone()) else {
            return false;
        };
        let shift = ctx.input(|i| i.events.iter().any(|event| matches!(event,
            egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } if modifiers.shift
        )));
        let tab = ctx.input_mut(|i| {
            i.consume_key(
                if shift {
                    egui::Modifiers::SHIFT
                } else {
                    egui::Modifiers::NONE
                },
                egui::Key::Tab,
            )
        });
        if tab {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            let target = if shift {
                annotation.main_target()
            } else {
                annotation.content.start..annotation.content.start
            };
            self.focus_annotation_target(target);
            return true;
        }
        if annotation.enter_leaves(&range)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.leave_annotation(&annotation);
            return true;
        }
        false
    }
    fn text_shortcuts(&mut self, ctx: &egui::Context) {
        if self.annotation_shortcuts(ctx) {
            return;
        }
        let range = self.range();
        let line = self.session.document.view.line_at(range.start);
        let lines = self.session.document.view.line_ranges();
        let start = lines[line].start;
        let prefix: String = self
            .session
            .document
            .view
            .text
            .chars()
            .skip(start)
            .take(range.start - start)
            .collect();
        let typed = ctx.input(|i| {
            if !range.is_empty() {
                return None;
            }
            i.events.iter().find_map(|event| {
                if let egui::Event::Text(text) = event {
                    Some(text.clone())
                } else {
                    None
                }
            })
        });
        if let BlockKind::Image(id) = self.session.document.view.blocks[line] {
            let object_selected = range == (start..start + 1);
            let deleting = ctx
                .input(|i| i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete));
            if object_selected && deleting {
                ctx.input_mut(|i| {
                    i.events.retain(|e| {
                        !matches!(
                            e,
                            egui::Event::Key {
                                key: egui::Key::Backspace | egui::Key::Delete,
                                ..
                            }
                        )
                    })
                });
                self.remove_image(id);
                return;
            }
            if range.is_empty()
                && range.start == start + 1
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
            {
                self.selection = editor::cursor(start..start + 1);
                self.force_selection = true;
                return;
            }
        }
        if typed.as_deref() == Some(" ") {
            let kind = match prefix.as_str() {
                "=" => Some(BlockKind::Heading(1)),
                "==" => Some(BlockKind::Heading(2)),
                "===" => Some(BlockKind::Heading(3)),
                "-" => Some(BlockKind::Bullet),
                "+" | "1." => Some(BlockKind::Numbered),
                "/" => Some(BlockKind::Description),
                ">" => Some(BlockKind::Quote),
                _ => None,
            };
            if let Some(kind) = kind {
                let mut before = self.snapshot();
                before.document.view.insert_text(" ", range.start);
                before.document.sync_source();
                before.selection = editor::cursor(range.start + 1..range.start + 1).into();
                self.session
                    .document
                    .view
                    .delete_char_range(start..range.start);
                self.session.document.view.set_block(start..start, kind);
                self.selection = editor::cursor(start..start);
                self.force_selection = true;
                self.remember(before, false);
                ctx.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e,egui::Event::Text(t) if t==" "))
                });
                return;
            }
        }
        if self.session.document.view.blocks[line].is_letter_note() {
            if range.is_empty()
                && lines[line].is_empty()
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                let before = self.snapshot();
                self.session.document.view.blocks[line] = BlockKind::Paragraph;
                self.remember(before, false);
                self.focus_annotation_target(start..start);
                return;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                let before = self.snapshot();
                let (_, group) = self
                    .session
                    .document
                    .view
                    .letter_note_groups()
                    .into_iter()
                    .find(|(_, lines)| lines.contains(&line))
                    .expect("bloc de courrier");
                let at = lines[group.end - 1].end;
                self.session.document.view.insert_text("\n", at);
                let next = self.session.document.view.line_at(at + 1);
                self.session.document.view.blocks[next] = BlockKind::Paragraph;
                self.remember(before, false);
                self.focus_annotation_target(at + 1..at + 1);
                return;
            }
        }
        if self.session.document.view.blocks[line] == BlockKind::Quote
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            let before = self.snapshot();
            if lines[line].is_empty() && range.is_empty() {
                let group = self
                    .session
                    .document
                    .view
                    .quote_groups()
                    .into_iter()
                    .find(|group| group.body.contains(&line))
                    .expect("paragraphe de citation");
                // La terminaison du corps cité reste une règle propre à la citation.
                // Retirer sa ligne vide, puis utiliser la sortie d’annotation commune.
                if group.body.len() > 1 {
                    self.session
                        .document
                        .view
                        .delete_char_range(start..start + 1);
                    self.session.document.view.blocks
                        [group.attribution.expect("attribution éditable") - 1] =
                        BlockKind::QuoteAttribution;
                }
                let annotation = self
                    .session
                    .document
                    .view
                    .annotation_at(start..start)
                    .expect("annotation de citation");
                let at = self.session.document.view.leave_annotation(&annotation);
                self.remember(before, false);
                self.focus_annotation_target(at..at);
            } else {
                self.session.document.view.delete_char_range(range.clone());
                self.session.document.view.insert_text("\n", range.start);
                self.selection = editor::cursor(range.start + 1..range.start + 1);
                self.remember(before, false);
                self.force_selection = true;
            }
            return;
        }
        if self.session.document.view.blocks[line] == BlockKind::Description {
            let text = &self.session.document.view.text[self
                .session
                .document
                .view
                .byte_index(lines[line].start)
                ..self.session.document.view.byte_index(lines[line].end)];
            let separator = text.chars().position(|c| c == '\t').map(|at| start + at);
            let shift = ctx.input(|i| i.events.iter().any(|event| matches!(event,
                egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } if modifiers.shift
            )));
            if ctx.input_mut(|i| {
                i.consume_key(
                    if shift {
                        egui::Modifiers::SHIFT
                    } else {
                        egui::Modifiers::NONE
                    },
                    egui::Key::Tab,
                )
            }) {
                // egui prépare la navigation du focus au début de la frame,
                // avant que l’éditeur ne consomme la touche.
                ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                if shift {
                    self.selection = editor::cursor(start..start);
                } else if let Some(at) = separator {
                    self.selection = editor::cursor(at + 1..at + 1);
                } else {
                    let before = self.snapshot();
                    self.session.document.view.insert_text("\t", range.start);
                    self.selection = editor::cursor(range.start + 1..range.start + 1);
                    self.remember(before, false);
                }
                self.force_selection = true;
                return;
            }
            if (separator.is_none() || separator == Some(range.start))
                && prefix.ends_with(':')
                && typed.as_deref() == Some(" ")
            {
                let mut before = self.snapshot();
                before.document.view.insert_text(" ", range.start);
                before.document.sync_source();
                before.selection = editor::cursor(range.start + 1..range.start + 1).into();
                self.session
                    .document
                    .view
                    .delete_char_range(range.start - 1..range.start);
                if separator.is_none() {
                    self.session
                        .document
                        .view
                        .insert_text("\t", range.start - 1);
                }
                self.selection = editor::cursor(range.start..range.start);
                self.force_selection = true;
                self.remember(before, false);
                ctx.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e, egui::Event::Text(t) if t == " "))
                });
                return;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                let before = self.snapshot();
                if text.trim().is_empty() && range.is_empty() {
                    self.session
                        .document
                        .view
                        .delete_char_range(lines[line].clone());
                    self.session
                        .document
                        .view
                        .set_block(start..start, BlockKind::Paragraph);
                    self.selection = editor::cursor(start..start);
                } else {
                    let in_text = separator.is_some_and(|at| range.start > at);
                    self.session.document.view.delete_char_range(range.clone());
                    self.session
                        .document
                        .view
                        .insert_text(if in_text { "\n\t" } else { "\n" }, range.start);
                    self.selection = editor::cursor(range.start + 1..range.start + 1);
                }
                self.remember(before, false);
                self.force_selection = true;
                return;
            }
        }
        if !range.is_empty() {
            return;
        }
        if typed.as_deref() == Some("*") || typed.as_deref() == Some("_") {
            let delimiter = typed.as_ref().and_then(|s| s.chars().next()).unwrap_or('*');
            let chars: Vec<_> = prefix.chars().collect();
            if let Some(position) = chars.iter().rposition(|c| *c == delimiter)
                && position + 1 < chars.len()
                && (position == 0 || chars[position - 1].is_whitespace())
            {
                let mut before = self.snapshot();
                before
                    .document
                    .view
                    .insert_text(&delimiter.to_string(), range.start);
                before.document.sync_source();
                before.selection = editor::cursor(range.start + 1..range.start + 1).into();
                self.session
                    .document
                    .view
                    .delete_char_range(start + position..start + position + 1);
                let mark = if delimiter == '*' {
                    Mark::Bold
                } else {
                    Mark::Italic
                };
                let selected = start + position..range.start - 1;
                if !self
                    .session
                    .document
                    .view
                    .mark_state(selected.clone(), mark)
                    .0
                {
                    self.session.document.view.toggle_mark(selected, mark);
                }
                self.session.document.view.typing_style = InlineStyle::default();
                self.selection = editor::cursor(range.start - 1..range.start - 1);
                self.force_selection = true;
                self.remember(before, false);
                ctx.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e,egui::Event::Text(t) if t==&delimiter.to_string()))
                });
            }
        }
        if lines[line].is_empty()
            && matches!(
                self.session.document.view.blocks[line],
                BlockKind::Bullet | BlockKind::Numbered | BlockKind::Description
            )
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.apply(Command::Block(BlockKind::Paragraph));
        }
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        if let Some(command) =
            crate::toolbar::show(ui, &self.session.document.view, self.range(), &self.keymap)
        {
            self.apply(command);
        }
    }
    fn status_ui(&mut self, ui: &mut egui::Ui) {
        let range = self.range();
        let (total, selected) =
            self.statistics
                .counts(self.session.revision, &self.session.document.view, range);
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::tr!(
                "document-stats",
                words = total.words,
                signs = total.signs
            ))
            .on_hover_text(crate::tr!(
                "statistics-hint",
                without_spaces = total.signs_without_spaces
            ));
            if let Some(selected) = selected {
                ui.separator();
                ui.label(crate::tr!(
                    "selection-stats",
                    words = selected.words,
                    signs = selected.signs
                ))
                .on_hover_text(crate::tr!(
                    "statistics-hint",
                    without_spaces = selected.signs_without_spaces
                ));
            }
            if self.preferences.spelling.enabled {
                ui.separator();
                let language = self.session.metadata.options().language;
                if !crate::spelling::supported(language) {
                    ui.weak(crate::tr!(
                        "spelling-no-dictionary",
                        language = language.label()
                    ))
                    .on_hover_text(crate::tr!("spelling-dictionaries"));
                } else if let Some(error) = &self.spelling.error {
                    ui.colored_label(ui.visuals().error_fg_color, crate::tr!("spelling-error"))
                        .on_hover_text(error);
                } else if self.spelling.ready {
                    let response = ui.add(
                        egui::Button::new(crate::tr!(
                            "spelling-count",
                            count = self.spelling.issues.len()
                        ))
                        .frame(false),
                    );
                    let clicked = response
                        .on_hover_text(crate::tr!("spelling-hint"))
                        .clicked();
                    if clicked {
                        self.apply(Command::NextSpelling);
                    }
                } else {
                    ui.weak(crate::tr!("spelling-checking"));
                }
            }
            if let Some(message) = &self.message {
                ui.separator();
                ui.add(egui::Label::new(message).truncate())
                    .on_hover_text(message);
            }
        });
    }
    fn information_ui(&mut self, ui: &mut egui::Ui) {
        let before = self.snapshot();
        let old_revision = self.session.revision;
        let groups: Vec<_> =
            std::iter::once((InformationGroup::General, crate::tr!("group-general")))
                .chain(self.session.metadata.editor_groups().iter().map(|group| {
                    (
                        InformationGroup::Fields(group.id),
                        crate::i18n::text(group.label_key),
                    )
                }))
                .collect();
        let full_heading = match self.session.metadata.document_type() {
            DocumentType::Courrier => crate::tr!("document-settings-heading"),
            DocumentType::CompteRendu => crate::tr!("minutes-settings-heading"),
        };
        let font = egui::TextStyle::Button.resolve(ui.style());
        let label_width = |label: &str| {
            ui.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(label.to_owned(), font.clone(), ui.visuals().text_color())
                    .size()
                    .x
            })
        };
        let controls_width: f32 = groups
            .iter()
            .map(|(_, label)| label_width(label) + 2.0 * ui.spacing().button_padding.x)
            .sum();
        let heading = if label_width(&full_heading)
            + controls_width
            + groups.len() as f32 * ui.spacing().item_spacing.x
            <= ui.available_width()
        {
            full_heading.clone()
        } else {
            crate::tr!("document-settings-short")
        };
        let mut clicked = None;
        ui.scope(|ui| {
            crate::toolbar::neutral_selection(ui);
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Label::new(heading).truncate())
                    .on_hover_text(&full_heading);
                for (group, label) in groups {
                    let active = self.information_open && self.information_group == group;
                    if ui
                        .add(egui::Button::selectable(active, label.clone()).small())
                        .on_hover_text(if active {
                            crate::tr!("document-settings-close")
                        } else {
                            label
                        })
                        .clicked()
                    {
                        clicked = Some(group);
                    }
                }
            });
        });
        if let Some(group) = clicked {
            let close = self.information_open && self.information_group == group;
            if let Some(id) = self.history_field.take() {
                ui.ctx().memory_mut(|memory| memory.surrender_focus(id));
            }
            self.information_group = group;
            if close || !self.information_open {
                self.apply(Command::Information);
            }
            ui.ctx().request_repaint();
        }
        if self.information_open {
            // Une seule rubrique est visible ; chaque rubrique garde son défilement.
            let max_height = (ui.available_height() * 0.45).min(320.0);
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt(("information-scroll", self.information_group))
                .max_height(max_height)
                .auto_shrink([false, true])
                .show(ui, |ui| match self.information_group {
                    InformationGroup::General => self.general_information_ui(ui),
                    InformationGroup::Fields(group) => {
                        self.session.metadata.editor_group_ui(ui, group);
                        if group == "sender" {
                            self.document_signature_ui(ui);
                        }
                    }
                });
        }
        if self.session.revision == old_revision {
            let focused = ui.ctx().memory(|memory| memory.focused());
            let group = focused
                .filter(|id| egui::TextEdit::load_state(ui.ctx(), *id).is_some())
                .map_or(EditGroup::Discrete, |id| EditGroup::Metadata(id.value()));
            self.selection = self
                .session
                .record(before, self.selection.into(), group)
                .into();
        }
    }
    fn general_information_ui(&mut self, ui: &mut egui::Ui) {
        let fixed_page = crate::document_types::models::ALL
            .iter()
            .find(|model| model.source == self.session.template)
            .and_then(|model| model.page);
        egui::Grid::new("general-fields")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(crate::tr!("field-document-type"));
                ui.weak(self.keymap.registry().document_type().label());
                ui.end_row();
                ui.label(crate::tr!("field-template"));
                self.template_selector(ui);
                ui.end_row();
                ui.label(crate::tr!("field-paper"));
                if let Some((paper, _)) = fixed_page {
                    ui.label(paper.label())
                        .on_hover_text(crate::tr!("model-fixed-page"));
                } else {
                    egui::ComboBox::from_id_salt("document-paper")
                        .selected_text(self.session.metadata.options().paper.label())
                        .show_ui(ui, |ui| {
                            for paper in PaperFormat::ALL {
                                ui.selectable_value(
                                    &mut self.session.metadata.options_mut().paper,
                                    paper,
                                    paper.label(),
                                );
                            }
                        });
                }
                ui.end_row();
                ui.label(crate::tr!("field-orientation"));
                if let Some((_, landscape)) = fixed_page {
                    ui.label(crate::i18n::text(if landscape {
                        "orientation-landscape"
                    } else {
                        "orientation-portrait"
                    }))
                    .on_hover_text(crate::tr!("model-fixed-page"));
                } else {
                    egui::ComboBox::from_id_salt("document-orientation")
                        .selected_text(if self.session.metadata.options().landscape {
                            crate::tr!("orientation-landscape")
                        } else {
                            crate::tr!("orientation-portrait")
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.session.metadata.options_mut().landscape,
                                false,
                                crate::tr!("orientation-portrait"),
                            );
                            ui.selectable_value(
                                &mut self.session.metadata.options_mut().landscape,
                                true,
                                crate::tr!("orientation-landscape"),
                            );
                        });
                }
                ui.end_row();
                ui.label(crate::tr!("field-document-language"));
                egui::ComboBox::from_id_salt("document-language")
                    .selected_text(self.session.metadata.options().language.label())
                    .show_ui(ui, |ui| {
                        for language in DocumentLanguage::ALL {
                            ui.selectable_value(
                                &mut self.session.metadata.options_mut().language,
                                language,
                                language.label(),
                            );
                        }
                    });
                ui.end_row();
                ui.label(crate::tr!("field-bullet-marker"));
                self.bullet_marker_ui(ui);
                ui.end_row();
            });
    }
    fn bullet_marker_ui(&mut self, ui: &mut egui::Ui) {
        let policy = &self.presentation;
        if policy.bullet_markers.is_empty() && !policy.custom_bullet_marker {
            if self.session.metadata.options().bullet_marker.is_some() {
                if ui
                    .button(crate::tr!("bullet-marker-restore-automatic"))
                    .clicked()
                {
                    self.session.metadata.options_mut().bullet_marker = None;
                }
            } else {
                ui.weak(crate::tr!("bullet-marker-automatic"))
                    .on_hover_text(crate::tr!("bullet-marker-model-controlled"));
            }
            return;
        }
        let marker = &mut self.session.metadata.options_mut().bullet_marker;
        ui.vertical(|ui| {
            egui::ComboBox::from_id_salt("document-bullet-marker")
                .selected_text(
                    marker
                        .as_deref()
                        .map_or_else(|| crate::tr!("bullet-marker-automatic"), str::to_owned),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(marker, None, crate::tr!("bullet-marker-automatic"));
                    for choice in &policy.bullet_markers {
                        ui.selectable_value(marker, Some(choice.clone()), choice);
                    }
                    if policy.custom_bullet_marker
                        && ui
                            .selectable_label(marker.is_some(), crate::tr!("bullet-marker-custom"))
                            .clicked()
                    {
                        *marker = Some("◆".into());
                    }
                });
            if policy.custom_bullet_marker
                && let Some(value) = marker
            {
                ui.add(
                    egui::TextEdit::singleline(value)
                        .id_salt("custom-bullet-marker")
                        .desired_width(55.0),
                );
                if !crate::document::valid_bullet_marker(value) {
                    ui.colored_label(
                        Color32::from_rgb(183, 69, 57),
                        crate::tr!("bullet-marker-invalid"),
                    );
                }
            }
        })
        .response
        .on_hover_text(crate::tr!("bullet-marker-model-choices"));
    }
    fn template_selector(&mut self, ui: &mut egui::Ui) {
        let commands: Vec<_> = self
            .keymap
            .registry()
            .definitions()
            .filter(|d| d.in_model_menu())
            .map(|d| d.command)
            .collect();
        let current = commands.iter().copied().find(|command| {
            command.model_source().as_deref() == Some(self.session.template.as_str())
        });
        let mut selected = current;
        egui::ComboBox::from_id_salt("template")
            .selected_text(
                current.map_or_else(|| crate::tr!("embedded-template"), Command::short_label),
            )
            .show_ui(ui, |ui| {
                if current.is_none() {
                    ui.selectable_value(&mut selected, None, crate::tr!("embedded-template"));
                }
                for command in commands {
                    ui.selectable_value(&mut selected, Some(command), command.short_label())
                        .on_hover_text(self.keymap.hint(ui.ctx(), command));
                }
            });
        if selected != current
            && let Some(command) = selected
        {
            self.apply(command);
        }
    }
    fn refresh_search(&mut self) {
        self.search
            .refresh(self.session.revision, &self.session.document.view);
    }
    /// Diagnostic de présentation utilisant les mêmes commandes que l’interface.
    pub fn capture_search(&mut self, query: String) {
        self.load_demo();
        self.apply(Command::FindReplace);
        self.search.query = query;
        self.apply(Command::FindNext);
    }
    pub fn capture_creation(&mut self, ctx: &egui::Context) {
        self.perform_action(PendingAction::New, ctx);
    }
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
    fn close_search(&mut self) {
        self.search_open = false;
        self.search_focus_pending = false;
        self.force_selection = true;
    }
    fn search_focus(&self, ctx: &egui::Context) -> Option<egui::Id> {
        ctx.memory(|memory| memory.focused()).filter(|id| {
            self.search_open
                && [SEARCH_ID, REPLACE_ID]
                    .iter()
                    .any(|name| *id == egui::Id::new(name))
        })
    }
    fn select_search_result(&mut self, index: usize) {
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
    fn replace_search(&mut self, all: bool) {
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
    fn search_ui(&mut self, ui: &mut egui::Ui) {
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
    fn refresh_spelling(&mut self, ctx: &egui::Context) {
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
    fn selected_spelling_issue(&self) -> Option<crate::spelling::Issue> {
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
    fn valid_spelling_target(&self, revision: u64, issue: &crate::spelling::Issue) -> bool {
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
    fn correct_spelling(
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
    fn open_spelling_target(&mut self, issue: crate::spelling::Issue) {
        self.spelling
            .request_suggestions(self.session.metadata.options().language, &issue.word);
        self.selection = editor::cursor(issue.range.clone());
        self.force_selection = true;
        self.forward_pending = true;
        self.history_field = None;
        self.session.typing_group = false;
        self.spelling_target = Some((self.session.revision, issue));
    }
    fn spelling_context_menu(&mut self, output: &egui::text_edit::TextEditOutput) {
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
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        self.toolbar(ui);
        ui.separator();
        self.information_ui(ui);
        ui.separator();
        self.search_ui(ui);
        self.refresh_spelling(ui.ctx());
        let before = self.snapshot();
        let old_range = self.selection;
        if self.force_selection {
            let search_focus = self.search_focus(ui.ctx());
            editor::restore_cursor(ui.ctx(), self.selection);
            if let Some(id) = search_focus {
                ui.memory_mut(|memory| memory.request_focus(id));
            }
            self.force_selection = false;
        }
        for image in self.session.document.view.images.values() {
            if !self.image_textures.contains_key(&image.path)
                && let Some(data) = self.session.document.assets.get(&image.path)
                && let Ok(thumbnail) =
                    crate::assets::thumbnail(data, image.path.rsplit('.').next().unwrap())
            {
                self.image_textures.insert(
                    image.path.clone(),
                    ui.ctx()
                        .load_texture(&image.path, thumbnail, egui::TextureOptions::LINEAR),
                );
            }
        }
        let view = self.session.document.view.clone();
        let lock_tab = view.annotation_at(self.range()).is_some()
            || (view.blocks[view.line_at(self.selection.primary.index)] == BlockKind::Description
                || view.blocks[view.line_at(self.selection.primary.index)].is_letter_note());
        let editor_style = self.preferences.editor;
        let matches = if self.search_open {
            self.search.matches.clone()
        } else {
            vec![]
        };
        let active = self.search.selected(&self.range());
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
            if matches.is_empty() {
                editor::layout(ui, &view, text.as_str(), width, editor_style)
            } else {
                editor::layout_with_search(
                    ui,
                    &view,
                    text.as_str(),
                    width,
                    editor_style,
                    &matches,
                    active,
                )
            }
        };
        let mut image_action = None;
        egui::ScrollArea::vertical()
            .id_salt("editor-scroll")
            .show(ui, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        (editor_style.size * 22.0 / 18.0).round() as i8,
                        12,
                    ))
                    .show(ui, |ui| {
                        let background = ui.painter().add(egui::Shape::Noop);
                        let output = egui::TextEdit::multiline(&mut self.session.document.view)
                            .lock_focus(lock_tab)
                            .id(egui::Id::new(editor::EDITOR_ID))
                            .desired_width(f32::INFINITY)
                            .desired_rows(25)
                            .frame(false)
                            .font(editor_style.font.font_id(editor_style.size, 0))
                            .layouter(&mut layouter)
                            .show(ui);
                        let clicked_term = editor::paint_decorations(
                            ui,
                            &self.session.document.view,
                            &output,
                            editor_style,
                            background,
                        );
                        if let Some(range) = output.cursor_range {
                            self.selection = range;
                        }
                        if self.search_open
                            && output.response.clicked()
                            && let Some(pointer) = output.response.interact_pointer_pos()
                            && let Some((index, _)) = editor::range_rects(&output.galley, &matches)
                                .iter()
                                .find(|(_, rect)| {
                                    rect.translate(output.galley_pos.to_vec2())
                                        .contains(pointer)
                                })
                        {
                            self.selection = editor::cursor(matches[*index].clone());
                            self.force_selection = true;
                        }
                        if let Some(at) = clicked_term {
                            self.selection = editor::cursor(at..at);
                            self.force_selection = true;
                        }
                        if view.text == self.session.document.view.text {
                            let editing = output.response.has_focus()
                                && self.selection.is_empty()
                                && self.session.changed_at.elapsed() < Duration::from_millis(700);
                            let ranges: Vec<_> = self
                                .spelling
                                .issues
                                .iter()
                                .filter(|issue| {
                                    !editing
                                        || !(issue.range.start <= self.selection.primary.index
                                            && self.selection.primary.index <= issue.range.end)
                                })
                                .map(|issue| issue.range.clone())
                                .collect();
                            editor::paint_spelling(ui, &output, &ranges);
                            self.spelling_context_menu(&output);
                            if editing {
                                ui.ctx().request_repaint_after(
                                    Duration::from_millis(700)
                                        .saturating_sub(self.session.changed_at.elapsed()),
                                );
                            }
                        }
                        if self.editor_scroll_pending {
                            if self.search_focus(ui.ctx()).is_none() {
                                output.response.request_focus();
                            }
                            let cursor = output.galley.pos_from_cursor(self.selection.primary);
                            ui.scroll_to_rect(
                                cursor.translate(output.galley_pos.to_vec2()),
                                Some(egui::Align::Center),
                            );
                            self.editor_scroll_pending = false;
                        }
                        image_action = editor::paint_images(
                            ui,
                            &self.session.document.view,
                            &output,
                            &self.image_textures,
                            self.range(),
                            editor_style,
                        );
                        let changed = output.response.changed();
                        let mut state = output.state;
                        if self.force_selection {
                            state.cursor.set_char_range(Some(self.selection));
                        }
                        state.clear_undoer();
                        state.store(ui.ctx(), output.response.id);
                        if changed {
                            self.remember(before.clone(), true);
                            if self.search_open {
                                self.refresh_search();
                            }
                            ui.ctx().request_repaint();
                        } else if self.selection != old_range {
                            let selected = self.range();
                            let pos = self.selection.primary.index;
                            self.session.document.view.typing_style = if !selected.is_empty() {
                                self.session.document.view.style_at(selected.start)
                            } else if pos > 0 {
                                self.session.document.view.style_at(pos - 1)
                            } else {
                                InlineStyle::default()
                            };
                            self.session.typing_group = false;
                        }
                        if self.selection != old_range {
                            self.forward_pending = true;
                            ui.ctx().request_repaint();
                        }
                    });
            });
        if let Some((id, action)) = image_action {
            match action {
                editor::ImageAction::Select(at) => {
                    self.selection = editor::cursor(at..at + 1);
                    self.force_selection = true;
                    self.forward_pending = true;
                }
                editor::ImageAction::Replace => self.choose_image(Some(id)),
                editor::ImageAction::Information => {
                    self.image_alt
                        .clone_from(&self.session.document.view.images[&id].alt);
                    self.image_info = Some(id);
                }
                editor::ImageAction::Remove => self.remove_image(id),
            }
        }
    }
    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::tr!("preview-title")).strong())
                .on_hover_text(crate::tr!("preview-hint"));
            if self.rendered_revision != Some(self.session.revision) {
                ui.spinner();
                ui.weak(crate::tr!("preview-updating"));
            } else if let Some(time) = self.rendering_time {
                let time = if self.capture.is_some() {
                    Duration::ZERO
                } else {
                    time
                };
                ui.weak(crate::tr!(
                    "preview-stats",
                    pages = self.pages.len(),
                    milliseconds = time.as_millis() as f64
                ));
            }
        });
        ui.separator();
        if let Some(error) = &self.rendering_error {
            ui.colored_label(Color32::from_rgb(183, 69, 57), crate::tr!("preview-failed"));
            ui.collapsing(crate::tr!("error-details"), |ui| {
                ui.label(error);
            });
        }
        let current = self.sync_is_current();
        let highlights = if current {
            self.sync
                .as_ref()
                .map(|sync| sync.selection_highlights(self.range()))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let target = if self.forward_pending && current {
            self.forward_pending = false;
            self.sync
                .as_ref()
                .and_then(|sync| sync.forward(self.selection.primary.index))
        } else {
            None
        };
        let mut clicked = None;
        egui::ScrollArea::vertical()
            .id_salt("preview-scroll")
            .show(ui, |ui| {
                let width = (ui.available_width() - 24.0).max(1.0);
                if self.pages.is_empty() {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.label(crate::tr!("preview-composing"));
                    });
                }
                for (index, page) in self.pages.iter().enumerate() {
                    let size = page.size_vec2();
                    let displayed = egui::vec2(width, width * size.y / size.x);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        let response = ui.add(
                            egui::Image::new(page)
                                .fit_to_exact_size(displayed)
                                .sense(egui::Sense::click()),
                        );
                        if let Some(size) =
                            self.sync.as_ref().and_then(|sync| sync.page_size(index))
                        {
                            let painter = ui
                                .painter()
                                .with_clip_rect(ui.clip_rect().intersect(response.rect));
                            for highlight in highlights
                                .iter()
                                .filter(|highlight| highlight.page == index)
                            {
                                let points = highlight
                                    .corners
                                    .iter()
                                    .map(|point| {
                                        response.rect.min
                                            + egui::vec2(
                                                point[0] / size[0] * response.rect.width(),
                                                point[1] / size[1] * response.rect.height(),
                                            )
                                    })
                                    .collect();
                                painter.add(egui::Shape::convex_polygon(
                                    points,
                                    PREVIEW_SELECTION,
                                    egui::Stroke::NONE,
                                ));
                            }
                            if let Some(target) = target.filter(|target| target.page == index) {
                                let point = response.rect.min
                                    + egui::vec2(
                                        target.point[0] / size[0] * displayed.x,
                                        target.point[1] / size[1] * displayed.y,
                                    );
                                let passage =
                                    egui::Rect::from_center_size(point, egui::vec2(8.0, 32.0));
                                if !ui.clip_rect().contains_rect(passage) {
                                    ui.scroll_to_rect(passage, Some(egui::Align::Center));
                                }
                            }
                            if current
                                && response.double_clicked()
                                && let Some(pointer) = response.interact_pointer_pos()
                            {
                                let relative = pointer - response.rect.min;
                                clicked = Some(PagePoint {
                                    page: index,
                                    point: [
                                        relative.x / displayed.x * size[0],
                                        relative.y / displayed.y * size[1],
                                    ],
                                });
                            }
                        }
                    });
                    ui.add_space(10.0);
                }
            });
        if let Some(location) = clicked {
            self.navigate_backward(location, ui.ctx());
        }
    }
    fn sync_is_current(&self) -> bool {
        self.rendering_error.is_none()
            && self
                .sync
                .as_ref()
                .is_some_and(|sync| sync.revision == self.session.revision)
    }
    fn navigate_backward(&mut self, location: PagePoint, ctx: &egui::Context) {
        if !self.sync_is_current() {
            return;
        }
        if let Some(character) = self.sync.as_ref().and_then(|sync| sync.backward(location)) {
            self.selection = CCursorRange::one(CCursor::new(
                character.min(self.session.document.view.len()),
            ));
            self.session.document.view.typing_style = if character > 0 {
                self.session.document.view.style_at(character - 1)
            } else {
                InlineStyle::default()
            };
            self.force_selection = true;
            self.editor_scroll_pending = true;
            self.forward_pending = false;
            self.session.typing_group = false;
            ctx.request_repaint();
        }
    }
    fn preferences_ui(&mut self, ctx: &egui::Context) {
        if !self.preferences_open {
            return;
        }
        let response = egui::Modal::new(egui::Id::new("preferences")).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.heading(crate::tr!("preferences-title"));
            ui.horizontal(|ui| {
                for (page, label) in [
                    (PreferencesPage::Profile, crate::tr!("preferences-profile")),
                    (
                        PreferencesPage::Appearance,
                        crate::tr!("preferences-appearance"),
                    ),
                    (
                        PreferencesPage::Spelling,
                        crate::tr!("preferences-spelling"),
                    ),
                ] {
                    ui.selectable_value(&mut self.preferences_page, page, label);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("preferences-content")
                .max_height((ctx.content_rect().height() - 180.0).max(160.0))
                .show(ui, |ui| match self.preferences_page {
                    PreferencesPage::Profile => self.profile_preferences_ui(ui),
                    PreferencesPage::Appearance => self.appearance_preferences_ui(ui),
                    PreferencesPage::Spelling => self.spelling_preferences_ui(ui),
                });
            ui.add_space(8.0);
            if ui.button(crate::tr!("close")).clicked() {
                self.preferences_open = false;
            }
        });
        ctx.set_theme(self.preferences.appearance.theme());
        if response.should_close() {
            self.preferences_open = false;
        }
        if !self.preferences_open {
            self.force_selection = true;
        }
    }
    fn appearance_preferences_ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("appearance-preferences")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(crate::tr!("editor-font"));
                egui::ComboBox::from_id_salt("editor-font")
                    .selected_text(self.preferences.editor.font.label())
                    .show_ui(ui, |ui| {
                        for font in EditorFont::ALL {
                            ui.selectable_value(
                                &mut self.preferences.editor.font,
                                font,
                                font.label(),
                            );
                        }
                    });
                ui.end_row();
                ui.label(crate::tr!("editor-font-size"));
                ui.add(
                    egui::Slider::new(&mut self.preferences.editor.size, 10.0..=36.0).step_by(1.0),
                );
                ui.end_row();
                ui.label(crate::tr!("appearance"));
                egui::ComboBox::from_id_salt("appearance")
                    .selected_text(self.preferences.appearance.label())
                    .show_ui(ui, |ui| {
                        for appearance in Appearance::ALL {
                            ui.selectable_value(
                                &mut self.preferences.appearance,
                                appearance,
                                appearance.label(),
                            );
                        }
                    });
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.weak(crate::tr!("preferences-help"));
    }
    fn spelling_preferences_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.preferences.spelling.enabled,
            crate::tr!("cmd-spelling-toggle"),
        );
        ui.weak(crate::tr!("spelling-dictionaries"));
        egui::CollapsingHeader::new(crate::tr!("spelling-personal-dictionary")).show(ui, |ui| {
            let mut remove = None;
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for (language, words) in &self.preferences.spelling.personal {
                        for word in words {
                            ui.horizontal(|ui| {
                                ui.label(format!("{word} ({language})"));
                                if ui
                                    .small_button(crate::tr!("spelling-forget-word"))
                                    .clicked()
                                {
                                    remove = Some((language.clone(), word.clone()));
                                }
                            });
                        }
                    }
                });
            if let Some((language, word)) = remove {
                self.preferences
                    .spelling
                    .personal
                    .get_mut(&language)
                    .unwrap()
                    .remove(&word);
            }
            ui.weak(crate::tr!("spelling-personal-help"));
        });
    }
    fn profile_preferences_ui(&mut self, ui: &mut egui::Ui) {
        ui.weak(crate::tr!("profile-help"));
        ui.add_space(8.0);
        egui::Grid::new("profile-fields")
            .num_columns(2)
            .show(ui, |ui| {
                let profile = &mut self.preferences.profile;
                for (key, value, multiline) in [
                    ("field-full-name", &mut profile.name, false),
                    ("field-organisation", &mut profile.organisation, false),
                    ("field-role", &mut profile.role, false),
                    ("field-sender-address", &mut profile.address, true),
                    ("field-email", &mut profile.email, false),
                    ("field-phone", &mut profile.phone, false),
                ] {
                    let label = ui.label(crate::i18n::text(key));
                    let edit = if multiline {
                        egui::TextEdit::multiline(value).desired_rows(3)
                    } else {
                        egui::TextEdit::singleline(value)
                    };
                    ui.add(edit.id_salt(("profile", key)).desired_width(310.0))
                        .labelled_by(label.id);
                    ui.end_row();
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(crate::tr!("signature-label"));
            if ui
                .button(if self.preferences.profile.signature.is_some() {
                    crate::tr!("image-replace")
                } else {
                    crate::tr!("signature-choose")
                })
                .clicked()
            {
                self.choose_signature(true);
            }
            if self.preferences.profile.signature.is_some()
                && ui.button(crate::tr!("image-remove")).clicked()
            {
                self.preferences.profile.signature = None;
                self.profile_signature = None;
                self.profile_error = None;
            }
        });
        if let (Some(name), Some(image)) = (
            self.preferences.profile.signature.clone(),
            self.profile_signature.clone(),
        ) {
            self.signature_preview(ui, &format!("profile:{name}"), &image);
        } else if self.preferences.profile.signature.is_none() {
            ui.weak(crate::tr!("signature-none"));
        }
        ui.weak(crate::tr!("profile-signature-help"));
        if let Some(error) = &self.profile_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
    fn persist_preferences(
        &mut self,
        storage: &mut dyn eframe::Storage,
        now: Instant,
        force: bool,
    ) {
        if self.capture.is_some() {
            return;
        }
        if let Some(delay) = self.preferences_tracker.observe(&self.preferences, now)
            && (force || delay.is_zero())
        {
            self.preferences_tracker.save(&self.preferences, storage);
        }
    }
    /// La proportion appartient aux préférences, les rectangles à la frame.
    /// Seuls les bords gardent une petite zone accessible pour chaque colonne.
    fn columns_ui(&mut self, ui: &mut egui::Ui, disabled: bool) -> [egui::Rect; 2] {
        let bounds = ui.available_rect_before_wrap();
        let divider_width = 8.0;
        let usable = (bounds.width() - divider_width).max(1.0);
        let minimum = 48.0_f32.min(usable * 0.25);
        let mut left_width =
            (usable * self.preferences.editor_fraction).clamp(minimum, usable - minimum);
        let divider = egui::Rect::from_min_size(
            egui::pos2(bounds.left() + left_width, bounds.top()),
            egui::vec2(divider_width, bounds.height()),
        );
        let response = ui.interact(
            divider,
            ui.id().with("column-divider"),
            if disabled {
                egui::Sense::hover()
            } else {
                egui::Sense::click_and_drag()
            },
        );
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            left_width =
                (pointer.x - bounds.left() - divider_width / 2.0).clamp(minimum, usable - minimum);
            self.preferences.editor_fraction = left_width / usable;
        } else if response.double_clicked() {
            self.preferences.editor_fraction = 0.5;
            left_width = usable * 0.5;
        }
        let editor_rect = egui::Rect::from_min_max(
            bounds.min,
            egui::pos2(bounds.left() + left_width, bounds.bottom()),
        );
        let preview_rect = egui::Rect::from_min_max(
            egui::pos2(editor_rect.right() + divider_width, bounds.top()),
            bounds.max,
        );
        for (id, rect, preview) in [
            ("preview-column", preview_rect, true),
            ("editor-column", editor_rect, false),
        ] {
            ui.scope_builder(egui::UiBuilder::new().id_salt(id).max_rect(rect), |ui| {
                ui.set_clip_rect(rect);
                if disabled {
                    ui.disable();
                }
                egui::Frame::central_panel(ui.style()).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    if preview {
                        self.preview_ui(ui);
                    } else {
                        self.editor_ui(ui);
                    }
                });
            });
        }
        let stroke = if response.hovered() || response.dragged() {
            ui.visuals().widgets.hovered.fg_stroke
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };
        ui.painter().vline(
            editor_rect.right() + divider_width / 2.0,
            bounds.y_range(),
            stroke,
        );
        response
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
            .on_hover_text(crate::tr!("column-divider-hint"));
        [editor_rect, preview_rect]
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.new_document_open {
            let mut create = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("new-document")).show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.heading(crate::tr!("new-document-title"));
                ui.label(crate::tr!("field-document-type"));
                for kind in DocumentType::ALL {
                    ui.add_enabled_ui(kind.can_create(), |ui| {
                        if ui
                            .radio_value(&mut self.new_document_type, kind, kind.label())
                            .changed()
                        {
                            self.new_document_model = kind.models().next().map(|entry| entry.model);
                        }
                    })
                    .response
                    .on_disabled_hover_text(crate::tr!("type-no-model"));
                }
                ui.add_space(8.0);
                ui.label(crate::tr!("field-template"));
                for entry in self.new_document_type.models() {
                    ui.radio_value(
                        &mut self.new_document_model,
                        Some(entry.model),
                        entry.command.command.short_label(),
                    );
                }
                ui.separator();
                ui.horizontal(|ui| {
                    cancel = ui.button(crate::tr!("cancel")).clicked();
                    let valid = self
                        .new_document_model
                        .is_some_and(|model| model.definition().kind == self.new_document_type);
                    create = ui
                        .add_enabled(valid, egui::Button::new(crate::tr!("new-document-create")))
                        .clicked();
                });
            });
            if create {
                self.create_document(self.new_document_type);
            } else if cancel || modal.should_close() {
                self.new_document_open = false;
            }
        }

        if let Some(id) = self.image_info {
            let mut open = true;
            let mut apply = false;
            let mut cancel = false;
            egui::Window::new(crate::tr!("image-information"))
                .open(&mut open)
                .collapsible(false)
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label(crate::tr!("image-alt-label"));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.image_alt)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3),
                    );
                    ui.label(
                        egui::RichText::new(crate::tr!("image-alt-help"))
                            .small()
                            .weak(),
                    );
                    ui.horizontal(|ui| {
                        apply = ui.button(crate::tr!("image-apply")).clicked();
                        cancel = ui.button(crate::tr!("image-cancel")).clicked();
                    });
                });
            if cancel {
                open = false;
            }
            if apply {
                let before = self.snapshot();
                if let Some(image) = self.session.document.view.images.get_mut(&id) {
                    image.alt.clone_from(&self.image_alt);
                    self.remember(before, false);
                }
                open = false;
                self.force_selection = true;
                self.editor_scroll_pending = true;
            }
            if !open {
                self.image_info = None;
                if let Some(range) = self.session.document.view.image_range(id) {
                    self.focus_annotation_target(range.start + 1..range.start + 1);
                }
            }
        }

        if self.link_open {
            let mut open = true;
            let mut apply = false;
            let mut remove = false;
            egui::Window::new(crate::tr!("link-title"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label(crate::tr!(
                        "link-selected-text",
                        text = self
                            .session
                            .document
                            .view
                            .text
                            .chars()
                            .skip(self.link_range.start)
                            .take(self.link_range.len())
                            .collect::<String>()
                    ));
                    ui.label(crate::tr!("field-address"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.link_url)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://…"),
                    );
                    if let Some(error) = &self.link_error {
                        ui.colored_label(Color32::from_rgb(183, 69, 57), error);
                    }
                    ui.horizontal(|ui| {
                        apply = ui.button(crate::tr!("apply")).clicked();
                        remove = ui
                            .button(Command::RemoveLink.short_label())
                            .on_hover_text(self.keymap.hint(ui.ctx(), Command::RemoveLink))
                            .clicked();
                    });
                });
            self.link_open &= open;
            if remove {
                if self.range() != self.link_range {
                    self.selection = editor::cursor(self.link_range.clone());
                }
                self.apply(Command::RemoveLink);
                self.link_open = false;
                self.force_selection = true;
            } else if apply {
                let url = self.link_url.trim();
                if (url.starts_with("https://")
                    || url.starts_with("http://")
                    || url.starts_with("mailto:"))
                    && !url.chars().any(char::is_whitespace)
                {
                    let before = self.snapshot();
                    self.session
                        .document
                        .view
                        .set_link(self.link_range.clone(), Some(url.into()));
                    self.remember(before, false);
                    self.link_open = false;
                    self.force_selection = true;
                } else {
                    self.link_error = Some(crate::tr!("link-invalid-url"));
                }
            }
        }
        if self.palette_open {
            let mut open = true;
            let mut choice = None;
            egui::Window::new(Command::Palette.label())
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(430.0)
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 85.0))
                .show(ctx, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text(crate::tr!("command-search"))
                            .desired_width(f32::INFINITY),
                    );
                    if !response.has_focus() {
                        response.request_focus();
                    }
                    if response.changed() {
                        self.palette_index = 0;
                    }
                    let commands = self
                        .keymap
                        .registry()
                        .definitions()
                        .filter(|definition| {
                            definition.in_palette()
                                && (self.document_open
                                    || definition.command.available_without_document())
                        })
                        .map(|definition| definition.command);
                    let query = self.palette_query.to_lowercase();
                    let filtered: Vec<_> = commands
                        .into_iter()
                        .filter(|c| c.label().to_lowercase().contains(&query))
                        .collect();
                    self.palette_index = self.palette_index.min(filtered.len().saturating_sub(1));
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
                        && !filtered.is_empty()
                    {
                        self.palette_index = (self.palette_index + 1) % filtered.len();
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
                        && !filtered.is_empty()
                    {
                        self.palette_index =
                            (self.palette_index + filtered.len() - 1) % filtered.len();
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                        choice = filtered.get(self.palette_index).copied();
                    }
                    for (index, command) in filtered.into_iter().enumerate() {
                        if ui
                            .add(
                                egui::Button::new(command.label())
                                    .frame(false)
                                    .selected(index == self.palette_index)
                                    .shortcut_text(self.keymap.shortcut_text(ui.ctx(), command)),
                            )
                            .clicked()
                        {
                            choice = Some(command);
                        }
                    }
                });
            self.palette_open &= open;
            if let Some(command) = choice {
                self.palette_open = false;
                self.dispatch_action(command, ctx);
            }
        }
        if self.export_open {
            let mut open = true;
            let mut export = false;
            egui::Window::new(crate::tr!("export-title"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(470.0)
                .show(ctx, |ui| {
                    ui.label(crate::tr!("export-folder"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.export_path)
                            .desired_width(f32::INFINITY),
                    );
                    ui.weak(crate::tr!("export-help"));
                    export = ui.button(crate::tr!("export-button")).clicked();
                });
            self.export_open &= open;
            if export {
                if !self.session.metadata.ready_for_export() {
                    self.message = Some(crate::tr!("export-required-fields"));
                } else {
                    match self
                        .worker
                        .sender
                        .send(self.request(Some(PathBuf::from(&self.export_path))))
                    {
                        Ok(()) => {
                            self.export_open = false;
                            self.message = Some(crate::tr!("export-pending"));
                        }
                        Err(_) => self.message = Some(crate::tr!("renderer-stopped")),
                    }
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.recovery_open = false;
            self.palette_open = false;
            self.link_open = false;
            self.information_open = false;
            self.export_open = false;
            self.pending_action = None;
            self.file_error = None;
            self.force_selection = true;
        }
    }
    fn file_dialogs(&mut self, ctx: &egui::Context) {
        if self.pending_action.is_some() {
            let mut choice = None;
            egui::Window::new(crate::tr!("unsaved-title"))
                .id(egui::Id::new("unsaved-letter"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(crate::tr!("unsaved-message"));
                    if let Some(error) = &self.file_error {
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
                    if let Some(action) = self.pending_action.take() {
                        self.perform_action(action, ctx);
                    }
                }
                Some(1) => {
                    self.clear_recovery();
                    self.file_error = None;
                    if let Some(action) = self.pending_action.take() {
                        self.perform_action(action, ctx);
                    }
                }
                Some(2) => {
                    self.pending_action = None;
                    self.file_error = None;
                    self.force_selection = true;
                }
                _ => {}
            }
        } else if self.file_error.is_some() {
            let mut open = true;
            let mut close = false;
            egui::Window::new(crate::tr!("file-error-title"))
                .open(&mut open)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(self.file_error.as_deref().unwrap_or_default());
                    close = ui.button(crate::tr!("close")).clicked();
                });
            if close || !open {
                self.file_error = None;
                self.force_selection = true;
            }
        }
    }
    pub fn capture_profile(&mut self, ctx: &egui::Context, preferences: bool) {
        self.preferences.appearance = Appearance::Light;
        ctx.set_theme(egui::ThemePreference::Light);
        self.preferences.profile = crate::profile::Profile {
            name: "Camille Martin".into(),
            organisation: "Association des Tilleuls".into(),
            address: "12, rue des Tilleuls\n35000 Rennes".into(),
            email: "camille@example.org".into(),
            phone: "02 00 00 00 00".into(),
            role: "Trésorière".into(),
            signature: Some("signature-demo.svg".into()),
        };
        self.profile_signature = Some(crate::assets::ImageData {
            extension: "svg".into(),
            data: include_bytes!("../tests/fixtures/signature.svg")
                .as_slice()
                .into(),
        });
        self.create_document(DocumentType::Courrier);
        let before = self.snapshot();
        if let crate::document_types::Metadata::Courrier(meta) = &mut self.session.metadata {
            meta.destinataire = "Secrétariat de la mairie".into();
            meta.objet = "Confirmation de notre rendez-vous".into();
            meta.lieu = "Rennes".into();
            meta.date = "2026-10-21".into();
        }
        self.session.document.view.insert_text("Madame, Monsieur,\n\nJe vous confirme notre rendez-vous pour préparer la prochaine rencontre de l’association.\n\nJe vous remercie pour votre disponibilité.\n\nBien cordialement,", 0);
        self.remember(before, false);
        self.preferences_page = PreferencesPage::Profile;
        self.preferences_open = preferences;
        if !preferences {
            self.information_group = InformationGroup::Fields("information");
            self.information_open = true;
        }
    }
    pub fn ui_frame(&mut self, ctx: &egui::Context) {
        if self.capture.is_some() {
            // Le pointeur réel ne doit pas modifier les survols des captures de référence.
            ctx.input_mut(|input| input.pointer = Default::default());
        }
        if ctx.input(|input| input.viewport().close_requested())
            && !self.allow_close
            && self.is_dirty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.new_document_open = false;
            self.preferences_open = false;
            self.palette_open = false;
            self.link_open = false;
            self.export_open = false;
            self.information_open = false;
            self.pending_action = Some(PendingAction::Close);
        }
        self.rendering(ctx);
        self.refresh_spelling(ctx);
        if self.document_open && !self.modal_open() {
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            for file in dropped {
                if let Some(path) = file.path
                    && let Err(error) = self.import_image(&path, None)
                {
                    self.file_error = Some(error);
                    break;
                }
            }
        }
        self.track_history_focus(ctx);
        self.shortcuts(ctx);
        #[cfg(target_os = "macos")]
        if let Some(menus) = &self.native_menus {
            let commands = menus.drain();
            for command in commands {
                self.dispatch_action(command, ctx);
            }
        }
        let modal = self.modal_open();
        let dirty = self.is_dirty();
        let filename = self
            .session
            .file_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                self.recovered_name
                    .as_ref()
                    .map(|name| crate::tr!("recovery-document-name", name = name.as_str()))
                    .unwrap_or_else(|| crate::tr!("untitled-letter"))
            });
        let title = if !self.document_open {
            "typstOgraphe".into()
        } else {
            crate::tr!(
                "window-title",
                name = filename.as_str(),
                marker = if dirty { " *" } else { "" }
            )
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
        #[cfg(target_os = "macos")]
        let inline_menus = self.native_menus.is_none();
        #[cfg(not(target_os = "macos"))]
        let inline_menus = true;
        if inline_menus {
            egui::TopBottomPanel::top("menus").show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                if let Some(command) = menus::egui_bar(ui, &self.keymap, &self.menu_statuses()) {
                    self.dispatch_action(command, ctx);
                }
            });
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if modal {
                ui.disable();
            }
            if self.document_open {
                self.status_ui(ui);
            }
            if let Some(error) = &self.recovery_error {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    crate::tr!("recovery-unavailable"),
                )
                .on_hover_text(error);
            }
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                if self.document_open {
                    self.columns_ui(ui, modal);
                } else {
                    self.welcome_ui(ui);
                }
            });
        self.preferences_ui(ctx);
        self.dialogs(ctx);
        if let Some(command) = self.file_command.take() {
            self.handle_file_command(command, ctx);
        }
        self.file_dialogs(ctx);
        self.recovery_dialog(ctx);
        self.checkpoint_recovery();
        self.refresh_spelling(ctx);
        if self.recovery.is_some() {
            ctx.request_repaint_after(crate::recovery::INTERVAL);
        }
        self.track_history_focus(ctx);
        #[cfg(target_os = "macos")]
        {
            if self.native_menus.as_ref().is_some_and(|menus| {
                menus.document_type() != self.keymap.registry().document_type()
            }) {
                // Retirer les anciens menus avant d’installer ceux du type actif.
                self.native_menus.take();
                match menus::native::NativeMenus::new(ctx.clone(), &self.keymap) {
                    Ok(menus) => self.native_menus = Some(menus),
                    Err(error) => {
                        self.file_error =
                            Some(crate::tr!("native-menu-error", error = error.to_string()))
                    }
                }
            }
            let statuses = self.menu_statuses();
            if let Some(menus) = &mut self.native_menus
                && let Err(error) = menus.update(&self.keymap, &statuses)
            {
                self.file_error = Some(crate::tr!("native-menu-error", error = error.to_string()));
                ctx.request_repaint();
            }
        }
        self.capture(ctx);
    }
    fn capture(&mut self, ctx: &egui::Context) {
        if self.capture.is_none() {
            return;
        }
        if self.capture_spelling {
            if !self.spelling.ready {
                return;
            }
            if self.spelling_target.is_none() {
                self.apply(Command::NextSpelling);
                self.apply(Command::SpellingSuggestions);
                ctx.request_repaint();
                return;
            }
            if self
                .spelling
                .suggestions
                .as_ref()
                .is_none_or(|(_, values)| values.is_none())
            {
                return;
            }
        }
        if (!self.document_open || self.rendered_revision == Some(self.session.revision))
            && !self.screenshot_requested
            && self.started.elapsed() > Duration::from_secs(1)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = true;
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let (Some(image), Some(path)) = (image, self.capture.as_ref()) {
            let result = crate::write_screenshot(path, &image);
            if let Err(error) = result {
                eprintln!(
                    "{}",
                    crate::tr!("cli-capture-error", error = error.to_string())
                );
            }
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
impl eframe::App for App {
    fn persist_egui_memory(&self) -> bool {
        self.capture.is_none()
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.capture.is_some() {
            return;
        }
        self.persist_preferences(storage, Instant::now(), true);
        self.checkpoint_recovery();
    }
    fn auto_save_interval(&self) -> Duration {
        // Récupération pilotée séparément par ui_frame ; préférences sur changement.
        Duration::MAX
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if !self.is_dirty() {
            self.clear_recovery();
        }
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let preferences_were_open = self.preferences_open;
        self.ui_frame(ctx);
        if self.capture.is_none()
            && let Some(storage) = frame.storage_mut()
        {
            let force = (preferences_were_open && !self.preferences_open)
                || ctx.input(|input| input.viewport().close_requested());
            self.persist_preferences(storage, Instant::now(), force);
            if let Some(delay) = self
                .preferences_tracker
                .observe(&self.preferences, Instant::now())
            {
                ctx.request_repaint_after(delay);
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod interaction_tests {
    use super::*;
    use crate::document_types::models::Model;

    #[test]
    fn today_button_keeps_a_fixed_date_through_history_and_reopening() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.create_document(DocumentType::Courrier);
        app.information_open = true;
        app.information_group = InformationGroup::Fields("information");
        let source = app.session.document.source().to_owned();
        recovery_click(&mut app, &ctx, &crate::tr!("date-today"));
        let fixed = app.session.metadata.courrier().date.clone();
        assert_eq!(fixed, crate::dates::today());
        app.apply(Command::Undo);
        assert!(app.session.metadata.courrier().date.is_empty());
        app.apply(Command::Redo);
        assert_eq!(app.session.metadata.courrier().date, fixed);
        assert_eq!(app.session.document.source(), source);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("date-fixe.typsto");
        assert!(app.save_path(path.clone()));
        app.close_document();
        assert!(app.open_path(path));
        assert_eq!(app.session.metadata.courrier().date, fixed);
        assert!(!app.is_dirty());
    }

    #[test]
    fn welcome_creation_and_close_use_the_catalog_without_an_implicit_document() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        let dir = tempfile::tempdir().unwrap();
        app.start_recovery(dir.path().join("recovery"), ctx.clone());
        frame(&mut app, &ctx, vec![]);
        assert!(!app.document_open && !app.is_dirty());
        assert_eq!(app.window_title, "typstOgraphe");
        assert_eq!(app.submitted, 0);
        assert!(app.pages.is_empty() && app.rendered_revision.is_none());
        assert!(app.recovery_entries.is_empty());
        let original = app.stored_document();
        for command in [
            Command::Bold,
            Command::Find,
            Command::ImportModel,
            Command::File(FileCommand::Save),
            Command::File(FileCommand::Export),
            Command::Model(Model::Personal),
        ] {
            app.apply(command);
        }
        assert_eq!(app.stored_document(), original);
        assert!(!app.search_open && !app.export_open && app.file_command.is_none());
        for (command, status) in app.menu_statuses() {
            if !command.available_without_document() {
                assert!(!status.enabled);
            }
        }
        app.apply(Command::Preferences);
        assert!(app.preferences_open);
        app.preferences_open = false;
        recovery_click(&mut app, &ctx, &Command::File(FileCommand::New).label());
        assert!(app.new_document_open && !app.document_open);
        assert_eq!(app.new_document_model, Some(Model::Personal));
        recovery_click(&mut app, &ctx, &crate::tr!("cancel"));
        assert!(!app.new_document_open && !app.document_open);
        app.perform_action(PendingAction::New, &ctx);
        recovery_click(&mut app, &ctx, &crate::tr!("new-document-create"));
        assert!(app.document_open && !app.new_document_open);
        assert!(app.session.document.source().is_empty());
        assert_eq!(app.session.template, Model::Personal.source());
        let path = dir.path().join("courrier.typsto");
        assert!(app.save_path(path.clone()));
        app.handle_file_command(FileCommand::Close, &ctx);
        assert!(!app.document_open && !app.allow_close);
        assert!(app.open_path(path));
        assert!(app.document_open);
        app.session.document.view.insert_text("À conserver", 0);
        app.changed();
        app.handle_file_command(FileCommand::Close, &ctx);
        assert!(matches!(
            app.pending_action,
            Some(PendingAction::CloseDocument)
        ));
        recovery_click(&mut app, &ctx, &crate::tr!("cancel"));
        assert!(app.document_open && app.is_dirty());
        app.handle_file_command(FileCommand::Close, &ctx);
        recovery_click(&mut app, &ctx, &crate::tr!("discard"));
        assert!(!app.document_open && !app.is_dirty());
    }

    #[test]
    fn personal_profile_seeds_new_documents_and_signature_edits_are_independent_and_undoable() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("signature.svg");
        std::fs::write(&original, include_bytes!("../tests/fixtures/signature.svg")).unwrap();
        let (name, image) =
            crate::profile::Profile::import_signature(dir.path(), &original).unwrap();
        app.preferences.profile.name = "Zoé Martin".into();
        app.preferences.profile.organisation = "Les Tilleuls".into();
        app.preferences.profile.signature = Some(name);
        app.profile_signature = Some(image);
        app.create_document(DocumentType::Courrier);
        let initial = app.stored_document();
        assert_eq!(initial.metadata.courrier().expediteur, "Zoé Martin");
        assert_eq!(
            initial.metadata.signature_path(),
            Some("images/signature-1.svg")
        );
        assert!(!app.is_dirty());
        app.preferences.profile.name = "Autre nom".into();
        app.preferences.profile.signature = None;
        app.profile_signature = None;
        assert_eq!(app.stored_document(), initial);
        assert!(!app.is_dirty());
        let replacement = crate::assets::ImageData {
            extension: "png".into(),
            data: include_bytes!("../tests/fixtures/colours.png")
                .as_slice()
                .into(),
        };
        app.set_document_signature(Some(&replacement)).unwrap();
        let changed = app.stored_document();
        assert_ne!(
            initial.metadata.signature_path(),
            changed.metadata.signature_path()
        );
        assert!(app.is_dirty());
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), initial);
        app.apply(Command::Redo);
        assert_eq!(app.stored_document(), changed);
        // Une absence de signature dans le profil ne doit pas effacer celle du courrier.
        assert!(app.use_profile_signature().is_err());
        assert_eq!(app.stored_document(), changed);
        app.preferences.profile.signature = Some("signature-profile.svg".into());
        app.profile_signature = Some(crate::assets::ImageData {
            extension: "svg".into(),
            data: initial.assets[initial.metadata.signature_path().unwrap()].clone(),
        });
        let preferences = app.preferences.clone();
        app.use_profile_signature().unwrap();
        let restored = app.stored_document();
        assert_eq!(
            restored.assets[restored.metadata.signature_path().unwrap()],
            initial.assets[initial.metadata.signature_path().unwrap()]
        );
        assert_eq!(app.preferences, preferences);
        let revision = app.session.revision;
        let assets = app.session.document.assets.len();
        app.use_profile_signature().unwrap();
        assert_eq!(app.session.revision, revision);
        assert_eq!(app.session.document.assets.len(), assets);
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), changed);
        app.apply(Command::Redo);
        assert_eq!(app.stored_document(), restored);
        app.set_document_signature(None).unwrap();
        app.use_profile_signature().unwrap();
        assert!(app.session.metadata.signature_path().is_some());
        // Revenir à la signature remplacée pour poursuivre le test d'archivage.
        app.apply(Command::Undo);
        app.apply(Command::Undo);
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), changed);
        let path = dir.path().join("document.typsto");
        assert!(app.save_path(path.clone()));
        let saved = storage::read(&path).unwrap();
        assert_eq!(saved.assets.len(), 1);
        assert_eq!(
            saved.metadata.signature_path(),
            changed.metadata.signature_path()
        );
        app.set_document_signature(None).unwrap();
        assert!(app.save_path(path.clone()));
        assert!(storage::read(&path).unwrap().assets.is_empty());
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), changed);
        assert!(app.save_path(path.clone()));
        app.create_document(DocumentType::CompteRendu);
        assert!(app.file_error.is_some());
        assert_eq!(app.stored_document(), changed);
        assert!(app.open_path(path));
        assert_eq!(app.session.metadata.courrier().expediteur, "Zoé Martin");
        assert_eq!(app.preferences.profile.name, "Autre nom");
    }

    #[test]
    fn global_profile_saves_after_changes_or_closing_without_modifying_the_document() {
        #[derive(Default)]
        struct Store {
            data: std::collections::HashMap<String, String>,
            writes: usize,
        }
        impl eframe::Storage for Store {
            fn get_string(&self, key: &str) -> Option<String> {
                self.data.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.data.insert(key.into(), value);
                self.writes += 1;
            }
            fn flush(&mut self) {}
        }
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let before = app.stored_document();
        let mut store = Store::default();
        let now = Instant::now();
        app.persist_preferences(&mut store, now, false);
        app.preferences.profile.name = "Zoé".into();
        app.persist_preferences(&mut store, now, false);
        assert_eq!(store.writes, 0);
        app.persist_preferences(
            &mut store,
            now + crate::preferences::SaveTracker::DELAY,
            false,
        );
        assert_eq!(store.writes, 1);
        assert_eq!(Preferences::load(Some(&store)).profile.name, "Zoé");
        app.persist_preferences(&mut store, now + Duration::from_secs(100), false);
        assert_eq!(store.writes, 1);
        app.preferences.profile.email = "zoe@example.org".into();
        eframe::App::save(&mut app, &mut store); // Fermeture avant le délai de saisie.
        assert_eq!(
            Preferences::load(Some(&store)).profile.email,
            "zoe@example.org"
        );
        assert_eq!(store.writes, 2);
        assert_eq!(eframe::App::auto_save_interval(&app), Duration::MAX);
        app.preferences_open = true;
        app.preferences_page = PreferencesPage::Profile;
        frame(&mut app, &ctx, vec![]);
        app.preferences_page = PreferencesPage::Appearance;
        frame(&mut app, &ctx, vec![]);
        app.preferences_page = PreferencesPage::Spelling;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.stored_document(), before);
        assert!(!app.is_dirty());
        app.capture = Some("ignored.png".into());
        assert!(!eframe::App::persist_egui_memory(&app));
        app.preferences.profile.name = "Capture isolée".into();
        eframe::App::save(&mut app, &mut store);
        assert_eq!(store.writes, 2);
    }

    #[test]
    fn spelling_context_menu_corrects_rich_text_with_undo_and_never_applies_a_stale_suggestion() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::ThemePreference::Dark);
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut letter = StoredDocument::blank(DocumentType::Courrier);
        letter.source = "Un *courrrier* et une _réservvation_.".into();
        let document = Document::from_source(&letter.source).unwrap();
        app.install_document(letter, document, None).unwrap();
        let original = app.stored_document();
        let _ = search_draw(&mut app, &ctx, vec![]);
        app.spelling.wait();
        assert_eq!(app.spelling.issues.len(), 2);
        let issue = app.spelling.issues[0].clone();
        let revision = app.session.revision;
        let output = search_draw(&mut app, &ctx, vec![]);
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == app.session.document.view.text => {
                    editor::range_rects(&text.galley, std::slice::from_ref(&issue.range))
                        .first()
                        .map(|(_, rect)| text.pos + rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap();
        for pressed in [true, false] {
            let _ = search_draw(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(app.spelling_target, Some((revision, issue.clone())));
        assert_eq!(app.range(), issue.range);
        app.spelling.wait();
        recovery_click(&mut app, &ctx, "courrier");
        assert_eq!(
            app.session.document.view.text,
            "Un courrier et une réservvation."
        );
        assert!(
            app.session.document.view.styles[3..11]
                .iter()
                .all(|style| style.bold)
        );
        assert_eq!(app.session.undo.len(), 1);
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), original);
        // Même mot et mêmes positions après annulation : le ticket ancien reste invalide.
        app.correct_spelling(revision, &issue, "courrier");
        assert_eq!(app.stored_document(), original);
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        app.selection = editor::cursor(0..0);
        app.apply(Command::NextSpelling);
        app.apply(Command::SpellingSuggestions);
        app.spelling.wait();
        let _ = search_draw(&mut app, &ctx, vec![]);
        let output = search_draw(&mut app, &ctx, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "courrier")), "les suggestions sont accessibles sans souris");
        let _ = search_draw(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        app.apply(Command::Redo);
        assert_eq!(
            app.session.document.view.text,
            "Un courrier et une réservvation."
        );
    }

    #[test]
    fn spelling_exceptions_are_scoped_persisted_and_undoable_without_affecting_typst() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut letter = StoredDocument::blank(DocumentType::Courrier);
        letter.source = "Courrrier courrrier réservvation".into();
        let document = Document::from_source(&letter.source).unwrap();
        app.install_document(letter, document, None).unwrap();
        let original_main = app.session.metadata.main_source();
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        assert_eq!(app.spelling.issues.len(), 3);
        app.apply(Command::NextSpelling);
        let first = app.range();
        app.apply(Command::NextSpelling);
        assert_ne!(app.range(), first);
        app.apply(Command::PreviousSpelling);
        assert_eq!(app.range(), first);
        app.apply(Command::IgnoreSpelling);
        assert!(app.is_dirty());
        assert_eq!(app.session.metadata.main_source(), original_main);
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        assert_eq!(app.spelling.issues.len(), 1);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("orthographe.typsto");
        assert!(app.save_path(path.clone()));
        let stored = storage::read(&path).unwrap();
        assert_eq!(
            stored.metadata.options().spelling_ignored["fr"],
            std::collections::BTreeSet::from(["courrrier".into()])
        );
        app.apply(Command::Undo);
        assert!(app.session.metadata.options().spelling_ignored.is_empty());
        app.apply(Command::Redo);
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        app.apply(Command::NextSpelling);
        app.apply(Command::LearnSpelling);
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        assert!(app.spelling.issues.is_empty());
        assert!(
            !app.is_dirty(),
            "apprendre un mot ne modifie pas le document"
        );
        let encoded = serde_json::to_string(&app.preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&encoded).unwrap();
        assert!(restored.spelling.personal["fr"].contains("réservvation"));
        app.apply(Command::ResetSpelling);
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        assert_eq!(app.spelling.issues.len(), 2);
        let before = app.snapshot();
        app.session.metadata.options_mut().language = DocumentLanguage::German;
        app.remember(before, false);
        app.refresh_spelling(&ctx);
        assert!(app.spelling.issues.is_empty() && !app.spelling.ready);
        app.apply(Command::ToggleSpelling);
        assert!(!app.preferences.spelling.enabled);
        app.apply(Command::ToggleSpelling);
        let mut another = StoredDocument::blank(DocumentType::CompteRendu);
        another.source = stored.source.clone();
        let document = Document::from_source(&another.source).unwrap();
        app.install_document(another, document, None).unwrap();
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        assert_eq!(
            app.spelling.issues.len(),
            2,
            "les mots ignorés ne fuient pas vers un autre document ; le dictionnaire personnel reste partagé"
        );
        let document = Document::from_source(&stored.source).unwrap();
        app.install_document(stored, document, Some(path)).unwrap();
        app.refresh_spelling(&ctx);
        app.spelling.wait();
        assert!(
            app.spelling.issues.is_empty(),
            "la réouverture réapplique les deux niveaux d’exceptions"
        );
    }

    fn search_draw(
        app: &mut App,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1240.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.ui_frame(ctx),
        )
    }
    #[test]
    fn incremental_search_keeps_one_selection_and_clicking_another_result_updates_preview_highlights()
     {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut letter = StoredDocument::blank(DocumentType::Courrier);
        letter.source = "Un *été*, puis un ÉTÉ et encore un _été_.".into();
        let document = Document::from_source(&letter.source).unwrap();
        app.install_document(letter, document, None).unwrap();
        let before = app.stored_document();
        let mut world = crate::rendering::TypstWorld::default();
        let compiled = world
            .compile_with_template(
                app.session.document.source(),
                &app.session.metadata,
                &app.session.template,
            )
            .unwrap();
        app.sync = Some(RenderSync::new(
            app.session.revision,
            world,
            compiled,
            app.session.document.source_map.clone(),
        ));
        app.rendered_revision = Some(app.session.revision);
        let _ = search_draw(&mut app, &ctx, vec![]);
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let _ = search_draw(
            &mut app,
            &ctx,
            vec![key(egui::Key::F, egui::Modifiers::COMMAND)],
        );
        assert!(app.search_open && app.search_focus(&ctx).is_some());
        let caret = app.selection;
        let _ = search_draw(&mut app, &ctx, vec![egui::Event::Text("été".into())]);
        assert_eq!(app.search.matches.len(), 3);
        assert_eq!(
            app.selection, caret,
            "la saisie de recherche ne déplace pas le curseur du document"
        );
        assert_eq!(app.stored_document(), before);
        assert!(!app.is_dirty());
        let _ = search_draw(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
        );
        assert_eq!(app.range(), app.search.matches[0]);
        assert!(
            app.search_focus(&ctx).is_some(),
            "Entrée doit permettre de continuer la saisie de recherche"
        );
        let mut previous = None;
        for index in [1, 2] {
            let output = search_draw(&mut app, &ctx, vec![]);
            let point = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text.galley.job.text == app.session.document.view.text =>
                    {
                        editor::range_rects(&text.galley, &app.search.matches)
                            .into_iter()
                            .find(|(found, _)| *found == index)
                            .map(|(_, rect)| text.pos + rect.center().to_vec2())
                    }
                    _ => None,
                })
                .expect("résultat visible dans l’éditeur");
            for pressed in [true, false] {
                let _ = search_draw(
                    &mut app,
                    &ctx,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            let _ = search_draw(&mut app, &ctx, vec![]);
            assert_eq!(app.range(), app.search.matches[index]);
            let native = egui::TextEdit::load_state(&ctx, egui::Id::new(editor::EDITOR_ID))
                .unwrap()
                .cursor
                .char_range()
                .unwrap();
            assert_eq!(native.as_sorted_char_range(), app.search.matches[index]);
            assert_eq!(
                app.search.matches.len(),
                3,
                "l’ancienne occurrence reste un résultat surligné"
            );
            let highlights = app.sync.as_ref().unwrap().selection_highlights(app.range());
            assert!(!highlights.is_empty());
            let first = highlights[0].corners;
            assert!(previous.is_none_or(|previous| previous != first));
            previous = Some(first);
        }
        let _ = search_draw(
            &mut app,
            &ctx,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(!app.search_open);
        assert_eq!(app.range(), app.search.matches[2]);
    }

    #[test]
    fn replacement_commands_refresh_results_and_replace_all_is_a_single_document_transaction() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut stored = StoredDocument::blank(DocumentType::Courrier);
        stored.source = "Un *été* et un _ÉTÉ_, été.".into();
        app.install_document(
            stored.clone(),
            Document::from_source(&stored.source).unwrap(),
            None,
        )
        .unwrap();
        app.apply(Command::FindReplace);
        app.search.query = "été".into();
        app.search.replacement = "Noël".into();
        app.apply(Command::FindNext);
        assert_eq!(app.search.matches.len(), 3);
        app.apply(Command::ReplaceCurrent);
        assert_eq!(app.search.matches.len(), 2);
        assert_eq!(app.range(), app.search.matches[0]);
        assert!(app.session.document.view.text.contains("Noël"));
        app.apply(Command::Undo);
        app.refresh_search();
        assert_eq!(app.stored_document(), stored);
        app.search.replacement = "été été".into();
        app.apply(Command::ReplaceAll);
        assert_eq!(
            app.search.matches.len(),
            6,
            "les nouveaux résultats ne sont pas remplacés en boucle"
        );
        assert_eq!(app.session.undo.len(), 1);
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), stored);
        app.apply(Command::Redo);
        app.refresh_search();
        assert_eq!(app.search.matches.len(), 6);
        app.search.replacement.clear();
        app.apply(Command::ReplaceAll);
        assert!(app.search.matches.is_empty());
        app.apply(Command::Undo);
        app.refresh_search();
        assert_eq!(app.search.matches.len(), 6);
    }

    fn recovery_click(app: &mut App, ctx: &egui::Context, label: &str) {
        let draw = |app: &mut App, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1240.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        let _ = draw(app, vec![]);
        let output = draw(app, vec![]);
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("bouton absent : {label}"));
        for pressed in [true, false] {
            let _ = draw(
                app,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    fn seed_recovery(root: &std::path::Path, letter: &StoredDocument, name: Option<String>) {
        let (mut recovery, _) = crate::recovery::Recovery::start(root, Default::default()).unwrap();
        recovery.make_due();
        recovery
            .checkpoint(1, true, || (letter.clone(), name))
            .unwrap();
        recovery.wait();
        assert!(recovery.poll().into_iter().all(|result| result.is_ok()));
    }

    #[test]
    fn recovery_dialog_defers_restores_and_saves_a_copy_without_overwriting_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let original = dir.path().join("original.typsto");
        let saved = StoredDocument::demo(DocumentType::Courrier);
        storage::save(&original, &saved).unwrap();
        let mut unsaved = saved.clone();
        unsaved.source = "Un travail *non enregistré*.".into();
        unsaved.metadata.courrier_mut().objet = "Nouvel objet".into();
        unsaved.template = Model::Personal.source();
        seed_recovery(&root, &unsaved, Some("original.typsto".into()));
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.start_recovery(root.clone(), ctx.clone());
        assert!(app.recovery_open);
        recovery_click(&mut app, &ctx, &crate::tr!("recovery-later"));
        assert!(!app.recovery_open);
        assert_eq!(app.recovery_entries.len(), 1);
        app.apply(Command::RecoverDocuments);
        recovery_click(&mut app, &ctx, &crate::tr!("recovery-restore"));
        app.recovery.as_mut().unwrap().wait();
        assert!(!app.recovery_open);
        assert!(app.recovery_entries.is_empty());
        assert!(app.is_dirty() && app.session.recovered);
        assert!(app.session.file_path.is_none());
        assert!(app.session.undo.is_empty() && app.session.redo.is_empty());
        assert_eq!(app.stored_document(), unsaved);
        assert!(
            app.recovered_filename()
                .unwrap()
                .contains("original-recupere")
        );
        app.request_action(PendingAction::Close, &ctx);
        assert!(app.pending_action.is_some());
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.pending_action.is_none() && app.is_dirty());
        assert!(!app.save_path(dir.path().join("absent/fichier.typsto")));
        drop(app); // L’échec d’enregistrement laisse la récupération intacte.
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.start_recovery(root.clone(), ctx.clone());
        assert_eq!(app.recovery_entries.len(), 1);
        app.recover_document(0);
        assert_eq!(app.stored_document(), unsaved);
        let copy = dir.path().join("recupere.typsto");
        assert!(app.save_path(copy.clone()));
        app.recovery.as_mut().unwrap().wait();
        assert!(!app.is_dirty() && !app.session.recovered);
        assert_eq!(storage::read(&copy).unwrap(), unsaved);
        assert_eq!(storage::read(&original).unwrap(), saved);
        drop(app);
        assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    }

    #[test]
    fn explicit_delete_and_discard_remove_recovery_but_cancelling_keeps_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let saved = StoredDocument::demo(DocumentType::Courrier);
        seed_recovery(&root, &saved, None);
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.start_recovery(root.clone(), ctx.clone());
        recovery_click(&mut app, &ctx, &crate::tr!("recovery-delete"));
        assert!(app.recovery_entries.is_empty());
        assert!(!app.recovery_open);
        app.session.document.view.insert_text("À conserver. ", 0);
        app.changed();
        app.recovery.as_mut().unwrap().make_due();
        app.checkpoint_recovery();
        app.recovery.as_mut().unwrap().wait();
        app.request_action(PendingAction::Close, &ctx);
        recovery_click(&mut app, &ctx, &crate::tr!("cancel"));
        assert!(app.pending_action.is_none() && app.is_dirty());
        app.request_action(PendingAction::Close, &ctx);
        recovery_click(&mut app, &ctx, &crate::tr!("discard"));
        assert!(app.allow_close);
        drop(app);
        let (recovery, entries) = crate::recovery::Recovery::start(&root, ctx).unwrap();
        assert!(entries.is_empty());
        drop(recovery);
    }

    #[test]
    fn creation_and_template_import_preserve_the_type_and_use_unified_history() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.perform_action(PendingAction::New, &ctx);
        frame(&mut app, &ctx, vec![]);
        assert!(app.new_document_open && !app.document_open);
        app.create_document(DocumentType::CompteRendu);
        assert!(app.file_error.is_some() && !app.document_open);
        app.file_error = None;
        app.create_document(DocumentType::Courrier);
        assert!(app.document_open && !app.new_document_open);
        assert_eq!(
            app.keymap.registry().document_type(),
            DocumentType::Courrier
        );
        assert!(app.session.document.source().is_empty());
        assert!(!app.is_dirty());
        assert_eq!(app.session.template, Model::Personal.source());
        let before = app.stored_document();
        assert!(
            app.install_model("#let compte-rendu(..args) = []".into())
                .is_err()
        );
        assert_eq!(app.stored_document(), before);
        let template = format!(
            "{}\n// Modèle fourni par l’organisation",
            app.session.template
        );
        app.install_model(template.clone()).unwrap();
        assert_eq!(app.session.template, template);
        assert!(app.is_dirty());
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), before);
        assert!(!app.is_dirty());
        app.apply(Command::Redo);
        assert_eq!(app.session.template, template);
        assert_eq!(app.session.metadata.document_type(), DocumentType::Courrier);
    }

    #[test]
    fn links_accept_soft_lines_and_reject_paragraph_or_list_boundaries() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx, None);
        for (source, selection, allowed) in [
            ("Été\nNoël", 0..8, true),
            ("Été\n\nNoël", 0..9, false),
            ("- Un\n- Deux", 0..7, false),
        ] {
            app.session.document = Document::from_source(source).unwrap();
            app.selection = editor::cursor(selection);
            app.link_open = false;
            app.apply(Command::Link);
            assert_eq!(app.link_open, allowed, "{source}");
        }
    }

    #[test]
    fn native_enter_distinguishes_soft_newlines_and_paragraphs_with_undo_and_reopening() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.document = Document::from_source("Alpha").unwrap();
        app.selection = editor::cursor(5..5);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(app.session.document.source(), "Alpha\n");
        frame(&mut app, &ctx, vec![egui::Event::Text("Beta".into())]);
        assert_eq!(app.session.document.source(), "Alpha\nBeta");
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(app.session.document.view.text, "Alpha\nBeta\n\n");
        frame(&mut app, &ctx, vec![egui::Event::Text("Gamma".into())]);
        let paragraphs = app.session.document.clone();
        app.selection = editor::cursor(12..12);
        app.force_selection = true;
        app.session.typing_group = false;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Backspace)]);
        assert_eq!(app.session.document.source(), "Alpha\nBeta\nGamma");
        app.apply(Command::Undo);
        assert_eq!(app.session.document, paragraphs);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paragraphes.typsto");
        assert!(app.save_path(path.clone()));
        assert!(app.open_path(path));
        assert_eq!(app.session.document.view, paragraphs.view);
        assert_eq!(app.session.document.source(), paragraphs.source());
        assert!(!app.session.document.source().contains('¶'));
    }

    #[test]
    fn compact_toolbar_and_status_show_unicode_totals_selection_and_tooltips() {
        fn text_rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    let bounds = text
                        .galley
                        .rows
                        .iter()
                        .fold(egui::Rect::NOTHING, |bounds, row| {
                            bounds.union(row.rect_without_leading_space())
                        });
                    Some(bounds.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
        }
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);
        app.session.document = Document::from_source("#strong[e\u{301}] 👩🏽‍💻").unwrap();
        app.force_selection = true;
        let mut time = 0.0;
        let mut draw = |app: &mut App, events| {
            time += 0.2;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(960.0, 640.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        draw(&mut app, vec![]);
        let output = draw(&mut app, vec![]);
        let brand = text_rect(&output, "typstOgraphe").unwrap();
        let heading = icon_rect(&output, &ctx, crate::icons::Icon::Heading);
        assert!(brand.right() < heading.left());
        assert!(
            (brand.center().y - heading.center().y).abs() < 2.0,
            "nom {brand:?}, titres {heading:?}"
        );
        assert_eq!(output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "typstOgraphe")).count(), 1);
        let total = crate::tr!("document-stats", words = 1, signs = 3);
        assert!(text_rect(&output, &total).is_some());
        assert!(text_rect(&output, "Point d’insertion").is_none());
        app.selection = editor::cursor(0..2); // e + accent combiné : un signe.
        app.force_selection = true;
        draw(&mut app, vec![]);
        let output = draw(&mut app, vec![]);
        let selection = crate::tr!("selection-stats", words = 1, signs = 1);
        assert!(text_rect(&output, &total).is_some());
        assert!(text_rect(&output, &selection).is_some());
        for (label, without_spaces) in [(&total, 2), (&selection, 1)] {
            let output = draw(&mut app, vec![]);
            let pos = text_rect(&output, label).unwrap().center();
            draw(&mut app, vec![egui::Event::PointerMoved(pos)]);
            let mut output = draw(&mut app, vec![]);
            for _ in 0..4 {
                output = draw(&mut app, vec![]);
            }
            assert!(
                text_rect(
                    &output,
                    &crate::tr!("statistics-hint", without_spaces = without_spaces)
                )
                .is_some(),
                "détail hors espaces de {label}"
            );
        }
        draw(&mut app, vec![egui::Event::Text("Deux mots".into())]);
        let output = draw(&mut app, vec![]);
        let edited = crate::tr!("document-stats", words = 2, signs = 11);
        assert!(text_rect(&output, &edited).is_some());
        assert!(text_rect(&output, &selection).is_none());
        app.apply(Command::Undo);
        let output = draw(&mut app, vec![]);
        assert!(text_rect(&output, &total).is_some());
        assert!(text_rect(&output, &selection).is_some());
        app.apply(Command::Redo);
        let output = draw(&mut app, vec![]);
        assert!(text_rect(&output, &edited).is_some());
        app.create_document(DocumentType::Courrier);
        let output = draw(&mut app, vec![]);
        assert!(text_rect(&output, &crate::tr!("document-stats", words = 0, signs = 0)).is_some());
    }

    fn icon_rect(
        output: &egui::FullOutput,
        ctx: &egui::Context,
        icon: crate::icons::Icon,
    ) -> egui::Rect {
        fn find(shape: &egui::Shape, texture: egui::TextureId) -> Option<egui::Rect> {
            match shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh.calc_bounds()),
                egui::Shape::Rect(rect)
                    if rect
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == texture) =>
                {
                    Some(rect.rect)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, texture)),
                _ => None,
            }
        }
        let texture = icon.texture(ctx, crate::toolbar::ICON_SIZE).id();
        output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, texture))
            .unwrap_or_else(|| panic!("icône absente : {icon:?}"))
    }

    #[test]
    fn tab_places_the_native_caret_on_each_empty_annotation_placeholder() {
        for source in [
            "#figure(image(\"images/parc.svg\"))\n\nSuite.",
            "#quote(block: true)[Une pensée.]\n\nSuite.",
        ] {
            let ctx = egui::Context::default();
            let mut app = App::with_demo_context(ctx.clone(), None);
            // Vérifier la géométrie réellement peinte, indépendamment du clignotement.
            ctx.all_styles_mut(|style| style.visuals.text_cursor.blink = false);
            app.session.document = Document::from_source(source).unwrap();
            app.session.document.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../tests/fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
            let annotation = app.session.document.view.annotations().remove(0);
            let at = annotation.content.start;
            let placeholder = annotation.kind.placeholder();
            app.selection = editor::cursor(annotation.main_target());
            app.force_selection = true;
            let mut time = 0.0;
            let mut draw = |app: &mut App, events| {
                time += 0.1;
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1240.0, 900.0),
                        )),
                        time: Some(time),
                        events,
                        focused: true,
                        ..Default::default()
                    },
                    |ctx| app.ui_frame(ctx),
                )
            };
            draw(&mut app, vec![]);
            let tab = || egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            for _ in 0..2 {
                let output = draw(&mut app, vec![tab()]);
                assert_eq!(app.range(), at..at);
                assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
                fn annotation_hint(shape: &egui::Shape, placeholder: &str) -> Option<egui::Pos2> {
                    match shape {
                        egui::Shape::Text(text) if text.galley.text() == placeholder => {
                            Some(text.pos)
                        }
                        egui::Shape::Vec(shapes) => shapes
                            .iter()
                            .find_map(|shape| annotation_hint(shape, placeholder)),
                        _ => None,
                    }
                }
                let hint = output
                    .shapes
                    .iter()
                    .find_map(|shape| annotation_hint(&shape.shape, &placeholder))
                    .expect("texte indicatif de l’annotation vide");
                let caret = output
                    .platform_output
                    .ime
                    .expect("curseur natif du TextEdit")
                    .cursor_rect;
                assert!(
                    (caret.top() - hint.y).abs() < 2.0,
                    "curseur {caret:?}, annotation {hint:?}"
                );
                assert!((caret.left() - hint.x).abs() < 2.0);
                assert!(caret.height() < app.preferences.editor.size * 2.0);
                assert!(output.shapes.iter().any(|shape| matches!(shape.shape, egui::Shape::LineSegment { points, .. } if (points[0] - caret.center_top()).length() < 1.0 && (points[1] - caret.center_bottom()).length() < 1.0)), "le curseur doit être peint dans l’annotation");
                assert_eq!(
                    app.session.document.source(),
                    source,
                    "le texte indicatif reste virtuel"
                );
            }
            draw(&mut app, vec![egui::Event::Text("Une annotation.".into())]);
            assert!(app.session.document.source().contains("[Une annotation.]"));
            draw(&mut app, vec![tab()]);
            assert_eq!(app.range(), at..at);
            draw(&mut app, vec![egui::Event::Text("Été : ".into())]);
            assert!(
                app.session
                    .document
                    .view
                    .text
                    .contains("Été : Une annotation.")
            );
        }
    }

    #[test]
    fn annotation_navigation_and_exit_preserve_content_selection_and_undo() {
        for source in [
            "#figure(image(\"images/parc.svg\"), caption: [Été *à Paris*.])\n\nSuite.",
            "#quote(block: true, attribution: [Été *à Paris*.])[Une pensée.]\n\nSuite.",
        ] {
            let ctx = egui::Context::default();
            let mut app = App::with_demo_context(ctx.clone(), None);
            app.session.document = Document::from_source(source).unwrap();
            let annotation = app.session.document.view.annotations().remove(0);
            let at = annotation.content.start;
            app.selection = editor::cursor(annotation.content.clone());
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            let key = |key, modifiers| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            };
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Tab, egui::Modifiers::SHIFT)],
            );
            assert_eq!(app.range(), annotation.main_target());
            assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
            );
            assert_eq!(app.range(), at..at);
            assert!(
                app.session.undo.is_empty(),
                "la navigation ne modifie pas le document"
            );
            let original = app.session.document.clone();
            // Entrée quitte toute la structure sans remplacer l’annotation sélectionnée.
            app.selection = editor::cursor(annotation.content.clone());
            let selection = app.selection;
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
            );
            assert_eq!(
                app.range(),
                annotation.structure.end + 1..annotation.structure.end + 1
            );
            assert_eq!(
                app.session.document.view.blocks
                    [app.session.document.view.line_at(app.range().start)],
                BlockKind::Paragraph
            );
            assert_eq!(app.session.document.view.annotations()[0], annotation);
            assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
            frame(&mut app, &ctx, vec![egui::Event::Text("Après.".into())]);
            assert!(
                app.session
                    .document
                    .view
                    .text
                    .contains("Été à Paris.\nAprès.")
            );
            app.apply(Command::Undo);
            app.apply(Command::Undo);
            assert_eq!(app.session.document, original);
            assert_eq!(app.selection, selection);
        }
    }

    #[test]
    fn annotation_navigation_scrolls_to_the_target_in_both_structures() {
        for source in [
            format!(
                "{}#figure(image(\"images/parc.svg\"))",
                "Avant.\n\n".repeat(80)
            ),
            format!("#quote(block: true)[{}]", "Une pensée.\n\n".repeat(80)),
        ] {
            let ctx = egui::Context::default();
            let mut app = App::with_demo_context(ctx.clone(), None);
            app.session.document = Document::from_source(&source).unwrap();
            let annotation = app.session.document.view.annotations().remove(0);
            app.selection = editor::cursor(annotation.main_target());
            app.force_selection = true;
            let mut time = 0.0;
            let mut draw = |app: &mut App, events| {
                time += 0.1;
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1240.0, 900.0),
                        )),
                        time: Some(time),
                        events,
                        focused: true,
                        ..Default::default()
                    },
                    |ctx| app.ui_frame(ctx),
                )
            };
            draw(&mut app, vec![]);
            let key = |modifiers| egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            };
            for modifiers in [egui::Modifiers::NONE, egui::Modifiers::SHIFT] {
                draw(&mut app, vec![key(modifiers)]);
                // Laisser le défilement animé d’egui atteindre la cible.
                let mut output = draw(&mut app, vec![]);
                for _ in 0..8 {
                    output = draw(&mut app, vec![]);
                }
                let ime = output.platform_output.ime.expect("focus natif conservé");
                assert!(
                    ime.cursor_rect.top() >= 100.0 && ime.cursor_rect.bottom() <= 900.0,
                    "annotation {:?}, cible {:?}, curseur {:?}",
                    annotation.kind,
                    app.range(),
                    ime.cursor_rect
                );
                assert_eq!(
                    app.range(),
                    if modifiers.shift {
                        annotation.main_target()
                    } else {
                        annotation.content.clone()
                    }
                );
            }
            assert_eq!(app.session.document.source(), source);
        }
    }

    #[test]
    fn discarded_images_in_memory_do_not_exhaust_the_document_size_limit() {
        let mut app = App::with_demo_context(egui::Context::default(), None);
        app.session.document = Document::from_source("").unwrap();
        // Simuler plus de 64 Mio d’anciennes ressources partagées en mémoire.
        // Elles ne sont plus référencées par le document ni par son modèle.
        let unused: std::sync::Arc<[u8]> = vec![0; 1024 * 1024].into();
        for index in 0..65 {
            app.session
                .document
                .assets
                .insert(format!("images/old-{index}.png"), unused.clone());
        }
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("parc.svg");
        std::fs::write(&image, include_bytes!("../tests/fixtures/parc.svg")).unwrap();
        app.import_image(&image, None).unwrap();
        let path = dir.path().join("courrier.typsto");
        assert!(app.save_path(path.clone()), "{:?}", app.file_error);
        assert_eq!(storage::read(&path).unwrap().assets.len(), 1);
        assert!(!app.is_dirty());
    }

    #[test]
    fn saving_prunes_images_on_disk_without_changing_undo_redo_or_clean_state() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx, None);
        app.session.document = Document::from_source("").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.svg");
        std::fs::write(&original, include_bytes!("../tests/fixtures/parc.svg")).unwrap();
        app.import_image(&original, None).unwrap();
        let first = app.session.document.view.images[&1].path.clone();
        let path = dir.path().join("Courrier.typsto");
        assert!(app.save_path(path.clone()));
        app.import_image(&original, Some(1)).unwrap();
        let replacement = app.session.document.view.images[&1].path.clone();
        std::fs::remove_file(&original).unwrap();
        assert!(app.save_path(path.clone()));
        assert!(!app.is_dirty());
        assert_eq!(
            storage::read(&path)
                .unwrap()
                .assets
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![replacement.clone()]
        );
        assert!(app.session.document.assets.contains_key(&first));
        app.apply(Command::Undo);
        assert_eq!(app.session.document.view.images[&1].path, first);
        assert!(app.is_dirty());
        assert!(app.save_path(path.clone()));
        assert!(!app.is_dirty());
        assert_eq!(
            storage::read(&path)
                .unwrap()
                .assets
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![first]
        );
        app.apply(Command::Redo);
        assert_eq!(app.session.document.view.images[&1].path, replacement);
        app.remove_image(1);
        assert!(app.save_path(path.clone()));
        assert!(!app.is_dirty());
        assert!(storage::read(&path).unwrap().assets.is_empty());
        let export = dir.path().join("export");
        crate::rendering::render(
            &mut crate::rendering::TypstWorld::default(),
            &app.request(Some(export.clone())),
        )
        .unwrap();
        assert!(!export.join("images").exists());
        app.apply(Command::Undo);
        assert_eq!(app.session.document.view.images[&1].path, replacement);
        assert!(app.is_dirty());
        assert!(app.session.document.assets.contains_key(&replacement));
        assert!(app.save_path(path.clone()));
        assert_eq!(storage::read(&path).unwrap().assets.len(), 1);
        app.apply(Command::Redo);
        assert!(app.session.document.view.images.is_empty());
        assert!(app.save_path(path.clone()));
        assert!(storage::read(&path).unwrap().assets.is_empty());
        assert!(!app.is_dirty());
    }

    #[test]
    fn image_remove_button_deletes_the_selected_block_and_caption_with_undo() {
        for caption in ["", ", caption: [Le parc.]"] {
            let ctx = egui::Context::default();
            let mut app = App::with_demo_context(ctx.clone(), None);
            app.session.document = Document::from_source(&format!(
                "Avant.\n\n#figure(image(\"images/parc.svg\"){caption})\n\nAprès."
            ))
            .unwrap();
            app.session.document.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../tests/fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
            let original = app.session.document.clone();
            let at = app.session.document.view.image_range(1).unwrap().start;
            // Boutons accessibles depuis l’objet sélectionné comme depuis la légende.
            let selection = if caption.is_empty() {
                at..at + 1
            } else {
                at + 1..at + 1
            };
            app.selection = editor::cursor(selection.clone());
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == crate::tr!("image-remove")
                    {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    } else {
                        None
                    }
                })
                .expect("bouton Supprimer de l’image sélectionnée");
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
                if pressed {
                    assert_eq!(
                        app.range(),
                        selection,
                        "l’appui sur le bouton ne déplace pas le curseur dans le texte sous-jacent"
                    );
                }
            }
            assert!(
                app.session.document.view.images.is_empty(),
                "le clic doit supprimer l’image entière"
            );
            assert!(!app.session.document.view.text.contains("Le parc."));
            assert!(app.session.document.view.text.contains("Avant."));
            assert!(app.session.document.view.text.contains("Après."));
            app.apply(Command::Undo);
            assert_eq!(app.session.document, original);
        }
    }

    #[test]
    fn image_controls_edit_alt_and_keyboard_deletes_the_whole_block_with_undo() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.document = Document::from_source("Avant.\n\nAprès.").unwrap();
        app.selection = editor::cursor(6..6);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("parc.svg");
        std::fs::write(&path, include_bytes!("../tests/fixtures/parc.svg")).unwrap();
        app.import_image(&path, None).unwrap();
        frame(&mut app, &ctx, vec![]);
        let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
        let text_point = |output: &egui::FullOutput, label: &str| {
            output
                .shapes
                .iter()
                .rev()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == label
                    {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| panic!("Texte absent : {label}"))
        };
        let click = |app: &mut App, pos| {
            frame(
                app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            frame(
                app,
                &ctx,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        };
        click(
            &mut app,
            text_point(&output, &crate::tr!("image-information")),
        );
        assert_eq!(app.image_info, Some(1));
        app.image_alt = "Des arbres et un chemin".into();
        let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
        click(&mut app, text_point(&output, &crate::tr!("image-apply")));
        assert!(app.image_info.is_none());
        assert!(
            app.session
                .document
                .source()
                .contains("Des arbres et un chemin")
        );
        frame(&mut app, &ctx, vec![egui::Event::Text("Le parc.".into())]);
        assert!(
            app.session
                .document
                .source()
                .contains("caption: [Le parc.]"),
            "{} / {:?}",
            app.session.document.source(),
            app.range()
        );
        let before = app.session.document.source().to_owned();
        let at = app.session.document.view.image_range(1).unwrap().start + 1;
        app.selection = editor::cursor(at..at);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let backspace = || egui::Event::Key {
            key: egui::Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, &ctx, vec![backspace()]);
        assert_eq!(app.range(), at - 1..at);
        frame(&mut app, &ctx, vec![backspace()]);
        assert!(app.session.document.view.images.is_empty());
        assert!(!app.session.document.view.text.contains("Le parc."));
        app.apply(Command::Undo);
        assert_eq!(app.session.document.source(), before);
    }

    #[test]
    fn images_insert_edit_caption_replace_undo_and_reopen_without_original_file() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.document = Document::from_source("Avant.\n\nAprès.").unwrap();
        app.selection = editor::cursor(6..6);
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("Parc.svg");
        std::fs::write(&original, include_bytes!("../tests/fixtures/parc.svg")).unwrap();
        app.import_image(&original, None).unwrap();
        assert_eq!(app.session.document.view.images.len(), 1);
        let id = *app.session.document.view.images.keys().next().unwrap();
        let object = app.session.document.view.image_range(id).unwrap().start;
        assert_eq!(app.range(), object + 1..object + 1);
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Le parc municipal.".into())],
        );
        assert!(
            app.session
                .document
                .source()
                .contains("caption: [Le parc municipal.]"),
            "{}",
            app.session.document.source()
        );
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
        assert_eq!(app.range(), object + 1..object + 1);
        let before_alt = app.snapshot();
        app.session.document.view.images.get_mut(&id).unwrap().alt =
            "Trois arbres autour d’un chemin".into();
        app.remember(before_alt, false);
        app.import_image(&original, Some(id)).unwrap();
        assert_eq!(
            app.session.document.view.images[&id].alt,
            "Trois arbres autour d’un chemin"
        );
        assert!(app.session.document.source().contains("Le parc municipal."));
        app.apply(Command::Undo);
        assert_eq!(
            app.session.document.view.images[&id].path,
            "images/image-1.svg"
        );
        app.apply(Command::Redo);
        assert_eq!(
            app.session.document.view.images[&id].path,
            "images/image-2.svg"
        );
        let archive = dir.path().join("Courrier.typsto");
        assert!(app.save_path(archive.clone()));
        std::fs::remove_file(&original).unwrap();
        let saved_source = app.session.document.source().to_owned();
        app.remove_image(id);
        assert!(app.session.document.view.images.is_empty());
        app.apply(Command::Undo);
        assert_eq!(app.session.document.source(), saved_source);
        assert!(app.open_path(archive));
        assert!(!app.is_dirty());
        assert_eq!(app.session.document.source(), saved_source);
        assert_eq!(app.session.document.assets.len(), 1);
        app.selection = editor::cursor(object + 1..object + 1);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(
            app.session.document.view.blocks[app.session.document.view.line_at(app.range().start)],
            BlockKind::Paragraph
        );
        assert!(app.session.document.source().contains("Le parc municipal."));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Suite du courrier.".into())],
        );
        assert!(app.session.document.source().contains("Suite du courrier."));
    }

    #[test]
    fn column_drag_and_window_resize_keep_the_chosen_proportion_without_a_preview_cap() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut time = 0.0;
        let mut draw = |app: &mut App, width, events| {
            time += 0.1;
            let mut rects = [egui::Rect::NOTHING; 2];
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 900.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            rects = app.columns_ui(ui, false);
                        });
                },
            );
            rects
        };
        let initial = draw(&mut app, 1240.0, vec![]);
        assert!((initial[0].width() / initial[1].width() - 1.0).abs() < 0.01);
        let start = egui::pos2(initial[0].right() + 4.0, 450.0);
        draw(
            &mut app,
            1240.0,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        let end = egui::pos2(374.0, 450.0);
        draw(&mut app, 1240.0, vec![egui::Event::PointerMoved(end)]);
        draw(
            &mut app,
            1240.0,
            vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let fraction = app.preferences.editor_fraction;
        assert!((fraction - 0.3).abs() < 0.02, "{fraction}");
        for width in [960.0, 2400.0, 1240.0] {
            let rects = draw(&mut app, width, vec![]);
            let actual = rects[0].width() / (rects[0].width() + rects[1].width());
            assert!((actual - fraction).abs() < 0.001);
            if width > 2000.0 {
                assert!(rects[1].width() > 1100.0);
            }
        }
        app.preferences.editor_fraction = 0.9;
        let rects = draw(&mut app, 1240.0, vec![]);
        assert!(rects[1].width() < 300.0);
        let divider = egui::pos2(rects[0].right() + 4.0, 450.0);
        for _ in 0..2 {
            for pressed in [true, false] {
                draw(
                    &mut app,
                    1240.0,
                    vec![
                        egui::Event::PointerMoved(divider),
                        egui::Event::PointerButton {
                            pos: divider,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        assert_eq!(app.preferences.editor_fraction, 0.5);
    }

    #[test]
    fn editor_preferences_change_glyphs_and_theme_without_touching_the_letter() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let letter = app.stored_document();
        let revision = app.session.revision;
        app.dispatch_action(Command::Preferences, &ctx);
        assert!(app.preferences_open);
        let mut time = 0.0;
        let mut draw = |app: &mut App, events| {
            time += 0.1;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1240.0, 900.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.preferences_ui(ctx),
            )
        };
        for (current, next) in [
            ("Libertinus Serif", "DejaVu Sans Mono"),
            ("Système", "Sombre"),
        ] {
            for label in [current, next] {
                draw(&mut app, vec![]);
                let output = draw(&mut app, vec![]);
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            Some(text.pos + text.galley.rect.center().to_vec2())
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("choix absent : {label}"));
                for pressed in [true, false] {
                    draw(
                        &mut app,
                        vec![
                            egui::Event::PointerMoved(pos),
                            egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                }
            }
        }
        assert_eq!(app.preferences.editor.font, EditorFont::Monospace);
        app.preferences.editor.size = 24.0;
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.preferences_ui(ctx));
        assert_eq!(ctx.theme(), egui::Theme::Dark);
        app.preferences.appearance = Appearance::System;
        let _ = ctx.run(
            egui::RawInput {
                system_theme: Some(egui::Theme::Light),
                ..Default::default()
            },
            |ctx| app.preferences_ui(ctx),
        );
        let _ = ctx.run(
            egui::RawInput {
                system_theme: Some(egui::Theme::Light),
                ..Default::default()
            },
            |_| {},
        );
        assert_eq!(ctx.theme(), egui::Theme::Light);
        let mut view = Document::from_source("Normal\n\n= Titre").unwrap().view;
        view.toggle_mark(0..3, Mark::Bold);
        view.toggle_mark(0..3, Mark::Italic);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let galley = editor::layout(ui, &view, &view.text, 500.0, app.preferences.editor);
                assert_eq!(
                    galley.job.sections[0].format.font_id,
                    EditorFont::Monospace.font_id(24.0, 3)
                );
                let h1_size = 24.0 * 1.2_f32.powi(3);
                assert!(
                    galley
                        .job
                        .sections
                        .iter()
                        .any(|s| (s.format.font_id.size - h1_size).abs() < 0.001)
                );
            });
        });
        assert_eq!(app.stored_document(), letter);
        assert_eq!(app.session.revision, revision);
        assert!(!app.is_dirty());
    }

    #[test]
    fn general_settings_and_template_selection_preserve_letter_content() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.template = "#let courrier(..args, options: (:), body) = { set page(paper: options.paper, flipped: options.landscape); body }".into();
        app.session.saved.template = app.session.template.clone();
        app.information_open = true;
        ctx.style_mut(|s| s.animation_time = 0.0);
        let source = app.session.document.source().to_owned();
        let revision = app.session.revision;
        let mut time = 0.0;
        let mut draw = |app: &mut App, events| {
            time += 0.1;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1240.0, 900.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        fn point(output: &egui::FullOutput, value: &str) -> egui::Pos2 {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == value => {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("champ ou choix absent : {value}"))
        }
        fn click(
            app: &mut App,
            draw: &mut impl FnMut(&mut App, Vec<egui::Event>) -> egui::FullOutput,
            pos: egui::Pos2,
        ) {
            for pressed in [true, false] {
                draw(
                    app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        fn choose(
            app: &mut App,
            draw: &mut impl FnMut(&mut App, Vec<egui::Event>) -> egui::FullOutput,
            current: &str,
            next: &str,
        ) {
            draw(app, vec![]);
            let output = draw(app, vec![]);
            click(app, draw, point(&output, current));
            let output = draw(app, vec![]);
            click(app, draw, point(&output, next));
        }
        choose(&mut app, &mut draw, "A4", "A3");
        choose(&mut app, &mut draw, "Portrait", "Paysage");
        choose(&mut app, &mut draw, "Français", "Anglais");
        choose(
            &mut app,
            &mut draw,
            "Modèle du document",
            "Courrier personnel",
        );
        assert_eq!(app.session.metadata.options_mut().paper, PaperFormat::A4);
        assert!(!app.session.metadata.options_mut().landscape);
        assert_eq!(
            app.session.metadata.options_mut().language,
            DocumentLanguage::English
        );
        assert_eq!(app.session.template, Model::Personal.source());
        assert!(app.is_dirty());
        assert_eq!(app.session.revision, revision + 4);
        assert_eq!(app.session.document.source(), source);

        assert!(!app.window_title.contains("typstOgraphe"));
    }

    #[test]
    fn egui_file_menu_exports_and_quit_preserves_unsaved_work_without_header_duplicates() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.file_path = Some(PathBuf::from("exemple.typsto"));
        let source = app.session.document.source().to_owned();
        let mut time = 0.0;
        let mut draw = |app: &mut App, events| {
            time += 0.1;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1240.0, 900.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        let text_point = |output: &egui::FullOutput, value: &str| {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == value => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
        };
        draw(&mut app, vec![]);
        let output = draw(&mut app, vec![]);
        assert!(text_point(&output, "exemple.typsto").is_none());
        assert!(text_point(&output, "Enregistrer").is_none());
        assert!(text_point(&output, "Exporter").is_none());
        for label in ["Effacer", "Retirer la mise en forme", "Annuler", "Rétablir"] {
            assert!(
                text_point(&output, label).is_none(),
                "{label} reste dans la barre d’outils"
            );
        }
        assert!(app.window_title.contains("exemple.typsto"));
        let pos = text_point(&output, "Fichier").unwrap();
        for pressed in [true, false] {
            draw(
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        let output = draw(&mut app, vec![]);
        let pos = text_point(&output, &Command::File(FileCommand::Export).label())
            .expect("export dans Fichier");
        for pressed in [true, false] {
            draw(
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(app.export_open);
        assert_eq!(app.session.document.source(), source);
        // Une commande native déjà en transit ne contourne pas un dialogue.
        app.dispatch_action(Command::File(FileCommand::Quit), &ctx);
        assert!(app.file_command.is_none());
        app.export_open = false;
        app.session.metadata.courrier_mut().objet = "Brouillon à conserver".into();
        let before = app.stored_document();
        app.dispatch_action(Command::File(FileCommand::Quit), &ctx);
        let output = draw(&mut app, vec![]);
        assert!(matches!(app.pending_action, Some(PendingAction::Close)));
        assert!(!app.allow_close);
        assert_eq!(app.stored_document(), before);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .all(|command| !matches!(command, egui::ViewportCommand::Close))
        );
    }

    #[test]
    fn inline_information_keeps_text_accessible_and_metadata_edits_refresh_the_document() {
        fn text_rect(output: &egui::FullOutput, value: &str) -> egui::Rect {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == value => Some(
                        text.galley
                            .rect
                            .translate(text.pos.to_vec2())
                            .intersect(shape.clip_rect),
                    ),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("texte absent de l’interface : {value}"))
        }
        fn click(
            app: &mut App,
            draw: &mut impl FnMut(&mut App, Vec<egui::Event>) -> egui::FullOutput,
            pos: egui::Pos2,
        ) {
            for pressed in [true, false] {
                draw(
                    app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        ctx.style_mut(|style| style.animation_time = 0.0);
        app.selection = editor::cursor(10..20);
        app.force_selection = true;
        let source = app.session.document.source().to_owned();
        let selection = app.selection;
        let revision = app.session.revision;
        let mut time = 0.0;
        let mut draw = |app: &mut App, events| {
            time += 0.1;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(960.0, 640.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        draw(&mut app, vec![]);
        let closed = draw(&mut app, vec![]);
        let header = text_rect(&closed, "Général");
        assert!(header.top() > icon_rect(&closed, &ctx, crate::icons::Icon::Heading).bottom());
        for label in ["Destinataire", "Informations", "Expéditeur"] {
            let button = text_rect(&closed, label);
            assert_eq!(button.center().y, header.center().y);
        }
        let has_text = |output: &egui::FullOutput, value: &str| {
            output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == value)
            })
        };
        assert!(!has_text(&closed, "Format du papier"));
        let preview = text_rect(&closed, "Aperçu Typst");
        click(&mut app, &mut draw, header.center());
        draw(&mut app, vec![]);
        let opened = draw(&mut app, vec![]);
        assert!(app.information_open);
        assert!(!app.modal_open());
        assert!(!app.is_dirty());
        assert_eq!(app.session.revision, revision);
        assert_eq!(text_rect(&opened, "Aperçu Typst"), preview);
        assert!(has_text(&opened, "Format du papier"));
        assert!(!has_text(&opened, "Adresse"));
        assert!(!has_text(&opened, "Nom complet"));
        // Même à la taille minimale de la fenêtre, le formulaire reste dans la
        // colonne de gauche et laisse une zone de rédaction utilisable.
        let body = text_rect(&opened, &app.session.document.view.text);
        assert!(body.height() > 150.0, "zone de rédaction : {body:?}");
        click(
            &mut app,
            &mut draw,
            text_rect(&opened, "Expéditeur").center(),
        );
        draw(&mut app, vec![]);
        let sender = draw(&mut app, vec![]);
        assert!(has_text(&sender, "Nom complet"));
        assert!(!has_text(&sender, "Format du papier"));
        assert!(!has_text(&sender, "Adresse"));
        let sender_button = text_rect(&sender, "Expéditeur");
        click(&mut app, &mut draw, sender_button.center());
        let collapsed = draw(&mut app, vec![]);
        assert!(!app.information_open);
        assert!(!has_text(&collapsed, "Nom complet"));
        assert!(text_rect(&collapsed, &app.session.document.view.text).height() >= body.height());
        app.apply(Command::Information);
        let reopened = draw(&mut app, vec![]);
        assert!(app.information_open);
        assert_eq!(app.information_group, InformationGroup::Fields("sender"));
        assert!(has_text(&reopened, "Nom complet"));
        assert_eq!(app.session.revision, revision);
        assert!(!app.is_dirty());
        assert_eq!(app.session.document.source(), source);
        assert_eq!(app.selection, selection);
        click(
            &mut app,
            &mut draw,
            text_rect(&reopened, "Informations").center(),
        );
        draw(&mut app, vec![]);
        let correspondence = draw(&mut app, vec![]);
        let original_subject = app.session.metadata.courrier_mut().objet.clone();
        let subject = text_rect(&correspondence, &app.session.metadata.courrier_mut().objet);
        assert!(subject.is_positive());
        assert!(subject.right() < preview.left());
        click(&mut app, &mut draw, subject.center());
        draw(
            &mut app,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Text("Objet modifié".into()),
            ],
        );
        assert_eq!(app.session.metadata.courrier_mut().objet, "Objet modifié");
        assert_eq!(app.session.document.source(), source);
        assert_eq!(app.selection, selection);
        assert_eq!(app.session.revision, revision + 1);
        assert!(app.is_dirty());
        assert!(app.information_open);
        app.apply(Command::Palette);
        app.palette_query = "Annuler".into();
        draw(&mut app, vec![]);
        draw(
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                modifiers: egui::Modifiers::NONE,
                physical_key: None,
                pressed: true,
                repeat: false,
            }],
        );
        assert!(!app.palette_open);
        draw(&mut app, vec![]);
        assert_eq!(app.session.metadata.courrier_mut().objet, original_subject);
        assert_eq!(app.session.document.source(), source);
        app.dispatch_action(Command::Redo, &ctx);
        draw(&mut app, vec![]);
        assert_eq!(app.session.metadata.courrier_mut().objet, "Objet modifié");
        assert_eq!(app.session.revision, revision + 3);
        let current = draw(&mut app, vec![]);
        click(
            &mut app,
            &mut draw,
            text_rect(&current, "Informations").center(),
        );
        draw(&mut app, vec![]);
        assert!(!app.information_open);
        assert_eq!(app.session.metadata.courrier_mut().objet, "Objet modifié");
        assert_eq!(app.session.revision, revision + 3);
    }

    #[test]
    fn information_and_presentations_work_from_palette_and_custom_shortcuts() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.template.push_str("\n// modèle embarqué");
        app.session.saved.template = app.session.template.clone();
        let source = app.session.document.source().to_owned();
        let revision = app.session.revision;
        app.apply(Command::Palette);
        app.palette_query = "Informations du document".into();
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                modifiers: egui::Modifiers::NONE,
                pressed: true,
                repeat: false,
                physical_key: None,
            }],
        );
        assert!(app.information_open);
        assert!(!app.palette_open);
        assert_eq!(app.session.revision, revision);
        assert!(!app.is_dirty());
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                modifiers: egui::Modifiers::NONE,
                pressed: true,
                repeat: false,
                physical_key: None,
            }],
        );
        app.apply(Command::Palette);
        app.palette_query = "modèle de courrier personnel".into();
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                modifiers: egui::Modifiers::NONE,
                pressed: true,
                repeat: false,
                physical_key: None,
            }],
        );
        assert_eq!(app.session.template, Model::Personal.source());
        assert!(app.is_dirty());
        assert_eq!(app.session.revision, revision + 1);
        assert_eq!(app.stored_document().template, Model::Personal.source());
        assert_eq!(app.session.document.source(), source);
        app.apply(Command::Model(Model::Personal));
        assert_eq!(app.session.revision, revision + 1); // Choisir la présentation active est sans effet.

        app.apply(Command::Undo);
        let config = serde_json::from_str(
            r#"{"document.information":[{"key":"F6"}],"courrier.model.personal":[{"key":"F7"}]}"#,
        )
        .unwrap();
        app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &config).unwrap();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::F7,
                modifiers: egui::Modifiers::NONE,
                pressed: true,
                repeat: false,
                physical_key: None,
            }],
        );
        assert_eq!(app.session.template, Model::Personal.source());
        assert_eq!(app.session.revision, revision + 3);
        assert!(app.is_dirty());
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::F6,
                modifiers: egui::Modifiers::NONE,
                pressed: true,
                repeat: false,
                physical_key: None,
            }],
        );
        assert!(app.information_open);
        assert_eq!(app.session.document.source(), source);
    }

    #[test]
    fn remove_link_keeps_other_styles_and_selection_and_can_be_undone() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx, None);
        app.session.document = Document::from_source(
            "Avant #link(\"https://example.org\")[*Élodie* et _Noël_] après.",
        )
        .unwrap();
        let source = app.session.document.source().to_owned();
        let start = 6;
        let end_byte = app.session.document.view.text.find(" après.").unwrap();
        let end = app.session.document.view.text[..end_byte].chars().count();
        app.selection = editor::cursor(end..start);
        let selection = app.selection;
        let text = app.session.document.view.text.clone();
        let styles = app.session.document.view.styles.clone();
        app.apply(Command::RemoveLink);
        assert_eq!(app.selection, selection);
        assert_eq!(app.session.document.view.text, text);
        for (before, after) in styles.iter().zip(&app.session.document.view.styles) {
            assert_eq!(before.bold, after.bold);
            assert_eq!(before.italic, after.italic);
            assert!(after.link.is_none());
        }
        app.apply(Command::Undo);
        assert_eq!(app.session.document.source(), source);
        assert_eq!(app.selection, selection);
        app.apply(Command::Redo);
        assert!(
            app.session
                .document
                .view
                .styles
                .iter()
                .all(|style| style.link.is_none())
        );
    }

    #[test]
    fn shortcut_override_updates_dispatch_and_hints_without_changing_the_document() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        frame(&mut app, &ctx, vec![]);
        let command = Command::File(FileCommand::Export);
        let original = app.keymap.hint(&ctx, command);
        let config = serde_json::from_str(r#"{"document.export":[{"key":"F6"}]}"#).unwrap();
        app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &config).unwrap();
        let hint = app.keymap.hint(&ctx, command);
        assert_ne!(hint, original);
        assert!(hint.contains("F6"));
        let source = app.session.document.source().to_owned();
        for (key, modifiers) in [
            (egui::Key::E, egui::Modifiers::COMMAND),
            (egui::Key::F6, egui::Modifiers::NONE),
        ] {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            );
            assert_eq!(app.export_open, key == egui::Key::F6);
        }
        assert_eq!(app.session.document.source(), source);
        assert!(!app.is_dirty());
    }

    #[test]
    fn save_reopen_restores_the_letter_and_failed_open_preserves_current_work() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("courrier.typsto");
        assert!(!app.is_dirty());
        app.selection = editor::cursor(20..35);
        app.apply(Command::Bold);
        assert!(app.is_dirty());
        app.apply(Command::Undo);
        assert!(!app.is_dirty());
        app.session.metadata.courrier_mut().objet = "Objet personnalisé".into();
        app.session.template = Model::Personal.source();
        app.session.template = app
            .session
            .template
            .replace("VILLE DE VALMONT", "MODÈLE ARCHIVÉ");
        app.session.document.view.insert_text("Élodie : ", 0);
        app.changed();
        let letter = app.stored_document();
        assert!(app.save_path(path.clone()));
        assert!(!app.is_dirty());
        app.session.document.view.insert_text("Brouillon ", 0);
        app.changed();
        assert!(app.is_dirty());
        assert!(app.open_path(path.clone()));
        assert_eq!(app.stored_document(), letter);
        assert_eq!(app.session.file_path, Some(path.clone()));
        assert!(!app.is_dirty());
        assert!(app.session.undo.is_empty() && app.session.redo.is_empty());
        assert_eq!(app.selection.primary.index, 0);
        assert!(app.pages.is_empty() && app.sync.is_none());
        assert_eq!(app.request(None).template, letter.template);
        app.session.document.view.insert_text("À conserver ", 0);
        app.changed();
        let before = app.stored_document();
        let bad = dir.path().join("illisible.typsto");
        std::fs::write(&bad, "invalide").unwrap();
        assert!(!app.open_path(bad));
        assert_eq!(app.stored_document(), before);
        assert_eq!(app.session.file_path, Some(path.clone()));
        assert!(app.is_dirty());
        assert!(!app.save_path(dir.path().join("absent/impossible.typsto")));
        assert_eq!(app.session.file_path, Some(path.clone()));
        assert!(app.is_dirty());
        assert_eq!(storage::read(&path).unwrap(), letter);
    }

    #[test]
    fn save_shortcut_and_save_as_keep_both_files_and_track_clean_state() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("premier.typsto");
        assert!(app.save_path(first.clone()));
        let initial = storage::read(&first).unwrap();
        app.session.document.view.insert_text("Un ajout. ", 0);
        app.changed();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::S,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
        );
        assert!(!app.is_dirty());
        assert_ne!(storage::read(&first).unwrap(), initial);
        let saved = app.stored_document();
        app.session.metadata.courrier_mut().objet = "Deuxième courrier".into();
        let second = dir.path().join("second.typsto");
        assert!(app.save_path(second.clone()));
        assert_eq!(app.session.file_path, Some(second));
        assert_eq!(storage::read(&first).unwrap(), saved);
        assert!(!app.is_dirty());
    }

    #[test]
    fn unsaved_work_blocks_closing_or_replacing_until_the_user_decides() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.metadata.courrier_mut().objet = "Travail à conserver".into();
        let before = app.stored_document();
        app.request_action(PendingAction::Open, &ctx);
        assert!(app.pending_action.is_some()); // Aucune fenêtre de fichier tant que la décision manque.
        assert_eq!(app.stored_document(), before);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.pending_action.is_none());
        assert_eq!(app.stored_document(), before);

        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let output = ctx.run(input, |ctx| app.ui_frame(ctx));
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );
        assert!(app.pending_action.is_some());
        assert_eq!(app.stored_document(), before);
        app.pending_action = None;
        app.request_action(PendingAction::New, &ctx);
        let draw = |app: &mut App, time, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1240.0, 900.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        draw(&mut app, 1.0, vec![]);
        let output = draw(&mut app, 1.1, vec![]);
        let pointer = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Ne pas enregistrer" => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .expect("bouton de confirmation");
        for (i, pressed) in [true, false].into_iter().enumerate() {
            draw(
                &mut app,
                1.2 + i as f64 * 0.05,
                vec![
                    egui::Event::PointerMoved(pointer),
                    egui::Event::PointerButton {
                        pos: pointer,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(app.pending_action.is_none());
        assert!(app.new_document_open);
        app.create_document(DocumentType::Courrier);
        assert!(app.session.document.source().is_empty());
        assert!(app.session.file_path.is_none());
        assert!(!app.is_dirty());
    }

    fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1240.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| app.ui_frame(ctx));
    }

    #[test]
    fn synchronization_scrolls_both_panes_and_rejects_outdated_or_failed_renders() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.document =
            Document::from_source(&"Un paragraphe à retrouver.\n\n".repeat(100)).unwrap();
        let mut world = crate::rendering::TypstWorld::default();
        let rendered = world
            .compile_with_template(
                app.session.document.source(),
                &app.session.metadata,
                &app.session.template,
            )
            .unwrap();
        for page in 0..rendered.pages().len() {
            app.pages.push(ctx.load_texture(
                format!("test-page-{page}"),
                egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
                egui::TextureOptions::LINEAR,
            ));
        }
        app.sync = Some(RenderSync::new(
            app.session.revision,
            world,
            rendered,
            app.session.document.source_map.clone(),
        ));
        let end = app.session.document.view.len() - 12;
        app.selection = editor::cursor(end..end);
        let location = app.sync.as_ref().unwrap().forward(end).unwrap();

        let mut draw_pane = |preview: bool, time: f64| {
            let mut offset = 0.0;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 500.0),
                    )),
                    time: Some(time),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let id = ui.make_persistent_id(egui::Id::new(if preview {
                            "preview-scroll"
                        } else {
                            "editor-scroll"
                        }));
                        if preview {
                            app.preview_ui(ui);
                        } else {
                            app.editor_ui(ui);
                        }
                        offset = egui::scroll_area::State::load(ctx, id).unwrap().offset.y;
                    });
                },
            );
            offset
        };
        let mut offset = 0.0;
        for i in 0..10 {
            offset = draw_pane(true, f64::from(i) / 10.0);
        }
        assert!(offset > 1000.0, "preview offset {offset}");

        app.selection = editor::cursor(0..0);
        app.session.revision += 1;
        app.navigate_backward(location, &ctx);
        assert_eq!(app.selection.primary.index, 0);
        app.session.revision -= 1;
        app.rendering_error = Some("essai".into());
        app.navigate_backward(location, &ctx);
        assert_eq!(app.selection.primary.index, 0);
        app.rendering_error = None;
        app.navigate_backward(location, &ctx);
        assert!(app.selection.primary.index.abs_diff(end) <= 1);
        assert!(!app.forward_pending);

        for i in 10..20 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 500.0),
                    )),
                    time: Some(f64::from(i) / 10.0),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let id = ui.make_persistent_id(egui::Id::new("editor-scroll"));
                        app.editor_ui(ui);
                        offset = egui::scroll_area::State::load(ctx, id).unwrap().offset.y;
                    });
                },
            );
        }
        assert!(offset > 1000.0, "editor offset {offset}");
        assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
        assert!(app.selection.primary.index.abs_diff(end) <= 1);
        assert!(!app.editor_scroll_pending);
        assert!(!app.forward_pending);
    }

    #[test]
    fn double_click_on_the_scaled_preview_returns_focus_to_the_editor() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut world = crate::rendering::TypstWorld::default();
        let rendered = world
            .compile_with_template(
                app.session.document.source(),
                &app.session.metadata,
                &app.session.template,
            )
            .unwrap();
        let texture = ctx.load_texture(
            "click-test",
            egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
            egui::TextureOptions::LINEAR,
        );
        let texture_id = texture.id();
        app.pages = vec![texture];
        app.sync = Some(RenderSync::new(
            app.session.revision,
            world,
            rendered,
            app.session.document.source_map.clone(),
        ));
        app.submitted = app.session.revision;
        app.rendered_revision = Some(app.session.revision);
        let index = app
            .session
            .document
            .view
            .text
            .find("Notre")
            .map(|byte| app.session.document.view.text[..byte].chars().count())
            .unwrap();
        let location = app.sync.as_ref().unwrap().forward(index).unwrap();
        let size = app.sync.as_ref().unwrap().page_size(location.page).unwrap();
        let draw = |app: &mut App, time, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1240.0, 900.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        draw(&mut app, 0.0, vec![]);
        let output = draw(&mut app, 0.1, vec![]);
        let rect = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Rect(rect)
                    if rect
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == texture_id) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("image de l’aperçu");
        let pointer = rect.min
            + egui::vec2(
                location.point[0] / size[0] * rect.width(),
                location.point[1] / size[1] * rect.height(),
            );
        for (step, pressed) in [true, false, true, false].into_iter().enumerate() {
            draw(
                &mut app,
                0.2 + step as f64 * 0.05,
                vec![
                    egui::Event::PointerMoved(pointer),
                    egui::Event::PointerButton {
                        pos: pointer,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(app.selection.primary.index, index);
        assert!(app.selection.is_empty());
        assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
        let mut expected = app.session.document.view.text.clone();
        expected.insert_str(app.session.document.view.byte_index(index), "Ajout ");
        draw(&mut app, 0.5, vec![egui::Event::Text("Ajout ".into())]);
        assert_eq!(app.session.document.view.text, expected);
    }

    #[test]
    fn preview_paints_selection_at_page_scale_and_clears_it_when_unavailable() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut world = crate::rendering::TypstWorld::default();
        let rendered = world
            .compile_with_template(
                app.session.document.source(),
                &app.session.metadata,
                &app.session.template,
            )
            .unwrap();
        app.pages = vec![ctx.load_texture(
            "highlight-test",
            egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
            egui::TextureOptions::LINEAR,
        )];
        app.sync = Some(RenderSync::new(
            app.session.revision,
            world,
            rendered,
            app.session.document.source_map.clone(),
        ));
        app.submitted = app.session.revision;
        app.rendered_revision = Some(app.session.revision);
        let byte = app.session.document.view.text.find("jeudi").unwrap();
        let index = app.session.document.view.text[..byte].chars().count();
        app.selection = editor::cursor(index..index + 5);
        let draw = |app: &mut App, width| {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 900.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.preview_ui(ui));
                },
            );
            output
                .shapes
                .into_iter()
                .filter_map(|shape| match shape.shape {
                    egui::epaint::Shape::Path(path) if path.fill == PREVIEW_SELECTION => {
                        Some((shape.clip_rect, path))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        draw(&mut app, 600.0);
        let wide = draw(&mut app, 600.0);
        assert_eq!(wide.len(), 1);
        let (clip, path) = &wide[0];
        assert!(path.points.iter().all(|point| clip.contains(*point)));
        let wide_width = path.points[1].x - path.points[0].x;
        let narrow = draw(&mut app, 400.0);
        assert_eq!(narrow.len(), 1);
        assert!(narrow[0].1.points[1].x - narrow[0].1.points[0].x < wide_width);
        // Le sens de la sélection ne change pas le passage surligné.
        app.selection = editor::cursor(index + 5..index);
        assert_eq!(draw(&mut app, 400.0)[0].1.points, narrow[0].1.points);
        app.selection = editor::cursor(index..index);
        assert!(draw(&mut app, 400.0).is_empty());
        app.selection = editor::cursor(index..index + 5);
        app.session.revision += 1;
        assert!(draw(&mut app, 400.0).is_empty());
        app.session.revision -= 1;
        app.rendering_error = Some("essai".into());
        assert!(draw(&mut app, 400.0).is_empty());
        app.rendering_error = None;
        assert!(!draw(&mut app, 400.0).is_empty());
    }

    #[test]
    fn cross_paragraph_formatting_undo_restores_source_and_selection() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx, None);
        let source = app.session.document.source().to_owned();
        app.selection = editor::cursor(3..180);
        let selection = app.selection;
        app.apply(Command::Bold);
        assert!(app.session.document.view.mark_state(3..180, Mark::Bold).0);
        assert_eq!(app.selection, selection);
        app.apply(Command::Undo);
        assert_eq!(app.session.document.source(), source);
        assert_eq!(app.selection, selection);
        app.apply(Command::Redo);
        assert!(app.session.document.view.mark_state(3..180, Mark::Bold).0);
    }

    #[test]
    fn native_egui_replaces_a_unicode_selection_without_source_markers() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.document =
            Document::from_source("Élodie écrit.\n\nDeuxième paragraphe.").unwrap();
        app.selection = editor::cursor(0..6);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![egui::Event::Text("Noël".into())]);
        assert!(
            app.session.document.view.text.starts_with("Noël écrit."),
            "{}",
            app.session.document.view.text
        );
        assert_eq!(
            app.session.document.view.len(),
            app.session.document.view.text.chars().count()
        );
        assert!(Document::from_source(app.session.document.source()).is_ok());
        app.apply(Command::Undo);
        assert!(app.session.document.view.text.starts_with("Élodie écrit."));
    }

    #[test]
    fn native_keyboard_shortcut_formats_multiple_paragraphs() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.selection = editor::cursor(0..180);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::I,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
        );
        assert!(app.session.document.view.mark_state(0..180, Mark::Italic).0);
        assert!(!app.session.document.view.text.contains("#emph"));
        assert_eq!(app.selection.as_sorted_char_range(), 0..180);
    }

    #[test]
    fn text_shortcuts_can_be_undone_to_the_complete_literal_input() {
        for (source, closing, expected, kind) in [
            ("\\=", " ", "= ", Some(BlockKind::Heading(1))),
            ("\\/", " ", "/ ", Some(BlockKind::Description)),
            ("\\>", " ", "> ", Some(BlockKind::Quote)),
            ("\\*Bonjour", "*", "*Bonjour*", None),
        ] {
            let ctx = egui::Context::default();
            let mut app = App::with_demo_context(ctx.clone(), None);
            app.session.document = Document::from_source(source).unwrap();
            let end = app.session.document.view.len();
            app.selection = editor::cursor(end..end);
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            frame(&mut app, &ctx, vec![egui::Event::Text(closing.into())]);
            if let Some(kind) = kind {
                assert_eq!(app.session.document.view.blocks[0], kind);
            } else {
                assert!(app.session.document.view.mark_state(0..7, Mark::Bold).0);
            }
            app.apply(Command::Undo);
            assert_eq!(app.session.document.view.text, expected);
        }
    }

    #[test]
    fn description_editing_uses_tab_and_colon_without_exposing_typst() {
        for separator in [
            egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Text(" ".into()),
        ] {
            let ctx = egui::Context::default();
            let mut app = App::with_demo_context(ctx.clone(), None);
            app.session.document = Document::from_source("Échéance").unwrap();
            app.selection = editor::cursor(8..8);
            app.apply(Command::Block(BlockKind::Description));
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            if matches!(separator, egui::Event::Text(_)) {
                frame(&mut app, &ctx, vec![egui::Event::Text(":".into())]);
            }
            frame(&mut app, &ctx, vec![separator]);
            frame(&mut app, &ctx, vec![egui::Event::Text("Vendredi".into())]);
            assert_eq!(app.session.document.view.text, "Échéance\tVendredi");
            assert_eq!(app.session.document.view.blocks, [BlockKind::Description]);
            let restored = Document::from_source(app.session.document.source()).unwrap();
            assert_eq!(restored.view.text, app.session.document.view.text);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::SHIFT,
                }],
            );
            assert_eq!(app.selection.primary.index, 0);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(app.selection.primary.index, 9);
            let end = app.session.document.view.len();
            app.selection = editor::cursor(end..end);
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(
                app.session.document.view.blocks,
                [BlockKind::Description; 2]
            );
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(
                app.session.document.view.blocks,
                [BlockKind::Description, BlockKind::Paragraph]
            );
        }
    }

    #[test]
    fn quote_keyboard_edits_body_and_attribution_and_exits_without_losing_source() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.document = Document::from_source("Bonjour").unwrap();
        app.selection = editor::cursor(7..7);
        app.apply(Command::Block(BlockKind::Quote));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(
            app.session.document.view.blocks,
            [BlockKind::Quote, BlockKind::QuoteAttribution]
        );
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
        frame(&mut app, &ctx, vec![egui::Event::Text("Élodie".into())]);
        assert_eq!(app.session.document.view.text, "Bonjour\nÉlodie");
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
        );
        assert_eq!(app.selection.primary.index, 0);
        app.selection = editor::cursor(7..7);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Deuxième pensée.".into())],
        );
        assert_eq!(
            app.session.document.view.text,
            "Bonjour\nDeuxième pensée.\nÉlodie"
        );
        assert_eq!(
            app.session.document.view.blocks,
            [
                BlockKind::Quote,
                BlockKind::Quote,
                BlockKind::QuoteAttribution
            ]
        );
        let source = app.session.document.source().to_owned();
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(
            app.session.document.view.blocks,
            [
                BlockKind::Quote,
                BlockKind::Quote,
                BlockKind::QuoteAttribution,
                BlockKind::Paragraph
            ]
        );
        frame(&mut app, &ctx, vec![egui::Event::Text("Suite.".into())]);
        let restored = Document::from_source(app.session.document.source()).unwrap();
        assert_eq!(restored.view, app.session.document.view);
        app.apply(Command::Undo);
        app.apply(Command::Undo);
        app.apply(Command::Undo);
        assert_eq!(app.session.document.source(), source);
    }

    #[test]
    fn converting_multiple_paragraphs_to_quote_preserves_selection_and_can_be_undone() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx, None);
        app.session.document = Document::from_source("Un.\n\nDeux.\n\nTrois.").unwrap();
        let original = app.session.document.clone();
        app.selection = CCursorRange::two(CCursor::new(3), CCursor::new(0));
        app.apply(Command::Block(BlockKind::Quote));
        assert_eq!(app.selection.primary.index, 0);
        assert_eq!(app.selection.secondary.index, 3);
        assert_eq!(app.session.document.view.text, "Un.\n\n\nDeux.\n\nTrois.");
        assert_eq!(
            app.session.document.view.blocks,
            [
                BlockKind::Quote,
                BlockKind::QuoteAttribution,
                BlockKind::Paragraph,
                BlockKind::Paragraph,
                BlockKind::Paragraph,
                BlockKind::Paragraph
            ]
        );
        assert_eq!(
            Document::from_source(app.session.document.source())
                .unwrap()
                .view,
            app.session.document.view
        );
        app.apply(Command::Undo);
        assert_eq!(app.session.document, original);
        app.selection = editor::cursor(0..8);
        app.apply(Command::Block(BlockKind::Quote));
        assert_eq!(app.session.document.view.quote_groups()[0].body, 0..3);
        app.selection = editor::cursor(0..app.session.document.view.len());
        app.apply(Command::Block(BlockKind::Paragraph));
        assert!(
            !app.session
                .document
                .view
                .blocks
                .iter()
                .any(|kind| matches!(kind, BlockKind::Quote | BlockKind::QuoteAttribution))
        );
    }

    #[test]
    fn description_tab_with_a_selection_and_enter_in_text_create_an_empty_label() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        app.session.document = Document::from_source("/ Échéance: Vendredi").unwrap();
        app.selection = editor::cursor(2..5);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
        assert_eq!(app.selection.as_sorted_char_range(), 9..9);
        assert_eq!(app.session.document.view.text, "Échéance\tVendredi");
        app.selection = editor::cursor(12..12);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let before = app.session.document.source().to_owned();
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(app.session.document.view.text, "Échéance\tVen\n\tdredi");
        assert_eq!(app.selection.primary.index, 13);
        assert_eq!(
            app.session.document.view.blocks,
            [BlockKind::Description; 2]
        );
        app.apply(Command::Undo);
        assert_eq!(app.session.document.source(), before);
        app.apply(Command::Redo);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![egui::Event::Text("Contact".into())]);
        assert_eq!(
            app.session.document.view.text,
            "Échéance\tVen\nContact\tdredi"
        );
        let restored = Document::from_source(app.session.document.source()).unwrap();
        assert_eq!(restored.view, app.session.document.view);
    }

    #[test]
    fn toolbar_icons_dispatch_commands_and_keep_headings_separate_from_quotes() {
        use crate::icons::Icon;
        fn draw(
            app: &mut App,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> (egui::FullOutput, Option<Command>) {
            let mut action = None;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(520.0, 260.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = crate::toolbar::show(
                            ui,
                            &app.session.document.view,
                            app.range(),
                            &app.keymap,
                        );
                    });
                },
            );
            // Vérifier le dispatch de l’image sans ouvrir un sélecteur natif pendant le test.
            if let Some(command) = action
                && command != Command::InsertImage
            {
                app.apply(command);
            }
            (output, action)
        }
        fn click(app: &mut App, ctx: &egui::Context, pos: egui::Pos2) -> Option<Command> {
            let mut action = None;
            for pressed in [true, false] {
                action = draw(
                    app,
                    ctx,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                )
                .1
                .or(action);
            }
            action
        }
        fn text_point(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
        }
        for dark in [false, true] {
            let ctx = egui::Context::default();
            let mut app = App::with_demo_context(ctx.clone(), None);
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            ctx.style_mut(|style| style.animation_time = 0.0);
            app.session.document = Document::from_source("Été à Paris.").unwrap();
            app.selection = editor::cursor(0..3);
            let text = app.session.document.view.text.clone();
            for (icon, command, mark) in [
                (Icon::Bold, Command::Bold, Mark::Bold),
                (Icon::Italic, Command::Italic, Mark::Italic),
            ] {
                let (output, _) = draw(&mut app, &ctx, vec![]);
                let pos = icon_rect(&output, &ctx, icon).center();
                assert_eq!(click(&mut app, &ctx, pos), Some(command));
                assert!(app.session.document.view.mark_state(0..3, mark).0);
                assert_eq!(app.range(), 0..3);
                let (output, _) = draw(&mut app, &ctx, vec![]);
                let texture = icon.texture(&ctx, crate::toolbar::ICON_SIZE).id();
                let tint = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect
                                .brush
                                .as_ref()
                                .is_some_and(|brush| brush.fill_texture_id == texture) =>
                        {
                            Some(rect.fill)
                        }
                        _ => None,
                    })
                    .expect("icône active peinte");
                assert_eq!(tint.r(), tint.g());
                assert_eq!(tint.g(), tint.b());
            }
            let mut heading_icon = Icon::Heading;
            for (kind, next_icon) in [
                (BlockKind::Heading(1), Icon::Heading1),
                (BlockKind::Heading(2), Icon::Heading2),
                (BlockKind::Heading(3), Icon::Heading3),
                (BlockKind::Paragraph, Icon::Heading),
            ] {
                let (output, _) = draw(&mut app, &ctx, vec![]);
                click(
                    &mut app,
                    &ctx,
                    icon_rect(&output, &ctx, heading_icon).center(),
                );
                let (menu, _) = draw(&mut app, &ctx, vec![]);
                for icon in [
                    Icon::Paragraph,
                    Icon::Heading1,
                    Icon::Heading2,
                    Icon::Heading3,
                ] {
                    assert!(icon_rect(&menu, &ctx, icon).is_positive());
                }
                assert!(text_point(&menu, &Command::Block(BlockKind::Quote).label()).is_none());
                let command = Command::Block(kind);
                let pos = text_point(&menu, &command.short_label()).expect("entrée du menu Titres");
                assert_eq!(click(&mut app, &ctx, pos), Some(command));
                assert_eq!(app.session.document.view.blocks[0], kind);
                assert_eq!(app.range(), 0..3);
                heading_icon = next_icon;
            }
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let pos = icon_rect(&output, &ctx, Icon::Link).center();
            assert_eq!(click(&mut app, &ctx, pos), Some(Command::Link));
            assert!(app.link_open);
            assert_eq!(app.link_range, 0..3);
            app.link_open = false;
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let pos = icon_rect(&output, &ctx, Icon::Image).center();
            assert_eq!(click(&mut app, &ctx, pos), Some(Command::InsertImage));
            assert_eq!(app.session.document.view.text, text);
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let pos = icon_rect(&output, &ctx, Icon::Quote).center();
            assert_eq!(
                click(&mut app, &ctx, pos),
                Some(Command::Block(BlockKind::Quote))
            );
            assert_eq!(
                app.session.document.view.blocks,
                [BlockKind::Quote, BlockKind::QuoteAttribution]
            );
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let pos = icon_rect(&output, &ctx, Icon::Quote).center();
            assert_eq!(
                click(&mut app, &ctx, pos),
                Some(Command::Block(BlockKind::Paragraph))
            );
            assert_eq!(app.session.document.view.blocks[0], BlockKind::Paragraph);
        }
    }

    #[test]
    fn toolbar_groups_the_three_list_commands_in_a_single_menu() {
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut app = App::with_demo_context(ctx.clone(), None);
        let draw = |app: &mut App, events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.toolbar(ui));
                },
            )
        };
        fn point(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
            output.shapes.iter().find_map(|s| match &s.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
        }
        for kind in [
            BlockKind::Bullet,
            BlockKind::Numbered,
            BlockKind::Description,
        ] {
            draw(&mut app, vec![]);
            let output = draw(&mut app, vec![]);
            assert!(point(&output, "• Liste").is_none());
            assert!(point(&output, "1. Liste").is_none());
            let pos = icon_rect(&output, &ctx, crate::icons::Icon::List).center();
            for pressed in [true, false] {
                draw(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            let output = draw(&mut app, vec![]);
            for icon in [
                crate::icons::Icon::ListOrdered,
                crate::icons::Icon::ListDescription,
            ] {
                assert!(icon_rect(&output, &ctx, icon).is_positive());
            }
            for list in [
                BlockKind::Bullet,
                BlockKind::Numbered,
                BlockKind::Description,
            ] {
                assert!(point(&output, &Command::Block(list).label()).is_some());
            }
            let pos = point(&output, &Command::Block(kind).label()).unwrap();
            for pressed in [true, false] {
                draw(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(app.session.document.view.blocks[0], kind);
        }
    }

    #[test]
    fn changing_model_restores_automatic_markers_without_altering_the_body() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx, None);
        app.session.template.push_str("\n// modèle personnalisé");
        let source = app.session.document.source().to_owned();
        app.session.metadata.options_mut().bullet_marker = Some("◆".into());
        app.presentation.custom_bullet_marker = true;
        app.apply(Command::Model(Model::Personal));
        assert!(app.session.metadata.options().bullet_marker.is_none());
        assert_eq!(app.presentation, ModelPresentation::default());
        assert_eq!(app.session.document.source(), source);
    }

    #[test]
    fn document_information_offers_only_the_markers_allowed_by_the_template() {
        let ctx = egui::Context::default();
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut app = App::with_demo_context(ctx.clone(), None);
        let mut world = crate::rendering::TypstWorld::default();
        let draw = |app: &mut App, events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.bullet_marker_ui(ui));
                },
            )
        };
        fn point(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
        }
        let output = draw(&mut app, vec![]);
        assert!(point(&output, "Automatique (modèle)").is_some());
        assert!(point(&output, "▪").is_none());
        assert!(point(&output, "Caractère personnalisé…").is_none());
        app.presentation = world
            .model_presentation("#let editor-options = (bullet-markers: (\"▪\",),)")
            .unwrap();
        for label in ["Automatique (modèle)", "▪"] {
            draw(&mut app, vec![]);
            let output = draw(&mut app, vec![]);
            let pos = point(&output, label).unwrap();
            for pressed in [true, false] {
                draw(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        assert_eq!(
            app.session.metadata.options().bullet_marker.as_deref(),
            Some("▪")
        );
        let output = draw(&mut app, vec![]);
        assert!(point(&output, "Caractère personnalisé…").is_none());
    }
    #[test]
    fn model_dependencies_follow_imports_choices_and_undo_without_changing_the_body() {
        let mut app = App::with_demo_context(egui::Context::default(), None);
        let original = app.session.dependencies.clone();
        assert!(!original.is_empty());
        let body = app.session.document.source().to_owned();
        app.install_model(app.session.template.clone() + "\n// Modèle dérivé")
            .unwrap();
        assert!(app.session.dependencies == original);
        app.install_model("#let courrier(..args) = args.pos().last()".into())
            .unwrap();
        assert!(app.session.dependencies.is_empty());
        assert!(app.request(None).dependencies.is_empty());
        app.apply(Command::Undo);
        assert!(app.session.dependencies == original);
        assert!(app.stored_document().dependencies == original);
        app.apply(Command::Redo);
        assert!(app.session.dependencies.is_empty());
        app.apply(Command::Model(Model::Personal));
        assert!(app.session.dependencies == original);
        app.apply(Command::Undo);
        assert!(app.session.dependencies.is_empty());
        assert_eq!(app.session.document.source(), body);
    }
    #[test]
    fn letter_notes_insert_type_scope_editing_exit_and_undo_use_the_shared_session() {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(ctx.clone(), None);
        let body = app.session.document.view.text.clone();
        app.apply(Command::InsertPostscript);
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Merci encore.".into())],
        );
        assert!(
            app.session
                .document
                .source()
                .contains("#postscript[Merci encore.]")
        );
        app.apply(Command::InsertAttachment);
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Formulaire signé".into())],
        );
        let key = |key| egui::Event::Key {
            key,
            pressed: true,
            repeat: false,
            physical_key: None,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Rapport annuel".into())],
        );
        assert!(
            app.session
                .document
                .source()
                .contains("#attachment[Formulaire signé]")
        );
        assert!(
            app.session
                .document
                .source()
                .contains("#attachment[Rapport annuel]")
        );
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        let line = app.session.document.view.line_at(app.range().start);
        assert_eq!(app.session.document.view.blocks[line], BlockKind::Paragraph);
        app.apply(Command::Undo);
        assert_eq!(
            app.session.document.view.blocks[line],
            BlockKind::Attachment
        );
        app.apply(Command::Redo);
        assert_eq!(app.session.document.view.blocks[line], BlockKind::Paragraph);
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("courrier.typsto");
        assert!(app.save_path(archive.clone()));
        let stored = crate::storage::read(&archive).unwrap();
        assert_eq!(stored.source, app.session.document.source());
        assert!(app.session.document.view.text.starts_with(&body));
        assert!(
            stored
                .dependencies
                .contains_key("capabilities/correspondence.typ")
        );
        let registry = crate::commands::Registry::new(DocumentType::CompteRendu);
        assert!(!registry.contains(Command::InsertPostscript));
        assert!(!registry.contains(Command::InsertAttachment));
    }
}
