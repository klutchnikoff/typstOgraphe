use crate::{
    annotations::OptionalAnnotation,
    commands::{Command, FileCommand, Keymap, Registry, ShortcutOverrides},
    document::{
        BlockKind, Document, DocumentLanguage, ImageBlock, InlineStyle, Mark, PaperFormat,
        Projection,
    },
    document_types::{
        DocumentType, Metadata, ModelPresentation, courrier::Command as CourrierCommand,
    },
    editor, menus,
    navigation::{PagePoint, RenderSync},
    preferences::{Appearance, EditorFont, Preferences},
    rendering::{RenderRequest, RenderWorker},
    storage::{self, StoredDocument},
};
use eframe::egui::{
    self, Color32, RichText, TextBuffer,
    text::{CCursor, CCursorRange},
};
use std::{
    ops::Range,
    path::PathBuf,
    time::{Duration, Instant},
};

const ACCENT: Color32 = Color32::from_rgb(40, 92, 119);
const PREVIEW_SELECTION: Color32 = Color32::from_rgba_premultiplied(16, 33, 59, 65);
#[derive(Clone)]
struct Snapshot {
    document: Document,
    selection: CCursorRange,
}
#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
    Close,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum InformationGroup {
    #[default]
    General,
    Fields(&'static str),
}

pub struct App {
    preferences: Preferences,
    preferences_open: bool,
    keymap: Keymap,
    #[cfg(target_os = "macos")]
    native_menus: Option<menus::native::NativeMenus>,
    history_field: Option<egui::Id>,
    field_history_action: Option<(Command, egui::Id)>,
    pub document: Document,
    metadata: Metadata,
    template: String,
    presentation: ModelPresentation,
    file_path: Option<PathBuf>,
    saved: StoredDocument,
    file_command: Option<FileCommand>,
    pending_action: Option<PendingAction>,
    file_error: Option<String>,
    allow_close: bool,
    window_title: String,
    selection: CCursorRange,
    force_selection: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: Instant,
    typing_group: bool,
    worker: RenderWorker,
    revision: u64,
    statistics: crate::statistics::Counter,
    submitted: u64,
    changed_at: Instant,
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
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut app = Self::with_context(cc.egui_ctx.clone(), capture);
        app.preferences = Preferences::load(cc.storage);
        cc.egui_ctx.set_theme(app.preferences.appearance.theme());
        if let Some(path) = path {
            app.open_path(path);
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
        let saved = StoredDocument::demo(DocumentType::Courrier);
        let keymap = Keymap::new(
            Registry::new(saved.metadata.document_type()),
            ctx.os().is_mac(),
            &ShortcutOverrides::default(),
        )
        .expect("raccourcis par défaut cohérents");
        let worker = RenderWorker::spawn(ctx);
        let document =
            Document::from_source(&saved.source).expect("document de démonstration valide");
        Self {
            preferences: Preferences::default(),
            preferences_open: false,
            keymap,
            #[cfg(target_os = "macos")]
            native_menus: None,
            history_field: None,
            field_history_action: None,
            document,
            metadata: saved.metadata.clone(),
            template: saved.template.clone(),
            presentation: ModelPresentation::default(),
            file_path: None,
            saved,
            file_command: None,
            pending_action: None,
            file_error: None,
            allow_close: false,
            window_title: String::new(),
            selection: CCursorRange::one(CCursor::new(0)),
            force_selection: false,
            undo: vec![],
            redo: vec![],
            last_edit: Instant::now(),
            typing_group: false,
            worker,
            revision: 1,
            statistics: Default::default(),
            submitted: 0,
            changed_at: Instant::now() - Duration::from_secs(1),
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
        Snapshot {
            document: self.document.clone(),
            selection: self.selection,
        }
    }
    fn stored_document(&self) -> StoredDocument {
        StoredDocument {
            source: self.document.source().into(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            assets: self.document.assets.clone(),
        }
    }
    fn is_dirty(&self) -> bool {
        self.saved.source != self.document.source()
            || self.saved.metadata != self.metadata
            || self.saved.template != self.template
            || self.saved.assets != self.document.assets
    }
    fn modal_open(&self) -> bool {
        self.image_info.is_some()
            || self.preferences_open
            || self.palette_open
            || self.link_open
            || self.export_open
            || self.pending_action.is_some()
            || self.file_error.is_some()
    }
    fn save_path(&mut self, path: PathBuf) -> bool {
        let letter = self.stored_document();
        match storage::save(&path, &letter) {
            Ok(()) => {
                self.message = Some(crate::tr!(
                    "letter-saved",
                    path = path.display().to_string()
                ));
                self.file_path = Some(path);
                // Le fichier est nettoyé par storage::save ; ce repère en mémoire
                // garde les mêmes ressources que l’historique pour is_dirty.
                self.saved = letter;
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
        if !save_as && let Some(path) = self.file_path.clone() {
            return self.save_path(path);
        }
        let mut dialog = rfd::FileDialog::new()
            .set_title(crate::tr!("save-dialog"))
            .add_filter(crate::tr!("letter-filter"), &[storage::EXTENSION])
            .set_file_name(
                self.file_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| crate::tr!("default-filename")),
            );
        if let Some(parent) = self.file_path.as_ref().and_then(|path| path.parent()) {
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
        self.keymap = keymap;
        self.history_field = None;
        self.field_history_action = None;
        self.document = document;
        self.document.assets = letter.assets.clone();
        self.image_textures.clear();
        self.image_info = None;
        self.information_group = InformationGroup::General;
        self.metadata = letter.metadata.clone();
        self.template = letter.template.clone();
        self.presentation = ModelPresentation::default();
        self.saved = letter;
        self.file_path = path;
        self.undo.clear();
        self.redo.clear();
        self.typing_group = false;
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
        self.revision += 1;
        self.changed_at = Instant::now() - Duration::from_secs(1);
        Ok(())
    }
    fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        if self.is_dirty() {
            self.pending_action = Some(action);
        } else {
            self.perform_action(action, ctx);
        }
    }
    fn perform_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        match action {
            PendingAction::New => {
                let letter = StoredDocument::blank(self.metadata.document_type());
                let document = Document::from_source(&letter.source).expect("courrier vide valide");
                if let Err(error) = self.install_document(letter, document, None) {
                    self.file_error = Some(error);
                }
            }
            PendingAction::Open => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(crate::tr!("open-dialog"))
                    .add_filter(crate::tr!("letter-filter"), &[storage::EXTENSION])
                    .add_filter(crate::tr!("all-files"), &["*"]);
                if let Some(parent) = self.file_path.as_ref().and_then(|path| path.parent()) {
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
        }
        ctx.request_repaint();
    }
    fn handle_file_command(&mut self, command: FileCommand, ctx: &egui::Context) {
        match command {
            FileCommand::New => self.request_action(PendingAction::New, ctx),
            FileCommand::Open => self.request_action(PendingAction::Open, ctx),
            FileCommand::Close | FileCommand::Quit => {
                self.request_action(PendingAction::Close, ctx)
            }
            FileCommand::Save => {
                self.save_document(false);
            }
            FileCommand::SaveAs => {
                self.save_document(true);
            }
            FileCommand::Export => self.export_open = true,
        }
        ctx.request_repaint();
    }
    fn range(&self) -> Range<usize> {
        let range = self.selection.as_sorted_char_range();
        range.start.min(self.document.view.len())..range.end.min(self.document.view.len())
    }
    fn changed(&mut self) {
        let inserted = self.document.sync_source();
        for at in &inserted {
            for cursor in [&mut self.selection.primary, &mut self.selection.secondary] {
                if cursor.index > *at {
                    cursor.index += 1;
                }
            }
        }
        for cursor in [&mut self.selection.primary, &mut self.selection.secondary] {
            cursor.index = cursor.index.min(self.document.view.len());
        }
        self.force_selection |= !inserted.is_empty();
        self.revision += 1;
        self.changed_at = Instant::now();
    }
    fn remember(&mut self, before: Snapshot, typing: bool) {
        if !typing || !self.typing_group || self.last_edit.elapsed() > Duration::from_millis(650) {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.typing_group = typing;
        self.last_edit = Instant::now();
        self.changed();
    }
    fn apply(&mut self, command: Command) {
        if !self.keymap.registry().contains(command) {
            return;
        }
        match command {
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
                    self.field_history_action = None;
                    self.force_selection = true;
                }
            }
            Command::Courrier(CourrierCommand::Model(model)) => {
                let template = model.source();
                if self.template != template {
                    self.template = template;
                    self.presentation = ModelPresentation::default();
                    self.metadata.options_mut().bullet_marker = None;
                    self.revision += 1;
                    self.changed_at = Instant::now();
                }
            }
            Command::Undo | Command::Redo => self.apply_history(command == Command::Undo),
            Command::Link => self.open_link(),
            Command::RemoveLink => {
                let range = self.range();
                if range.is_empty() {
                    self.message = Some(crate::tr!("select-link-remove"));
                } else if self.document.view.styles[range]
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
        let current = self.snapshot();
        let item = if undo {
            self.undo.pop()
        } else {
            self.redo.pop()
        };
        if let Some(item) = item {
            if undo {
                self.redo.push(current);
            } else {
                self.undo.push(current);
            }
            self.document = item.document;
            self.image_textures.clear();
            self.selection = item.selection;
            self.force_selection = true;
            self.typing_group = false;
            self.revision += 1;
            self.changed_at = Instant::now();
        }
    }
    fn edit(&mut self, edit: impl FnOnce(&mut Projection, Range<usize>)) {
        let range = self.range();
        let before = self.snapshot();
        edit(&mut self.document.view, range);
        let selected = self.range();
        if !selected.is_empty() {
            self.document.view.typing_style = self.document.view.style_at(selected.start);
        }
        self.remember(before, false);
        self.force_selection = true;
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
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let extension = if extension == "jpeg" {
            "jpg"
        } else {
            extension.as_str()
        };
        // Lire au plus la limite, même si le fichier a grandi depuis sa sélection.
        use std::io::Read;
        let mut data = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(crate::assets::MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        crate::assets::decode(&data, extension)?;
        let total: usize = self.document.assets.values().map(|v| v.len()).sum();
        let needs_size_check = total + data.len() > crate::assets::MAX_DOCUMENT_BYTES as usize;
        let before = self.snapshot();
        let mut number = 1;
        let asset_path = loop {
            let name = format!("images/image-{number}.{extension}");
            if !self.document.assets.contains_key(&name) {
                break name;
            }
            number += 1;
        };
        self.document.assets.insert(asset_path.clone(), data.into());
        let at = if let Some(id) = replace {
            let Some(image) = self.document.view.images.get_mut(&id) else {
                self.document = before.document;
                return Ok(());
            };
            image.path = asset_path;
            self.document.view.image_range(id).unwrap().start
        } else {
            let range = self.range();
            self.document.view.delete_char_range(range.clone());
            self.document.view.insert_image(
                range.start,
                ImageBlock {
                    path: asset_path,
                    alt: String::new(),
                },
            )
        };
        if needs_size_check {
            let mut persisted = self.document.clone();
            persisted.sync_source();
            let retained = crate::rendering::retained_assets(
                persisted.source(),
                &self.metadata,
                &self.template,
                &persisted.assets,
            );
            let too_large = retained.as_ref().is_ok_and(|assets| {
                assets.values().map(|data| data.len()).sum::<usize>()
                    > crate::assets::MAX_DOCUMENT_BYTES as usize
            });
            if retained.is_err() || too_large {
                self.document = before.document;
                return Err(retained
                    .err()
                    .unwrap_or_else(|| crate::tr!("storage-letter-too-large")));
            }
        }
        self.focus_annotation_target(at + 1..at + 1);
        self.remember(before, false);
        Ok(())
    }
    fn remove_image(&mut self, id: u64) {
        if let Some(range) = self.document.view.image_range(id) {
            let before = self.snapshot();
            let end = (range.end + 1).min(self.document.view.len());
            self.document.view.delete_char_range(range.start..end);
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
        if self.document.view.selected_lines(range.clone()).len() > 1 {
            self.message = Some(crate::tr!("link-one-paragraph"));
            return;
        }
        self.link_url = self
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
            revision: self.revision,
            source: self.document.source().to_owned(),
            source_map: self.document.source_map.clone(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            assets: self.document.assets.clone(),
            export_dir,
        }
    }
    fn rendering(&mut self, ctx: &egui::Context) {
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
                    if response.revision == self.revision {
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
                    if response.revision == self.revision || response.is_export {
                        self.rendering_error = Some(error);
                        self.rendered_revision = Some(response.revision);
                    }
                }
            }
        }
        if self.submitted != self.revision && self.changed_at.elapsed() > Duration::from_millis(220)
        {
            match self.worker.sender.send(self.request(None)) {
                Ok(()) => self.submitted = self.revision,
                Err(_) => self.rendering_error = Some(crate::tr!("renderer-stopped")),
            }
        }
        if self.rendered_revision != Some(self.revision) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.modal_open() {
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
        if focused && !self.modal_open() {
            self.text_shortcuts(ctx);
        }
    }
    fn track_history_focus(&mut self, ctx: &egui::Context) {
        // La recherche de la palette et les champs des dialogues ne doivent pas
        // remplacer la cible de l’historique du document.
        if self.modal_open() || self.field_history_action.is_some() {
            return;
        }
        if let Some(id) = ctx.memory(|memory| memory.focused()) {
            if id == egui::Id::new(editor::EDITOR_ID) {
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
                    && match d.command {
                        Command::Undo => self.history_field.is_some() || !self.undo.is_empty(),
                        Command::Redo => self.history_field.is_some() || !self.redo.is_empty(),
                        Command::Clear => self.history_field.is_none(),
                        _ => true,
                    };
                (
                    d.command,
                    menus::Status {
                        enabled,
                        checked: d.command == Command::Information && self.information_open,
                    },
                )
            })
            .collect()
    }
    fn dispatch_action(&mut self, command: Command, ctx: &egui::Context) {
        if self.modal_open() {
            return;
        }
        // Le menu Édition agit sur le champ actif. Le corps possède son propre
        // historique ; les champs de métadonnées utilisent celui de TextEdit.
        if let (Command::Undo | Command::Redo, Some(id)) = (command, self.history_field) {
            self.field_history_action = Some((command, id));
        } else {
            self.apply(command);
        }
        ctx.request_repaint();
    }
    fn field_history(&mut self, ctx: &egui::Context) {
        // Différer jusqu’à une frame où les champs peuvent consommer l’événement,
        // y compris si la commande a été choisie dans la palette en fin de frame.
        if let Some((command, id)) = self.field_history_action.take() {
            if self.modal_open() || !self.information_open {
                return;
            }
            ctx.memory_mut(|memory| memory.request_focus(id));
            let modifiers = egui::Modifiers {
                shift: command == Command::Redo,
                ..egui::Modifiers::COMMAND
            };
            ctx.input_mut(|input| {
                input.events.push(egui::Event::Key {
                    key: egui::Key::Z,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                })
            });
        }
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
        self.typing_group = false;
    }
    fn leave_annotation(&mut self, annotation: &OptionalAnnotation) {
        let before = self.snapshot();
        let at = self.document.view.leave_annotation(annotation);
        self.remember(before, false);
        self.focus_annotation_target(at..at);
    }
    fn annotation_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        let range = self.range();
        let Some(annotation) = self.document.view.annotation_at(range.clone()) else {
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
        let line = self.document.view.line_at(range.start);
        let lines = self.document.view.line_ranges();
        let start = lines[line].start;
        let prefix: String = self
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
        if let BlockKind::Image(id) = self.document.view.blocks[line] {
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
                before.selection = editor::cursor(range.start + 1..range.start + 1);
                self.document.view.delete_char_range(start..range.start);
                self.document.view.set_block(start..start, kind);
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
        if self.document.view.blocks[line] == BlockKind::Quote
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            let before = self.snapshot();
            if lines[line].is_empty() && range.is_empty() {
                let group = self
                    .document
                    .view
                    .quote_groups()
                    .into_iter()
                    .find(|group| group.body.contains(&line))
                    .expect("paragraphe de citation");
                // La terminaison du corps cité reste une règle propre à la citation.
                // Retirer sa ligne vide, puis utiliser la sortie d’annotation commune.
                if group.body.len() > 1 {
                    self.document.view.delete_char_range(start..start + 1);
                    self.document.view.blocks
                        [group.attribution.expect("attribution éditable") - 1] =
                        BlockKind::QuoteAttribution;
                }
                let annotation = self
                    .document
                    .view
                    .annotation_at(start..start)
                    .expect("annotation de citation");
                let at = self.document.view.leave_annotation(&annotation);
                self.remember(before, false);
                self.focus_annotation_target(at..at);
            } else {
                self.document.view.delete_char_range(range.clone());
                self.document.view.insert_text("\n", range.start);
                self.selection = editor::cursor(range.start + 1..range.start + 1);
                self.remember(before, false);
                self.force_selection = true;
            }
            return;
        }
        if self.document.view.blocks[line] == BlockKind::Description {
            let text = &self.document.view.text[self.document.view.byte_index(lines[line].start)
                ..self.document.view.byte_index(lines[line].end)];
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
                    self.document.view.insert_text("\t", range.start);
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
                before.selection = editor::cursor(range.start + 1..range.start + 1);
                self.document
                    .view
                    .delete_char_range(range.start - 1..range.start);
                if separator.is_none() {
                    self.document.view.insert_text("\t", range.start - 1);
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
                    self.document.view.delete_char_range(lines[line].clone());
                    self.document
                        .view
                        .set_block(start..start, BlockKind::Paragraph);
                    self.selection = editor::cursor(start..start);
                } else {
                    let in_text = separator.is_some_and(|at| range.start > at);
                    self.document.view.delete_char_range(range.clone());
                    self.document
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
                before.selection = editor::cursor(range.start + 1..range.start + 1);
                self.document
                    .view
                    .delete_char_range(start + position..start + position + 1);
                let mark = if delimiter == '*' {
                    Mark::Bold
                } else {
                    Mark::Italic
                };
                let selected = start + position..range.start - 1;
                if !self.document.view.mark_state(selected.clone(), mark).0 {
                    self.document.view.toggle_mark(selected, mark);
                }
                self.document.view.typing_style = InlineStyle::default();
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
                self.document.view.blocks[line],
                BlockKind::Bullet | BlockKind::Numbered | BlockKind::Description
            )
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.apply(Command::Block(BlockKind::Paragraph));
        }
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        if let Some(command) =
            crate::toolbar::show(ui, &self.document.view, self.range(), &self.keymap)
        {
            self.apply(command);
        }
    }
    fn status_ui(&mut self, ui: &mut egui::Ui) {
        let range = self.range();
        let (total, selected) = self
            .statistics
            .counts(self.revision, &self.document.view, range);
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
            if let Some(message) = &self.message {
                ui.separator();
                ui.add(egui::Label::new(message).truncate())
                    .on_hover_text(message);
            }
        });
    }
    fn information_ui(&mut self, ui: &mut egui::Ui) {
        let old = self.metadata.clone();
        let groups: Vec<_> =
            std::iter::once((InformationGroup::General, crate::tr!("group-general")))
                .chain(self.metadata.editor_groups().iter().map(|group| {
                    (
                        InformationGroup::Fields(group.id),
                        crate::i18n::text(group.label_key),
                    )
                }))
                .collect();
        let full_heading = crate::tr!("document-settings-heading");
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
            self.field_history_action = None;
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
                    InformationGroup::Fields(group) => self.metadata.editor_group_ui(ui, group),
                });
        }
        if self.metadata != old {
            self.revision += 1;
            self.changed_at = Instant::now();
            ui.ctx().request_repaint();
        }
    }
    fn general_information_ui(&mut self, ui: &mut egui::Ui) {
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
                egui::ComboBox::from_id_salt("document-paper")
                    .selected_text(self.metadata.options().paper.label())
                    .show_ui(ui, |ui| {
                        for paper in PaperFormat::ALL {
                            ui.selectable_value(
                                &mut self.metadata.options_mut().paper,
                                paper,
                                paper.label(),
                            );
                        }
                    });
                ui.end_row();
                ui.label(crate::tr!("field-orientation"));
                egui::ComboBox::from_id_salt("document-orientation")
                    .selected_text(if self.metadata.options().landscape {
                        crate::tr!("orientation-landscape")
                    } else {
                        crate::tr!("orientation-portrait")
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.metadata.options_mut().landscape,
                            false,
                            crate::tr!("orientation-portrait"),
                        );
                        ui.selectable_value(
                            &mut self.metadata.options_mut().landscape,
                            true,
                            crate::tr!("orientation-landscape"),
                        );
                    });
                ui.end_row();
                ui.label(crate::tr!("field-document-language"));
                egui::ComboBox::from_id_salt("document-language")
                    .selected_text(self.metadata.options().language.label())
                    .show_ui(ui, |ui| {
                        for language in DocumentLanguage::ALL {
                            ui.selectable_value(
                                &mut self.metadata.options_mut().language,
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
            if self.metadata.options().bullet_marker.is_some() {
                if ui
                    .button(crate::tr!("bullet-marker-restore-automatic"))
                    .clicked()
                {
                    self.metadata.options_mut().bullet_marker = None;
                }
            } else {
                ui.weak(crate::tr!("bullet-marker-automatic"))
                    .on_hover_text(crate::tr!("bullet-marker-model-controlled"));
            }
            return;
        }
        let marker = &mut self.metadata.options_mut().bullet_marker;
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
        let current = commands
            .iter()
            .copied()
            .find(|command| command.model_source().as_deref() == Some(self.template.as_str()));
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
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        self.toolbar(ui);
        ui.separator();
        self.information_ui(ui);
        ui.separator();
        let before = self.snapshot();
        let old_range = self.selection;
        if self.force_selection {
            editor::restore_cursor(ui.ctx(), self.selection);
            self.force_selection = false;
        }
        for image in self.document.view.images.values() {
            if !self.image_textures.contains_key(&image.path)
                && let Some(data) = self.document.assets.get(&image.path)
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
        let view = self.document.view.clone();
        let lock_tab = view.annotation_at(self.range()).is_some()
            || view.blocks[view.line_at(self.selection.primary.index)] == BlockKind::Description;
        let editor_style = self.preferences.editor;
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
            editor::layout(ui, &view, text.as_str(), width, editor_style)
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
                        let output = egui::TextEdit::multiline(&mut self.document.view)
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
                            &self.document.view,
                            &output,
                            editor_style,
                            background,
                        );
                        if let Some(range) = output.cursor_range {
                            self.selection = range;
                        }
                        if let Some(at) = clicked_term {
                            self.selection = editor::cursor(at..at);
                            self.force_selection = true;
                        }
                        if self.editor_scroll_pending {
                            output.response.request_focus();
                            let cursor = output.galley.pos_from_cursor(self.selection.primary);
                            ui.scroll_to_rect(
                                cursor.translate(output.galley_pos.to_vec2()),
                                Some(egui::Align::Center),
                            );
                            self.editor_scroll_pending = false;
                        }
                        image_action = editor::paint_images(
                            ui,
                            &self.document.view,
                            &output,
                            &self.image_textures,
                            self.range(),
                            editor_style,
                        );
                        let changed = output.response.changed();
                        let mut state = output.state;
                        state.clear_undoer();
                        state.store(ui.ctx(), output.response.id);
                        if changed {
                            self.remember(before.clone(), true);
                            ui.ctx().request_repaint();
                        } else if self.selection != old_range {
                            let selected = self.range();
                            let pos = self.selection.primary.index;
                            self.document.view.typing_style = if !selected.is_empty() {
                                self.document.view.style_at(selected.start)
                            } else if pos > 0 {
                                self.document.view.style_at(pos - 1)
                            } else {
                                InlineStyle::default()
                            };
                            self.typing_group = false;
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
                        .clone_from(&self.document.view.images[&id].alt);
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
            if self.rendered_revision != Some(self.revision) {
                ui.spinner();
                ui.weak(crate::tr!("preview-updating"));
            } else if let Some(time) = self.rendering_time {
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
                .is_some_and(|sync| sync.revision == self.revision)
    }
    fn navigate_backward(&mut self, location: PagePoint, ctx: &egui::Context) {
        if !self.sync_is_current() {
            return;
        }
        if let Some(character) = self.sync.as_ref().and_then(|sync| sync.backward(location)) {
            self.selection =
                CCursorRange::one(CCursor::new(character.min(self.document.view.len())));
            self.document.view.typing_style = if character > 0 {
                self.document.view.style_at(character - 1)
            } else {
                InlineStyle::default()
            };
            self.force_selection = true;
            self.editor_scroll_pending = true;
            self.forward_pending = false;
            self.typing_group = false;
            ctx.request_repaint();
        }
    }
    fn preferences_ui(&mut self, ctx: &egui::Context) {
        if !self.preferences_open {
            return;
        }
        let response = egui::Modal::new(egui::Id::new("preferences")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading(crate::tr!("preferences-title"));
            ui.add_space(8.0);
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
                        egui::Slider::new(&mut self.preferences.editor.size, 10.0..=36.0)
                            .step_by(1.0),
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
                if let Some(image) = self.document.view.images.get_mut(&id) {
                    image.alt.clone_from(&self.image_alt);
                    self.remember(before, false);
                }
                open = false;
                self.force_selection = true;
                self.editor_scroll_pending = true;
            }
            if !open {
                self.image_info = None;
                if let Some(range) = self.document.view.image_range(id) {
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
                    self.document
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
                        .filter(|definition| definition.in_palette())
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
                if !self.metadata.ready_for_export() {
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
        if ctx.input(|input| input.viewport().close_requested())
            && !self.allow_close
            && self.is_dirty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.preferences_open = false;
            self.palette_open = false;
            self.link_open = false;
            self.export_open = false;
            self.information_open = false;
            self.pending_action = Some(PendingAction::Close);
        }
        self.rendering(ctx);
        if !self.modal_open() {
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
        self.field_history(ctx);
        let modal = self.modal_open();
        let dirty = self.is_dirty();
        let filename = self
            .file_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| crate::tr!("untitled-letter"));
        let title = crate::tr!(
            "window-title",
            name = filename.as_str(),
            marker = if dirty { " *" } else { "" }
        );
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
            self.status_ui(ui);
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.columns_ui(ui, modal);
            });
        self.preferences_ui(ctx);
        self.dialogs(ctx);
        if let Some(command) = self.file_command.take() {
            self.handle_file_command(command, ctx);
        }
        self.file_dialogs(ctx);
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
        if self.rendered_revision == Some(self.revision)
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.preferences.save(storage);
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_frame(ctx);
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::document_types::courrier::Model;

    #[test]
    fn native_enter_distinguishes_soft_newlines_and_paragraphs_with_undo_and_reopening() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.document = Document::from_source("Alpha").unwrap();
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
        assert_eq!(app.document.source(), "Alpha\n");
        frame(&mut app, &ctx, vec![egui::Event::Text("Beta".into())]);
        assert_eq!(app.document.source(), "Alpha\nBeta");
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(app.document.view.text, "Alpha\nBeta\n\n");
        frame(&mut app, &ctx, vec![egui::Event::Text("Gamma".into())]);
        let paragraphs = app.document.clone();
        app.selection = editor::cursor(12..12);
        app.force_selection = true;
        app.typing_group = false;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Backspace)]);
        assert_eq!(app.document.source(), "Alpha\nBeta\nGamma");
        app.apply(Command::Undo);
        assert_eq!(app.document, paragraphs);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paragraphes.typsto");
        assert!(app.save_path(path.clone()));
        assert!(app.open_path(path));
        assert_eq!(app.document.view, paragraphs.view);
        assert_eq!(app.document.source(), paragraphs.source());
        assert!(!app.document.source().contains('¶'));
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
        let mut app = App::with_context(ctx.clone(), None);
        ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);
        app.document = Document::from_source("#strong[e\u{301}] 👩🏽‍💻").unwrap();
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
        app.perform_action(PendingAction::New, &ctx);
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
            let mut app = App::with_context(ctx.clone(), None);
            // Vérifier la géométrie réellement peinte, indépendamment du clignotement.
            ctx.all_styles_mut(|style| style.visuals.text_cursor.blink = false);
            app.document = Document::from_source(source).unwrap();
            app.document.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../tests/fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
            let annotation = app.document.view.annotations().remove(0);
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
                    app.document.source(),
                    source,
                    "le texte indicatif reste virtuel"
                );
            }
            draw(&mut app, vec![egui::Event::Text("Une annotation.".into())]);
            assert!(app.document.source().contains("[Une annotation.]"));
            draw(&mut app, vec![tab()]);
            assert_eq!(app.range(), at..at);
            draw(&mut app, vec![egui::Event::Text("Été : ".into())]);
            assert!(app.document.view.text.contains("Été : Une annotation."));
        }
    }

    #[test]
    fn annotation_navigation_and_exit_preserve_content_selection_and_undo() {
        for source in [
            "#figure(image(\"images/parc.svg\"), caption: [Été *à Paris*.])\n\nSuite.",
            "#quote(block: true, attribution: [Été *à Paris*.])[Une pensée.]\n\nSuite.",
        ] {
            let ctx = egui::Context::default();
            let mut app = App::with_context(ctx.clone(), None);
            app.document = Document::from_source(source).unwrap();
            let annotation = app.document.view.annotations().remove(0);
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
                app.undo.is_empty(),
                "la navigation ne modifie pas le document"
            );
            let original = app.document.clone();
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
                app.document.view.blocks[app.document.view.line_at(app.range().start)],
                BlockKind::Paragraph
            );
            assert_eq!(app.document.view.annotations()[0], annotation);
            assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
            frame(&mut app, &ctx, vec![egui::Event::Text("Après.".into())]);
            assert!(app.document.view.text.contains("Été à Paris.\nAprès."));
            app.apply(Command::Undo);
            app.apply(Command::Undo);
            assert_eq!(app.document, original);
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
            let mut app = App::with_context(ctx.clone(), None);
            app.document = Document::from_source(&source).unwrap();
            let annotation = app.document.view.annotations().remove(0);
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
            assert_eq!(app.document.source(), source);
        }
    }

    #[test]
    fn discarded_images_in_memory_do_not_exhaust_the_document_size_limit() {
        let mut app = App::with_context(egui::Context::default(), None);
        app.document = Document::from_source("").unwrap();
        // Simuler plus de 64 Mio d’anciennes ressources partagées en mémoire.
        // Elles ne sont plus référencées par le document ni par son modèle.
        let unused: std::sync::Arc<[u8]> = vec![0; 1024 * 1024].into();
        for index in 0..65 {
            app.document
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
        let mut app = App::with_context(ctx, None);
        app.document = Document::from_source("").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.svg");
        std::fs::write(&original, include_bytes!("../tests/fixtures/parc.svg")).unwrap();
        app.import_image(&original, None).unwrap();
        let first = app.document.view.images[&1].path.clone();
        let path = dir.path().join("Courrier.typsto");
        assert!(app.save_path(path.clone()));
        app.import_image(&original, Some(1)).unwrap();
        let replacement = app.document.view.images[&1].path.clone();
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
        assert!(app.document.assets.contains_key(&first));
        app.apply(Command::Undo);
        assert_eq!(app.document.view.images[&1].path, first);
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
        assert_eq!(app.document.view.images[&1].path, replacement);
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
        assert_eq!(app.document.view.images[&1].path, replacement);
        assert!(app.is_dirty());
        assert!(app.document.assets.contains_key(&replacement));
        assert!(app.save_path(path.clone()));
        assert_eq!(storage::read(&path).unwrap().assets.len(), 1);
        app.apply(Command::Redo);
        assert!(app.document.view.images.is_empty());
        assert!(app.save_path(path.clone()));
        assert!(storage::read(&path).unwrap().assets.is_empty());
        assert!(!app.is_dirty());
    }

    #[test]
    fn image_remove_button_deletes_the_selected_block_and_caption_with_undo() {
        for caption in ["", ", caption: [Le parc.]"] {
            let ctx = egui::Context::default();
            let mut app = App::with_context(ctx.clone(), None);
            app.document = Document::from_source(&format!(
                "Avant.\n\n#figure(image(\"images/parc.svg\"){caption})\n\nAprès."
            ))
            .unwrap();
            app.document.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../tests/fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
            let original = app.document.clone();
            let at = app.document.view.image_range(1).unwrap().start;
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
                app.document.view.images.is_empty(),
                "le clic doit supprimer l’image entière"
            );
            assert!(!app.document.view.text.contains("Le parc."));
            assert!(app.document.view.text.contains("Avant."));
            assert!(app.document.view.text.contains("Après."));
            app.apply(Command::Undo);
            assert_eq!(app.document, original);
        }
    }

    #[test]
    fn image_controls_edit_alt_and_keyboard_deletes_the_whole_block_with_undo() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.document = Document::from_source("Avant.\n\nAprès.").unwrap();
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
        assert!(app.document.source().contains("Des arbres et un chemin"));
        frame(&mut app, &ctx, vec![egui::Event::Text("Le parc.".into())]);
        assert!(
            app.document.source().contains("caption: [Le parc.]"),
            "{} / {:?}",
            app.document.source(),
            app.range()
        );
        let before = app.document.source().to_owned();
        let at = app.document.view.image_range(1).unwrap().start + 1;
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
        assert!(app.document.view.images.is_empty());
        assert!(!app.document.view.text.contains("Le parc."));
        app.apply(Command::Undo);
        assert_eq!(app.document.source(), before);
    }

    #[test]
    fn images_insert_edit_caption_replace_undo_and_reopen_without_original_file() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.document = Document::from_source("Avant.\n\nAprès.").unwrap();
        app.selection = editor::cursor(6..6);
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("Parc.svg");
        std::fs::write(&original, include_bytes!("../tests/fixtures/parc.svg")).unwrap();
        app.import_image(&original, None).unwrap();
        assert_eq!(app.document.view.images.len(), 1);
        let id = *app.document.view.images.keys().next().unwrap();
        let object = app.document.view.image_range(id).unwrap().start;
        assert_eq!(app.range(), object + 1..object + 1);
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Le parc municipal.".into())],
        );
        assert!(
            app.document
                .source()
                .contains("caption: [Le parc municipal.]"),
            "{}",
            app.document.source()
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
        app.document.view.images.get_mut(&id).unwrap().alt =
            "Trois arbres autour d’un chemin".into();
        app.remember(before_alt, false);
        app.import_image(&original, Some(id)).unwrap();
        assert_eq!(
            app.document.view.images[&id].alt,
            "Trois arbres autour d’un chemin"
        );
        assert!(app.document.source().contains("Le parc municipal."));
        app.apply(Command::Undo);
        assert_eq!(app.document.view.images[&id].path, "images/image-1.svg");
        app.apply(Command::Redo);
        assert_eq!(app.document.view.images[&id].path, "images/image-2.svg");
        let archive = dir.path().join("Courrier.typsto");
        assert!(app.save_path(archive.clone()));
        std::fs::remove_file(&original).unwrap();
        let saved_source = app.document.source().to_owned();
        app.remove_image(id);
        assert!(app.document.view.images.is_empty());
        app.apply(Command::Undo);
        assert_eq!(app.document.source(), saved_source);
        assert!(app.open_path(archive));
        assert!(!app.is_dirty());
        assert_eq!(app.document.source(), saved_source);
        assert_eq!(app.document.assets.len(), 1);
        app.selection = editor::cursor(object + 1..object + 1);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(
            app.document.view.blocks[app.document.view.line_at(app.range().start)],
            BlockKind::Paragraph
        );
        assert!(app.document.source().contains("Le parc municipal."));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Suite du courrier.".into())],
        );
        assert!(app.document.source().contains("Suite du courrier."));
    }

    #[test]
    fn column_drag_and_window_resize_keep_the_chosen_proportion_without_a_preview_cap() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
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
        let mut app = App::with_context(ctx.clone(), None);
        let letter = app.stored_document();
        let revision = app.revision;
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
        assert_eq!(app.revision, revision);
        assert!(!app.is_dirty());
    }

    #[test]
    fn general_settings_and_template_selection_preserve_letter_content() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.information_open = true;
        ctx.style_mut(|s| s.animation_time = 0.0);
        let source = app.document.source().to_owned();
        let revision = app.revision;
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
            "Courrier institutionnel",
            "Courrier de service",
        );
        assert_eq!(app.metadata.options_mut().paper, PaperFormat::A3);
        assert!(app.metadata.options_mut().landscape);
        assert_eq!(
            app.metadata.options_mut().language,
            DocumentLanguage::English
        );
        assert_eq!(app.template, Model::Service.source());
        assert!(app.is_dirty());
        assert_eq!(app.revision, revision + 4);
        assert_eq!(app.document.source(), source);

        assert!(!app.window_title.contains("typstOgraphe"));
    }

    #[test]
    fn egui_file_menu_exports_and_quit_preserves_unsaved_work_without_header_duplicates() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.file_path = Some(PathBuf::from("exemple.typsto"));
        let source = app.document.source().to_owned();
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
        let pos = text_point(&output, "Exporter le courrier…").expect("export dans Fichier");
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
        assert_eq!(app.document.source(), source);
        // Une commande native déjà en transit ne contourne pas un dialogue.
        app.dispatch_action(Command::File(FileCommand::Quit), &ctx);
        assert!(app.file_command.is_none());
        app.export_open = false;
        app.metadata.courrier_mut().objet = "Brouillon à conserver".into();
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
        let mut app = App::with_context(ctx.clone(), None);
        ctx.style_mut(|style| style.animation_time = 0.0);
        app.selection = editor::cursor(10..20);
        app.force_selection = true;
        let source = app.document.source().to_owned();
        let selection = app.selection;
        let revision = app.revision;
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
        for label in ["Correspondance", "Expéditeur"] {
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
        assert_eq!(app.revision, revision);
        assert_eq!(text_rect(&opened, "Aperçu Typst"), preview);
        assert!(has_text(&opened, "Format du papier"));
        assert!(!has_text(&opened, "Destinataire"));
        assert!(!has_text(&opened, "Nom complet"));
        // Même à la taille minimale de la fenêtre, le formulaire reste dans la
        // colonne de gauche et laisse une zone de rédaction utilisable.
        let body = text_rect(&opened, &app.document.view.text);
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
        assert!(!has_text(&sender, "Destinataire"));
        let sender_button = text_rect(&sender, "Expéditeur");
        click(&mut app, &mut draw, sender_button.center());
        let collapsed = draw(&mut app, vec![]);
        assert!(!app.information_open);
        assert!(!has_text(&collapsed, "Nom complet"));
        assert!(text_rect(&collapsed, &app.document.view.text).height() >= body.height());
        app.apply(Command::Information);
        let reopened = draw(&mut app, vec![]);
        assert!(app.information_open);
        assert_eq!(app.information_group, InformationGroup::Fields("sender"));
        assert!(has_text(&reopened, "Nom complet"));
        assert_eq!(app.revision, revision);
        assert!(!app.is_dirty());
        assert_eq!(app.document.source(), source);
        assert_eq!(app.selection, selection);
        click(
            &mut app,
            &mut draw,
            text_rect(&reopened, "Correspondance").center(),
        );
        draw(&mut app, vec![]);
        let correspondence = draw(&mut app, vec![]);
        let original_subject = app.metadata.courrier_mut().objet.clone();
        let subject = text_rect(&correspondence, &app.metadata.courrier_mut().objet);
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
        assert_eq!(app.metadata.courrier_mut().objet, "Objet modifié");
        assert_eq!(app.document.source(), source);
        assert_eq!(app.selection, selection);
        assert_eq!(app.revision, revision + 1);
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
        assert_eq!(app.metadata.courrier_mut().objet, original_subject);
        assert_eq!(app.document.source(), source);
        app.dispatch_action(Command::Redo, &ctx);
        draw(&mut app, vec![]);
        assert_eq!(app.metadata.courrier_mut().objet, "Objet modifié");
        assert_eq!(app.revision, revision + 3);
        let current = draw(&mut app, vec![]);
        click(
            &mut app,
            &mut draw,
            text_rect(&current, "Correspondance").center(),
        );
        draw(&mut app, vec![]);
        assert!(!app.information_open);
        assert_eq!(app.metadata.courrier_mut().objet, "Objet modifié");
        assert_eq!(app.revision, revision + 3);
    }

    #[test]
    fn information_and_presentations_work_from_palette_and_custom_shortcuts() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        let source = app.document.source().to_owned();
        let revision = app.revision;
        app.apply(Command::Palette);
        app.palette_query = "Informations du courrier".into();
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
        assert_eq!(app.revision, revision);
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
        app.palette_query = "modèle de courrier de service".into();
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
        assert_eq!(app.template, Model::Service.source());
        assert!(app.is_dirty());
        assert_eq!(app.revision, revision + 1);
        assert_eq!(app.stored_document().template, Model::Service.source());
        assert_eq!(app.document.source(), source);
        app.apply(Command::Courrier(CourrierCommand::Model(Model::Service)));
        assert_eq!(app.revision, revision + 1); // Choisir la présentation active est sans effet.

        let config=serde_json::from_str(r#"{"document.information":[{"key":"F6"}],"courrier.model.institutional":[{"key":"F7"}]}"#).unwrap();
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
        assert_eq!(app.template, Model::Institutional.source());
        assert_eq!(app.revision, revision + 2);
        assert!(!app.is_dirty());
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
        assert_eq!(app.document.source(), source);
    }

    #[test]
    fn remove_link_keeps_other_styles_and_selection_and_can_be_undone() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx, None);
        app.document = Document::from_source(
            "Avant #link(\"https://example.org\")[*Élodie* et _Noël_] après.",
        )
        .unwrap();
        let source = app.document.source().to_owned();
        let start = 6;
        let end_byte = app.document.view.text.find(" après.").unwrap();
        let end = app.document.view.text[..end_byte].chars().count();
        app.selection = editor::cursor(end..start);
        let selection = app.selection;
        let text = app.document.view.text.clone();
        let styles = app.document.view.styles.clone();
        app.apply(Command::RemoveLink);
        assert_eq!(app.selection, selection);
        assert_eq!(app.document.view.text, text);
        for (before, after) in styles.iter().zip(&app.document.view.styles) {
            assert_eq!(before.bold, after.bold);
            assert_eq!(before.italic, after.italic);
            assert!(after.link.is_none());
        }
        app.apply(Command::Undo);
        assert_eq!(app.document.source(), source);
        assert_eq!(app.selection, selection);
        app.apply(Command::Redo);
        assert!(
            app.document
                .view
                .styles
                .iter()
                .all(|style| style.link.is_none())
        );
    }

    #[test]
    fn shortcut_override_updates_dispatch_and_hints_without_changing_the_document() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        frame(&mut app, &ctx, vec![]);
        let command = Command::File(FileCommand::Export);
        let original = app.keymap.hint(&ctx, command);
        let config = serde_json::from_str(r#"{"document.export":[{"key":"F6"}]}"#).unwrap();
        app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &config).unwrap();
        let hint = app.keymap.hint(&ctx, command);
        assert_ne!(hint, original);
        assert!(hint.contains("F6"));
        let source = app.document.source().to_owned();
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
        assert_eq!(app.document.source(), source);
        assert!(!app.is_dirty());
    }

    #[test]
    fn save_reopen_restores_the_letter_and_failed_open_preserves_current_work() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("courrier.typsto");
        assert!(!app.is_dirty());
        app.selection = editor::cursor(20..35);
        app.apply(Command::Bold);
        assert!(app.is_dirty());
        app.apply(Command::Undo);
        assert!(!app.is_dirty());
        app.metadata.courrier_mut().objet = "Objet personnalisé".into();
        app.template = Model::Service.source();
        app.template = app.template.replace("VILLE DE VALMONT", "MODÈLE ARCHIVÉ");
        app.document.view.insert_text("Élodie : ", 0);
        app.changed();
        let letter = app.stored_document();
        assert!(app.save_path(path.clone()));
        assert!(!app.is_dirty());
        app.document.view.insert_text("Brouillon ", 0);
        app.changed();
        assert!(app.is_dirty());
        assert!(app.open_path(path.clone()));
        assert_eq!(app.stored_document(), letter);
        assert_eq!(app.file_path, Some(path.clone()));
        assert!(!app.is_dirty());
        assert!(app.undo.is_empty() && app.redo.is_empty());
        assert_eq!(app.selection.primary.index, 0);
        assert!(app.pages.is_empty() && app.sync.is_none());
        assert_eq!(app.request(None).template, letter.template);
        app.document.view.insert_text("À conserver ", 0);
        app.changed();
        let before = app.stored_document();
        let bad = dir.path().join("illisible.typsto");
        std::fs::write(&bad, "invalide").unwrap();
        assert!(!app.open_path(bad));
        assert_eq!(app.stored_document(), before);
        assert_eq!(app.file_path, Some(path.clone()));
        assert!(app.is_dirty());
        assert!(!app.save_path(dir.path().join("absent/impossible.typsto")));
        assert_eq!(app.file_path, Some(path.clone()));
        assert!(app.is_dirty());
        assert_eq!(storage::read(&path).unwrap(), letter);
    }

    #[test]
    fn save_shortcut_and_save_as_keep_both_files_and_track_clean_state() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("premier.typsto");
        assert!(app.save_path(first.clone()));
        let initial = storage::read(&first).unwrap();
        app.document.view.insert_text("Un ajout. ", 0);
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
        app.metadata.courrier_mut().objet = "Deuxième courrier".into();
        let second = dir.path().join("second.typsto");
        assert!(app.save_path(second.clone()));
        assert_eq!(app.file_path, Some(second));
        assert_eq!(storage::read(&first).unwrap(), saved);
        assert!(!app.is_dirty());
    }

    #[test]
    fn unsaved_work_blocks_closing_or_replacing_until_the_user_decides() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.metadata.courrier_mut().objet = "Travail à conserver".into();
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
        assert!(app.document.source().is_empty());
        assert!(app.file_path.is_none());
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
        let mut app = App::with_context(ctx.clone(), None);
        app.document =
            Document::from_source(&"Un paragraphe à retrouver.\n\n".repeat(100)).unwrap();
        let mut world = crate::rendering::TypstWorld::default();
        let rendered = world
            .compile_with_template(app.document.source(), &app.metadata, &app.template)
            .unwrap();
        for page in 0..rendered.pages().len() {
            app.pages.push(ctx.load_texture(
                format!("test-page-{page}"),
                egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
                egui::TextureOptions::LINEAR,
            ));
        }
        app.sync = Some(RenderSync::new(
            app.revision,
            world,
            rendered,
            app.document.source_map.clone(),
        ));
        let end = app.document.view.len() - 12;
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
        app.revision += 1;
        app.navigate_backward(location, &ctx);
        assert_eq!(app.selection.primary.index, 0);
        app.revision -= 1;
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
        let mut app = App::with_context(ctx.clone(), None);
        let mut world = crate::rendering::TypstWorld::default();
        let rendered = world
            .compile_with_template(app.document.source(), &app.metadata, &app.template)
            .unwrap();
        let texture = ctx.load_texture(
            "click-test",
            egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
            egui::TextureOptions::LINEAR,
        );
        let texture_id = texture.id();
        app.pages = vec![texture];
        app.sync = Some(RenderSync::new(
            app.revision,
            world,
            rendered,
            app.document.source_map.clone(),
        ));
        app.submitted = app.revision;
        app.rendered_revision = Some(app.revision);
        let index = app
            .document
            .view
            .text
            .find("Notre")
            .map(|byte| app.document.view.text[..byte].chars().count())
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
        let mut expected = app.document.view.text.clone();
        expected.insert_str(app.document.view.byte_index(index), "Ajout ");
        draw(&mut app, 0.5, vec![egui::Event::Text("Ajout ".into())]);
        assert_eq!(app.document.view.text, expected);
    }

    #[test]
    fn preview_paints_selection_at_page_scale_and_clears_it_when_unavailable() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        let mut world = crate::rendering::TypstWorld::default();
        let rendered = world
            .compile_with_template(app.document.source(), &app.metadata, &app.template)
            .unwrap();
        app.pages = vec![ctx.load_texture(
            "highlight-test",
            egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
            egui::TextureOptions::LINEAR,
        )];
        app.sync = Some(RenderSync::new(
            app.revision,
            world,
            rendered,
            app.document.source_map.clone(),
        ));
        app.submitted = app.revision;
        app.rendered_revision = Some(app.revision);
        let byte = app.document.view.text.find("jeudi").unwrap();
        let index = app.document.view.text[..byte].chars().count();
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
        app.revision += 1;
        assert!(draw(&mut app, 400.0).is_empty());
        app.revision -= 1;
        app.rendering_error = Some("essai".into());
        assert!(draw(&mut app, 400.0).is_empty());
        app.rendering_error = None;
        assert!(!draw(&mut app, 400.0).is_empty());
    }

    #[test]
    fn cross_paragraph_formatting_undo_restores_source_and_selection() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx, None);
        let source = app.document.source().to_owned();
        app.selection = editor::cursor(3..180);
        let selection = app.selection;
        app.apply(Command::Bold);
        assert!(app.document.view.mark_state(3..180, Mark::Bold).0);
        assert_eq!(app.selection, selection);
        app.apply(Command::Undo);
        assert_eq!(app.document.source(), source);
        assert_eq!(app.selection, selection);
        app.apply(Command::Redo);
        assert!(app.document.view.mark_state(3..180, Mark::Bold).0);
    }

    #[test]
    fn native_egui_replaces_a_unicode_selection_without_source_markers() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.document = Document::from_source("Élodie écrit.\n\nDeuxième paragraphe.").unwrap();
        app.selection = editor::cursor(0..6);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![egui::Event::Text("Noël".into())]);
        assert!(
            app.document.view.text.starts_with("Noël écrit."),
            "{}",
            app.document.view.text
        );
        assert_eq!(
            app.document.view.len(),
            app.document.view.text.chars().count()
        );
        assert!(Document::from_source(app.document.source()).is_ok());
        app.apply(Command::Undo);
        assert!(app.document.view.text.starts_with("Élodie écrit."));
    }

    #[test]
    fn native_keyboard_shortcut_formats_multiple_paragraphs() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
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
        assert!(app.document.view.mark_state(0..180, Mark::Italic).0);
        assert!(!app.document.view.text.contains("#emph"));
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
            let mut app = App::with_context(ctx.clone(), None);
            app.document = Document::from_source(source).unwrap();
            let end = app.document.view.len();
            app.selection = editor::cursor(end..end);
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            frame(&mut app, &ctx, vec![egui::Event::Text(closing.into())]);
            if let Some(kind) = kind {
                assert_eq!(app.document.view.blocks[0], kind);
            } else {
                assert!(app.document.view.mark_state(0..7, Mark::Bold).0);
            }
            app.apply(Command::Undo);
            assert_eq!(app.document.view.text, expected);
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
            let mut app = App::with_context(ctx.clone(), None);
            app.document = Document::from_source("Échéance").unwrap();
            app.selection = editor::cursor(8..8);
            app.apply(Command::Block(BlockKind::Description));
            app.force_selection = true;
            frame(&mut app, &ctx, vec![]);
            if matches!(separator, egui::Event::Text(_)) {
                frame(&mut app, &ctx, vec![egui::Event::Text(":".into())]);
            }
            frame(&mut app, &ctx, vec![separator]);
            frame(&mut app, &ctx, vec![egui::Event::Text("Vendredi".into())]);
            assert_eq!(app.document.view.text, "Échéance\tVendredi");
            assert_eq!(app.document.view.blocks, [BlockKind::Description]);
            let restored = Document::from_source(app.document.source()).unwrap();
            assert_eq!(restored.view.text, app.document.view.text);
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
            let end = app.document.view.len();
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
            assert_eq!(app.document.view.blocks, [BlockKind::Description; 2]);
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
                app.document.view.blocks,
                [BlockKind::Description, BlockKind::Paragraph]
            );
        }
    }

    #[test]
    fn quote_keyboard_edits_body_and_attribution_and_exits_without_losing_source() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.document = Document::from_source("Bonjour").unwrap();
        app.selection = editor::cursor(7..7);
        app.apply(Command::Block(BlockKind::Quote));
        frame(&mut app, &ctx, vec![]);
        assert_eq!(
            app.document.view.blocks,
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
        assert_eq!(app.document.view.text, "Bonjour\nÉlodie");
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
        assert_eq!(app.document.view.text, "Bonjour\nDeuxième pensée.\nÉlodie");
        assert_eq!(
            app.document.view.blocks,
            [
                BlockKind::Quote,
                BlockKind::Quote,
                BlockKind::QuoteAttribution
            ]
        );
        let source = app.document.source().to_owned();
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(
            app.document.view.blocks,
            [
                BlockKind::Quote,
                BlockKind::Quote,
                BlockKind::QuoteAttribution,
                BlockKind::Paragraph
            ]
        );
        frame(&mut app, &ctx, vec![egui::Event::Text("Suite.".into())]);
        let restored = Document::from_source(app.document.source()).unwrap();
        assert_eq!(restored.view, app.document.view);
        app.apply(Command::Undo);
        app.apply(Command::Undo);
        app.apply(Command::Undo);
        assert_eq!(app.document.source(), source);
    }

    #[test]
    fn converting_multiple_paragraphs_to_quote_preserves_selection_and_can_be_undone() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx, None);
        app.document = Document::from_source("Un.\n\nDeux.\n\nTrois.").unwrap();
        let original = app.document.clone();
        app.selection = CCursorRange::two(CCursor::new(3), CCursor::new(0));
        app.apply(Command::Block(BlockKind::Quote));
        assert_eq!(app.selection.primary.index, 0);
        assert_eq!(app.selection.secondary.index, 3);
        assert_eq!(app.document.view.text, "Un.\n\n\nDeux.\n\nTrois.");
        assert_eq!(
            app.document.view.blocks,
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
            Document::from_source(app.document.source()).unwrap().view,
            app.document.view
        );
        app.apply(Command::Undo);
        assert_eq!(app.document, original);
        app.selection = editor::cursor(0..8);
        app.apply(Command::Block(BlockKind::Quote));
        assert_eq!(app.document.view.quote_groups()[0].body, 0..3);
        app.selection = editor::cursor(0..app.document.view.len());
        app.apply(Command::Block(BlockKind::Paragraph));
        assert!(
            !app.document
                .view
                .blocks
                .iter()
                .any(|kind| matches!(kind, BlockKind::Quote | BlockKind::QuoteAttribution))
        );
    }

    #[test]
    fn description_tab_with_a_selection_and_enter_in_text_create_an_empty_label() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx.clone(), None);
        app.document = Document::from_source("/ Échéance: Vendredi").unwrap();
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
        assert_eq!(app.document.view.text, "Échéance\tVendredi");
        app.selection = editor::cursor(12..12);
        app.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let before = app.document.source().to_owned();
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert_eq!(app.document.view.text, "Échéance\tVen\n\tdredi");
        assert_eq!(app.selection.primary.index, 13);
        assert_eq!(app.document.view.blocks, [BlockKind::Description; 2]);
        app.apply(Command::Undo);
        assert_eq!(app.document.source(), before);
        app.apply(Command::Redo);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![egui::Event::Text("Contact".into())]);
        assert_eq!(app.document.view.text, "Échéance\tVen\nContact\tdredi");
        let restored = Document::from_source(app.document.source()).unwrap();
        assert_eq!(restored.view, app.document.view);
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
                        action =
                            crate::toolbar::show(ui, &app.document.view, app.range(), &app.keymap);
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
            let mut app = App::with_context(ctx.clone(), None);
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            ctx.style_mut(|style| style.animation_time = 0.0);
            app.document = Document::from_source("Été à Paris.").unwrap();
            app.selection = editor::cursor(0..3);
            let text = app.document.view.text.clone();
            for (icon, command, mark) in [
                (Icon::Bold, Command::Bold, Mark::Bold),
                (Icon::Italic, Command::Italic, Mark::Italic),
            ] {
                let (output, _) = draw(&mut app, &ctx, vec![]);
                let pos = icon_rect(&output, &ctx, icon).center();
                assert_eq!(click(&mut app, &ctx, pos), Some(command));
                assert!(app.document.view.mark_state(0..3, mark).0);
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
                assert_eq!(app.document.view.blocks[0], kind);
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
            assert_eq!(app.document.view.text, text);
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let pos = icon_rect(&output, &ctx, Icon::Quote).center();
            assert_eq!(
                click(&mut app, &ctx, pos),
                Some(Command::Block(BlockKind::Quote))
            );
            assert_eq!(
                app.document.view.blocks,
                [BlockKind::Quote, BlockKind::QuoteAttribution]
            );
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let pos = icon_rect(&output, &ctx, Icon::Quote).center();
            assert_eq!(
                click(&mut app, &ctx, pos),
                Some(Command::Block(BlockKind::Paragraph))
            );
            assert_eq!(app.document.view.blocks[0], BlockKind::Paragraph);
        }
    }

    #[test]
    fn toolbar_groups_the_three_list_commands_in_a_single_menu() {
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut app = App::with_context(ctx.clone(), None);
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
            assert_eq!(app.document.view.blocks[0], kind);
        }
    }

    #[test]
    fn changing_model_restores_automatic_markers_without_altering_the_body() {
        let ctx = egui::Context::default();
        let mut app = App::with_context(ctx, None);
        let source = app.document.source().to_owned();
        app.metadata.options_mut().bullet_marker = Some("◆".into());
        app.presentation.custom_bullet_marker = true;
        app.apply(Command::Courrier(CourrierCommand::Model(Model::Service)));
        assert!(app.metadata.options().bullet_marker.is_none());
        assert_eq!(app.presentation, ModelPresentation::default());
        assert_eq!(app.document.source(), source);
    }

    #[test]
    fn document_information_offers_only_the_markers_allowed_by_the_template() {
        let ctx = egui::Context::default();
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut app = App::with_context(ctx.clone(), None);
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
        assert_eq!(app.metadata.options().bullet_marker.as_deref(), Some("▪"));
        let output = draw(&mut app, vec![]);
        assert!(point(&output, "Caractère personnalisé…").is_none());
    }
}
