use crate::{
    document::{BlockKind, Document, InlineStyle, Mark, Metadata},
    editor,
    rendering::{RenderRequest, RenderWorker},
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
#[derive(Clone)]
struct Snapshot {
    document: Document,
    selection: CCursorRange,
}
#[derive(Clone, Copy)]
enum Command {
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

pub struct App {
    pub document: Document,
    metadata: Metadata,
    compact: bool,
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
    pub fn new(cc: &eframe::CreationContext<'_>, capture: Option<PathBuf>) -> Self {
        Self::with_context(cc.egui_ctx.clone(), capture)
    }
    pub fn with_context(ctx: egui::Context, capture: Option<PathBuf>) -> Self {
        editor::install_fonts(&ctx);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.visuals.selection.bg_fill = Color32::from_rgb(174, 211, 235);
        style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
        ctx.set_style(style);
        let worker = RenderWorker::spawn(ctx);
        Self {
            document: Document::demo(),
            metadata: Metadata::default(),
            compact: false,
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
            metadata: self.metadata.clone(),
            compact: self.compact,
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
        let modal =
            self.palette_open || self.link_open || self.export_open || self.information_open;
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
        if modal {
            return;
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
                    });
            });
    }
    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Aperçu Typst").strong());
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
                for page in &self.pages {
                    let size = page.size_vec2();
                    let displayed = egui::vec2(width, width * size.y / size.x);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.add(egui::Image::new(page).fit_to_exact_size(displayed));
                    });
                    ui.add_space(10.0);
                }
            });
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
                ui.label("Dossier de destination");ui.add(egui::TextEdit::singleline(&mut self.export_path).desired_width(f32::INFINITY));ui.weak("PDF, source Typst, modèle et métadonnées. L’archive unique viendra dans une étape suivante.");export=ui.button("Exporter").clicked();
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
            self.force_selection = true;
        }
    }
    pub fn ui_frame(&mut self, ctx: &egui::Context) {
        self.rendering(ctx);
        self.shortcuts(ctx);
        let modal =
            self.information_open || self.link_open || self.palette_open || self.export_open;
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
                    ui.label(RichText::new("Réservation de la salle des Tilleuls").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Exporter").clicked() {
                            self.export_open = true;
                        }
                        if ui.button("Informations").clicked() {
                            self.information_open = true;
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
                    ui.weak("Ville de Valmont");
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
