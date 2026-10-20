use crate::{
    document::{BlockKind, Document, InlineStyle, Mark, Metadata},
    editor,
    navigation::{PagePoint, RenderSync},
    rendering::{RenderRequest, RenderWorker},
    storage::{self, Letter},
};
use eframe::egui::{
    self, Color32, FontId, RichText, TextBuffer,
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
enum Command {
    File(FileCommand),
    Bold,
    Italic,
    Clear,
    Block(BlockKind),
    Link,
    Undo,
    Redo,
}
impl Command {
    fn label(self) -> &'static str {
        match self {
            Self::File(command) => command.label(),
            Self::Bold => "Gras",
            Self::Italic => "Italique",
            Self::Clear => "Retirer la mise en forme",
            Self::Block(kind) => kind.label(),
            Self::Link => "Insérer ou modifier un lien",
            Self::Undo => "Annuler",
            Self::Redo => "Rétablir",
        }
    }
}

#[derive(Clone, Copy)]
enum FileCommand {
    New,
    Open,
    Save,
    SaveAs,
    Close,
}
impl FileCommand {
    fn label(self) -> &'static str {
        match self {
            Self::New => "Nouveau courrier",
            Self::Open => "Ouvrir un courrier…",
            Self::Save => "Enregistrer",
            Self::SaveAs => "Enregistrer sous…",
            Self::Close => "Fermer",
        }
    }
}

#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
    Close,
}

