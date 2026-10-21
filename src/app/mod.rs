mod images;
mod information;
mod links;
mod models;
mod preferences;
mod search;
mod signature;
mod spelling;
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
    /// Diagnostic de présentation utilisant les mêmes commandes que l’interface.
    pub fn capture_creation(&mut self, ctx: &egui::Context) {
        self.perform_action(PendingAction::New, ctx);
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
mod tests;
