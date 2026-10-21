mod chrome;
mod commands;
mod editor_view;
mod files;
mod images;
mod information;
mod links;
mod models;
mod preferences;
mod preview;
mod recovery;
mod search;
mod signature;
mod spelling;
mod text_input;
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
    #[cfg(test)]
    fn with_demo_context(ctx: egui::Context, capture: Option<PathBuf>) -> Self {
        let mut app = Self::with_context(ctx, capture);
        app.load_demo();
        app
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
    /// Une seule voie pour rendre visible et focaliser une cible d’annotation.
    /// Diagnostic de présentation utilisant les mêmes commandes que l’interface.
    pub fn capture_creation(&mut self, ctx: &egui::Context) {
        self.perform_action(PendingAction::New, ctx);
    }
    /// La proportion appartient aux préférences, les rectangles à la frame.
    /// Seuls les bords gardent une petite zone accessible pour chaque colonne.
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