pub struct App {
    pub document: Document,
    metadata: Metadata,
    compact: bool,
    template: String,
    file_path: Option<PathBuf>,
    saved: Letter,
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
    palette_open: bool,
    palette_query: String,
    palette_index: usize,
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
    ) -> Self {
        let mut app = Self::with_context(cc.egui_ctx.clone(), capture);
        if let Some(path) = path {
            app.open_path(path);
        }
        app
    }
    pub fn with_context(ctx: egui::Context, capture: Option<PathBuf>) -> Self {
        editor::install_fonts(&ctx);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.visuals.selection.bg_fill = Color32::from_rgb(174, 211, 235);
        style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
        ctx.set_style(style);
        let worker = RenderWorker::spawn(ctx);
        let saved = Letter::demo();
        Self {
            document: Document::demo(),
            metadata: saved.metadata.clone(),
            compact: saved.compact,
            template: saved.template.clone(),
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
            palette_open: false,
            palette_query: String::new(),
            palette_index: 0,
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
    fn letter(&self) -> Letter {
        Letter {
            source: self.document.source().into(),
            metadata: self.metadata.clone(),
            template: self.template.clone(),
            compact: self.compact,
        }
    }
    fn is_dirty(&self) -> bool {
        self.saved.source != self.document.source()
            || self.saved.metadata != self.metadata
            || self.saved.template != self.template
            || self.saved.compact != self.compact
    }
    fn modal_open(&self) -> bool {
        self.palette_open
            || self.link_open
            || self.export_open
            || self.information_open
            || self.pending_action.is_some()
            || self.file_error.is_some()
    }
    fn save_path(&mut self, path: PathBuf) -> bool {
        let letter = self.letter();
        match storage::save(&path, &letter) {
            Ok(()) => {
                self.message = Some(format!("Courrier enregistré : {}", path.display()));
                self.file_path = Some(path);
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
            .set_title("Enregistrer le courrier")
            .add_filter("Courrier typstOgraphe", &[storage::EXTENSION])
            .set_file_name(
                self.file_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Courrier.typsto".into()),
            );
        if let Some(parent) = self.file_path.as_ref().and_then(|path| path.parent()) {
            dialog = dialog.set_directory(parent);
        }
        if let Some(mut path) = dialog.save_file() {
            if path.extension().is_none() {
                path.set_extension(storage::EXTENSION);
                if path.exists()
                    && rfd::MessageDialog::new()
                        .set_title("Remplacer le courrier ?")
                        .set_description(format!(
                            "Le fichier {} existe déjà. Voulez-vous le remplacer ?",
                            path.display()
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
                self.install_letter(letter, document, Some(path));
                self.message = Some("Courrier ouvert.".into());
                true
            }
            Err(error) => {
                self.file_error = Some(error);
                false
            }
        }
    }
    fn install_letter(&mut self, letter: Letter, document: Document, path: Option<PathBuf>) {
        self.document = document;
        self.metadata = letter.metadata.clone();
        self.template = letter.template.clone();
        self.compact = letter.compact;
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
                let letter = Letter::blank();
                let document = Document::from_source(&letter.source).expect("courrier vide valide");
                self.install_letter(letter, document, None);
            }
            PendingAction::Open => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Ouvrir un courrier")
                    .add_filter("Courrier typstOgraphe", &[storage::EXTENSION])
                    .add_filter("Tous les fichiers", &["*"]);
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
            FileCommand::Close => self.request_action(PendingAction::Close, ctx),
            FileCommand::Save => {
                self.save_document(false);
            }
            FileCommand::SaveAs => {
                self.save_document(true);
            }
        }
        ctx.request_repaint();
    }
    fn range(&self) -> Range<usize> {
        let range = self.selection.as_sorted_char_range();
        range.start.min(self.document.view.len())..range.end.min(self.document.view.len())
    }
    fn changed(&mut self) {
        self.document.sync_source();
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
        if let Command::File(command) = command {
            self.file_command = Some(command);
            return;
        }
        let range = self.range();
        if matches!(command, Command::Undo | Command::Redo) {
            let current = self.snapshot();
            let item = if matches!(command, Command::Undo) {
                self.undo.pop()
            } else {
                self.redo.pop()
            };
            if let Some(item) = item {
                if matches!(command, Command::Undo) {
                    self.redo.push(current)
                } else {
                    self.undo.push(current)
                }
                self.document = item.document;
                self.selection = item.selection;
                self.force_selection = true;
                self.typing_group = false;
                self.revision += 1;
                self.changed_at = Instant::now();
            }
            return;
        }
        if matches!(command, Command::Link) {
            self.open_link();
            return;
        }
        let before = self.snapshot();
        match command {
            Command::Bold => self.document.view.toggle_mark(range, Mark::Bold),
            Command::Italic => self.document.view.toggle_mark(range, Mark::Italic),
            Command::Clear => self.document.view.clear_marks(range),
            Command::Block(kind) => self.document.view.set_block(range, kind),
            _ => {}
        }
        let selected = self.range();
        if !selected.is_empty() {
            self.document.view.typing_style = self.document.view.style_at(selected.start);
        }
        self.remember(before, false);
        self.force_selection = true;
    }
    fn open_link(&mut self) {
        let range = self.range();
        if range.is_empty() {
            self.message = Some("Sélectionnez d’abord le texte du lien.".into());
            return;
        }
        if self.document.view.selected_lines(range.clone()).len() > 1 {
            self.message = Some("Un lien doit rester dans un seul paragraphe.".into());
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
            compact: self.compact,
            template: self.template.clone(),
            export_dir,
        }
    }
    fn rendering(&mut self, ctx: &egui::Context) {
        while let Ok(response) = self.worker.receiver.try_recv() {
            match response.result {
                Ok(rendered) => {
                    if response.is_export {
                        self.message = Some(format!(
                            "Courrier exporté dans {}",
                            rendered
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
                Err(_) => self.rendering_error = Some("Le moteur de rendu s’est arrêté.".into()),
            }
        }
        if self.rendered_revision != Some(self.revision) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let modal = self.modal_open();
        if modal {
            return;
        }
        for (modifiers, key, command) in [
            (
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::S,
                FileCommand::SaveAs,
            ),
            (egui::Modifiers::COMMAND, egui::Key::S, FileCommand::Save),
            (egui::Modifiers::COMMAND, egui::Key::O, FileCommand::Open),
            (egui::Modifiers::COMMAND, egui::Key::N, FileCommand::New),
        ] {
            if ctx.input_mut(|input| input.consume_key(modifiers, key)) {
                self.file_command = Some(command);
            }
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::P,
            )
        }) {
            self.palette_open = !self.palette_open;
            self.palette_query.clear();
            self.palette_index = 0;
        }
        let focused = ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID)));
        if !focused {
            return;
        }
        for (modifiers, key, command) in [
            (
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
                Command::Redo,
            ),
            (egui::Modifiers::COMMAND, egui::Key::Z, Command::Undo),
            (egui::Modifiers::COMMAND, egui::Key::Y, Command::Redo),
            (egui::Modifiers::COMMAND, egui::Key::B, Command::Bold),
            (egui::Modifiers::COMMAND, egui::Key::I, Command::Italic),
            (egui::Modifiers::COMMAND, egui::Key::K, Command::Link),
        ] {
            if ctx.input_mut(|i| i.consume_key(modifiers, key)) {
                self.apply(command);
            }
        }
        self.text_shortcuts(ctx);
    }
    fn text_shortcuts(&mut self, ctx: &egui::Context) {
        let range = self.range();
        if !range.is_empty() {
            return;
        }
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
            i.events.iter().find_map(|event| {
                if let egui::Event::Text(text) = event {
                    Some(text.clone())
                } else {
                    None
                }
            })
        });
        if typed.as_deref() == Some(" ") {
            let kind = match prefix.as_str() {
                "=" => Some(BlockKind::Heading(1)),
                "==" => Some(BlockKind::Heading(2)),
                "===" => Some(BlockKind::Heading(3)),
                "-" => Some(BlockKind::Bullet),
                "+" | "1." => Some(BlockKind::Numbered),
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
                BlockKind::Bullet | BlockKind::Numbered
            )
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.apply(Command::Block(BlockKind::Paragraph));
        }
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let range = self.range();
        let lines = self.document.view.selected_lines(range.clone());
        let kind = self.document.view.blocks[lines.start];
        let mixed = self.document.view.blocks[lines].iter().any(|k| *k != kind);
        let mut selected = kind;
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("paragraph-style")
                .width(112.0)
                .selected_text(if mixed {
                    "Styles multiples"
                } else {
                    kind.label()
                })
                .show_ui(ui, |ui| {
                    for option in [
                        BlockKind::Paragraph,
                        BlockKind::Heading(1),
                        BlockKind::Heading(2),
                        BlockKind::Heading(3),
                    ] {
                        ui.selectable_value(&mut selected, option, option.label());
                    }
                });
            if selected != kind {
                self.apply(Command::Block(selected));
            }
            ui.separator();
            for (mark, label, command) in [
                (Mark::Bold, "G", Command::Bold),
                (Mark::Italic, "I", Command::Italic),
            ] {
                let (active, mixed) = self.document.view.mark_state(self.range(), mark);
                let text = if mixed {
                    format!("{label} ·")
                } else {
                    label.into()
                };
                let button = egui::Button::new(RichText::new(text).strong()).selected(active);
                if ui
                    .add(button)
                    .on_hover_text(format!(
                        "{}{}",
                        command.label(),
                        if mixed { " (sélection mixte)" } else { "" }
                    ))
                    .clicked()
                {
                    self.apply(command);
                }
            }
            if ui
                .button("Lien")
                .on_hover_text("Insérer ou modifier un lien · ⌘K / Ctrl+K")
                .clicked()
            {
                self.apply(Command::Link);
            }
            ui.separator();
            for (kind, label) in [
                (BlockKind::Bullet, "• Liste"),
                (BlockKind::Numbered, "1. Liste"),
            ] {
                if ui.selectable_label(selected == kind, label).clicked() {
                    self.apply(Command::Block(if selected == kind {
                        BlockKind::Paragraph
                    } else {
                        kind
                    }));
                }
            }
            if ui
                .button("Effacer")
                .on_hover_text("Retirer le gras, l’italique et les liens")
                .clicked()
            {
                self.apply(Command::Clear);
            }
            ui.separator();
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler"))
                .on_hover_text("Annuler")
                .clicked()
            {
                self.apply(Command::Undo);
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir"))
                .on_hover_text("Rétablir")
                .clicked()
            {
                self.apply(Command::Redo);
            }
        });
    }
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        self.toolbar(ui);
        ui.separator();
        let before = self.snapshot();
        let old_range = self.selection;
        if self.force_selection {
            editor::restore_cursor(ui.ctx(), self.selection);
            self.force_selection = false;
        }
        let view = self.document.view.clone();
        let color = ui.visuals().text_color();
        let accent = if ui.visuals().dark_mode {
            Color32::from_rgb(148, 199, 224)
        } else {
            ACCENT
        };
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
            ui.fonts_mut(|fonts| {
                fonts.layout_job(editor::layout(&view, text.as_str(), width, color, accent))
            })
        };
        egui::ScrollArea::vertical()
            .id_salt("editor-scroll")
            .show(ui, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(22, 12))
                    .show(ui, |ui| {
                        let output = egui::TextEdit::multiline(&mut self.document.view)
                            .id(egui::Id::new(editor::EDITOR_ID))
                            .desired_width(f32::INFINITY)
                            .desired_rows(25)
                            .frame(false)
                            .font(FontId::proportional(18.0))
                            .layouter(&mut layouter)
                            .show(ui);
                        editor::paint_list_markers(ui, &self.document.view, &output);
                        if let Some(range) = output.cursor_range {
                            self.selection = range;
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
    }
    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Aperçu Typst").strong())
                .on_hover_text("Double-cliquez sur un passage pour le retrouver dans l’éditeur.");
            if self.rendered_revision != Some(self.revision) {
                ui.spinner();
                ui.weak("Actualisation…");
            } else if let Some(time) = self.rendering_time {
                ui.weak(format!(
                    "{} page(s) · {} ms",
                    self.pages.len(),
                    time.as_millis()
                ));
            }
        });
        ui.separator();
        if let Some(error) = &self.rendering_error {
            ui.colored_label(
                Color32::from_rgb(183, 69, 57),
                "Le document n’a pas pu être composé. Le dernier aperçu réussi est conservé.",
            );
            ui.collapsing("Détails", |ui| {
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
                let width = (ui.available_width() - 24.0).max(100.0);
                if self.pages.is_empty() {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.label("Composition du courrier…");
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
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.information_open {
            let old = self.metadata.clone();
            let mut open = true;
            egui::Window::new("Informations du courrier")
                .id(egui::Id::new("info"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(370.0)
                .show(ctx, |ui| {
                    egui::CollapsingHeader::new("Correspondance")
                        .default_open(true)
                        .show(ui, |ui| {
                            for (label, value) in [
                                ("Destinataire", &mut self.metadata.destinataire),
                                ("Adresse", &mut self.metadata.adresse),
                                ("Objet", &mut self.metadata.objet),
                                ("Référence", &mut self.metadata.reference),
                                ("Lieu", &mut self.metadata.lieu),
                                ("Date", &mut self.metadata.date),
                            ] {
                                ui.label(label);
                                if label == "Adresse" {
                                    ui.add(
                                        egui::TextEdit::multiline(value)
                                            .desired_rows(3)
                                            .desired_width(f32::INFINITY),
                                    );
                                } else {
                                    ui.add(
                                        egui::TextEdit::singleline(value)
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                            }
                        });
                    egui::CollapsingHeader::new("Expéditeur").show(ui, |ui| {
                        for (label, value) in [
                            ("Nom complet", &mut self.metadata.expediteur),
                            ("Fonction", &mut self.metadata.fonction),
                            ("Service", &mut self.metadata.service),
                            ("Courriel", &mut self.metadata.email),
                        ] {
                            ui.label(label);
                            ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
                        }
                    });
                    ui.add_space(8.0);
                    if ui.button("Revenir au texte").clicked() {
                        self.information_open = false;
                        self.force_selection = true;
                    }
                });
            self.information_open &= open;
            if self.metadata != old {
                self.revision += 1;
                self.changed_at = Instant::now();
            }
        }
        if self.link_open {
            let mut open = true;
            let mut apply = false;
            let mut remove = false;
            egui::Window::new("Lien")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Texte : {}",
                        self.document
                            .view
                            .text
                            .chars()
                            .skip(self.link_range.start)
                            .take(self.link_range.len())
                            .collect::<String>()
                    ));
                    ui.label("Adresse");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.link_url)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://…"),
                    );
                    if let Some(error) = &self.link_error {
                        ui.colored_label(Color32::from_rgb(183, 69, 57), error);
                    }
                    ui.horizontal(|ui| {
                        apply = ui.button("Appliquer").clicked();
                        remove = ui.button("Retirer le lien").clicked();
                    });
                });
            self.link_open &= open;
            if apply || remove {
                let url = self.link_url.trim();
                if remove
                    || ((url.starts_with("https://")
                        || url.starts_with("http://")
                        || url.starts_with("mailto:"))
                        && !url.chars().any(char::is_whitespace))
                {
                    let before = self.snapshot();
                    self.document.view.set_link(
                        self.link_range.clone(),
                        if remove { None } else { Some(url.into()) },
                    );
                    self.remember(before, false);
                    self.link_open = false;
                    self.force_selection = true;
                } else {
                    self.link_error =
                        Some("Utilisez une adresse https://, http:// ou mailto:.".into());
                }
            }
        }
        if self.palette_open {
            let mut open = true;
            let mut choice = None;
            egui::Window::new("Commandes")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(430.0)
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 85.0))
                .show(ctx, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text("Rechercher une commande…")
                            .desired_width(f32::INFINITY),
                    );
                    if !response.has_focus() {
                        response.request_focus();
                    }
                    if response.changed() {
                        self.palette_index = 0;
                    }
                    let commands = [
                        Command::File(FileCommand::New),
                        Command::File(FileCommand::Open),
                        Command::File(FileCommand::Save),
                        Command::File(FileCommand::SaveAs),
                        Command::Bold,
                        Command::Italic,
                        Command::Link,
                        Command::Clear,
                        Command::Block(BlockKind::Paragraph),
                        Command::Block(BlockKind::Heading(1)),
                        Command::Block(BlockKind::Heading(2)),
                        Command::Block(BlockKind::Heading(3)),
                        Command::Block(BlockKind::Bullet),
                        Command::Block(BlockKind::Numbered),
                        Command::Undo,
                        Command::Redo,
                    ];
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
                            .selectable_label(index == self.palette_index, command.label())
                            .clicked()
                        {
                            choice = Some(command);
                        }
                    }
                });
            self.palette_open &= open;
            if let Some(command) = choice {
                self.palette_open = false;
                self.apply(command);
            }
        }
        if self.export_open {
            let mut open = true;
            let mut export = false;
            egui::Window::new("Exporter le courrier").open(&mut open).collapsible(false).resizable(false).default_width(470.0).show(ctx,|ui|{
                ui.label("Dossier de destination");ui.add(egui::TextEdit::singleline(&mut self.export_path).desired_width(f32::INFINITY));ui.weak("PDF, pages PNG et fichiers Typst séparés. Pour conserver un courrier modifiable dans un fichier unique, utilisez Enregistrer.");export=ui.button("Exporter").clicked();
            });
            self.export_open &= open;
            if export {
                if self.metadata.destinataire.trim().is_empty()
                    || self.metadata.objet.trim().is_empty()
                {
                    self.message =
                        Some("Renseignez le destinataire et l’objet avant l’export.".into());
                } else {
                    match self
                        .worker
                        .sender
                        .send(self.request(Some(PathBuf::from(&self.export_path))))
                    {
                        Ok(()) => {
                            self.export_open = false;
                            self.message = Some("Export en cours…".into());
                        }
                        Err(_) => self.message = Some("Le moteur de rendu s’est arrêté.".into()),
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
            egui::Window::new("Enregistrer les modifications ?")
                .id(egui::Id::new("unsaved-letter"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Le courrier contient des modifications non enregistrées.");
                    if let Some(error) = &self.file_error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Enregistrer").clicked() {
                            choice = Some(0);
                        }
                        if ui.button("Ne pas enregistrer").clicked() {
                            choice = Some(1);
                        }
                        if ui.button("Annuler").clicked() {
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
            egui::Window::new("Fichier de courrier")
                .open(&mut open)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(self.file_error.as_deref().unwrap_or_default());
                    close = ui.button("Fermer").clicked();
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
            self.palette_open = false;
            self.link_open = false;
            self.export_open = false;
            self.information_open = false;
            self.pending_action = Some(PendingAction::Close);
        }
        self.rendering(ctx);
        self.shortcuts(ctx);
        let modal = self.modal_open();
        let dirty = self.is_dirty();
        let filename = self
            .file_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Courrier sans titre".into());
        let title = format!(
            "{}{} — typstOgraphe",
            filename,
            if dirty { " *" } else { "" }
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
        egui::TopBottomPanel::top("header")
            .exact_height(64.0)
            .show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("typstOgraphe").size(17.0).color(
                        if ui.visuals().dark_mode {
                            Color32::LIGHT_BLUE
                        } else {
                            ACCENT
                        },
                    ));
                    ui.separator();
                    ui.menu_button("Fichier", |ui| {
                        for command in [
                            FileCommand::New,
                            FileCommand::Open,
                            FileCommand::Save,
                            FileCommand::SaveAs,
                            FileCommand::Close,
                        ] {
                            if ui.button(command.label()).clicked() {
                                self.file_command = Some(command);
                                ui.close();
                            }
                        }
                    });
                    ui.add_sized(
                        [210.0, 18.0],
                        egui::Label::new(
                            RichText::new(format!("{}{}", filename, if dirty { " *" } else { "" }))
                                .strong(),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Exporter").clicked() {
                            self.export_open = true;
                        }
                        if ui.button("Informations").clicked() {
                            self.information_open = true;
                        }
                        if ui
                            .button("Enregistrer")
                            .on_hover_text("⌘S / Ctrl+S")
                            .clicked()
                        {
                            self.file_command = Some(FileCommand::Save);
                        }
                        if ui
                            .button("Commandes")
                            .on_hover_text("⌘⇧P / Ctrl+Shift+P")
                            .clicked()
                        {
                            self.palette_open = true;
                            self.palette_query.clear();
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.weak("Courrier");
                    ui.separator();
                    let old = self.compact;
                    egui::ComboBox::from_id_salt("template")
                        .selected_text(if self.compact {
                            "Courrier de service"
                        } else {
                            "Courrier institutionnel"
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.compact,
                                false,
                                "Courrier institutionnel",
                            );
                            ui.selectable_value(&mut self.compact, true, "Courrier de service");
                        });
                    if old != self.compact {
                        self.revision += 1;
                        self.changed_at = Instant::now();
                    }
                    ui.weak(if self.template == crate::rendering::TEMPLATE {
                        "Ville de Valmont"
                    } else {
                        "Modèle embarqué dans le courrier"
                    });
                });
            });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if modal {
                ui.disable();
            }
            ui.horizontal(|ui| {
                let range = self.range();
                if range.is_empty() {
                    ui.weak("Point d’insertion");
                } else {
                    let text: String = self
                        .document
                        .view
                        .text
                        .chars()
                        .skip(range.start)
                        .take(range.len())
                        .collect();
                    ui.label(format!(
                        "{} mots sélectionnés · {} paragraphe(s)",
                        text.split_whitespace().count(),
                        self.document.view.selected_lines(range).len()
                    ));
                }
                if let Some(message) = &self.message {
                    ui.separator();
                    ui.label(message);
                }
            });
        });
        egui::SidePanel::right("preview")
            .resizable(true)
            .default_width(520.0)
            .width_range(300.0..=1100.0)
            .show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                self.preview_ui(ui)
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if modal {
                ui.disable();
            }
            self.editor_ui(ui)
        });
        self.dialogs(ctx);
        if let Some(command) = self.file_command.take() {
            self.handle_file_command(command, ctx);
        }
        self.file_dialogs(ctx);
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
                eprintln!("Capture : {error}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_frame(ctx);
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

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
        app.metadata.objet = "Objet personnalisé".into();
        app.compact = true;
        app.template = app.template.replace("VILLE DE VALMONT", "MODÈLE ARCHIVÉ");
        app.document.view.insert_text("Élodie : ", 0);
        app.changed();
        let letter = app.letter();
        assert!(app.save_path(path.clone()));
        assert!(!app.is_dirty());
        app.document.view.insert_text("Brouillon ", 0);
        app.changed();
        assert!(app.is_dirty());
        assert!(app.open_path(path.clone()));
        assert_eq!(app.letter(), letter);
        assert_eq!(app.file_path, Some(path.clone()));
        assert!(!app.is_dirty());
        assert!(app.undo.is_empty() && app.redo.is_empty());
        assert_eq!(app.selection.primary.index, 0);
        assert!(app.pages.is_empty() && app.sync.is_none());
        assert_eq!(app.request(None).template, letter.template);
        app.document.view.insert_text("À conserver ", 0);
        app.changed();
        let before = app.letter();
        let bad = dir.path().join("illisible.typsto");
        std::fs::write(&bad, "invalide").unwrap();
        assert!(!app.open_path(bad));
        assert_eq!(app.letter(), before);
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
        let saved = app.letter();
        app.metadata.objet = "Deuxième courrier".into();
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
        app.metadata.objet = "Travail à conserver".into();
        let before = app.letter();
        app.request_action(PendingAction::Open, &ctx);
        assert!(app.pending_action.is_some()); // Aucune fenêtre de fichier tant que la décision manque.
        assert_eq!(app.letter(), before);
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
        assert_eq!(app.letter(), before);

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
        assert_eq!(app.letter(), before);
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
            .compile(app.document.source(), &app.metadata, false)
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
            .compile(app.document.source(), &app.metadata, false)
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
            .compile(app.document.source(), &app.metadata, false)
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
}
