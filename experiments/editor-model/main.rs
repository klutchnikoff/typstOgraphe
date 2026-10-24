//! Diagnostic borné de phase 1.6, exclu du logiciel et des builds par défaut.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![allow(
    dead_code,
    reason = "Réutilisation des vrais modules du prototype dans un diagnostic isolé."
)]

#[path = "../../src/annotations.rs"]
mod annotations;
#[path = "../../src/assets.rs"]
mod assets;
#[path = "../../src/commands.rs"]
mod commands;
#[path = "../../src/dates.rs"]
mod dates;
#[path = "../../src/document.rs"]
mod document;
#[path = "../../src/document_types/mod.rs"]
mod document_types;
#[path = "../../src/editor.rs"]
mod editor;
#[path = "../../src/engine/mod.rs"]
mod engine;
#[path = "../../src/errors.rs"]
mod errors;
#[path = "../../src/exporting.rs"]
mod exporting;
#[path = "../../src/fonts.rs"]
mod fonts;
#[path = "../../src/i18n.rs"]
mod i18n;
#[path = "../../src/icons.rs"]
mod icons;
#[path = "../../src/identity.rs"]
mod identity;
#[path = "../../src/menus.rs"]
mod menus;
#[path = "../../src/navigation.rs"]
mod navigation;
#[path = "../../src/packages.rs"]
mod packages;
#[path = "../../src/preferences.rs"]
mod preferences;
#[path = "../../src/profile.rs"]
mod profile;
#[path = "../../src/recovery.rs"]
mod recovery;
#[path = "../../src/rendering.rs"]
mod rendering;
#[path = "../../src/search.rs"]
mod search;
#[path = "../../src/session.rs"]
mod session;
#[path = "../../src/spelling.rs"]
mod spelling;
#[path = "../../src/statistics.rs"]
mod statistics;
#[path = "../../src/storage.rs"]
mod storage;
#[path = "../../src/toolbar.rs"]
mod toolbar;

use document::Projection;
use eframe::egui::{self, text::CCursorRange};
use std::{ops::Range, path::PathBuf};
const EDITOR: &str = "table-model-spike";
const COLUMN_WIDTH: f32 = 210.0;

#[derive(Clone)]
struct Snapshot {
    view: Projection,
    columns: usize,
    selection: CCursorRange,
}
struct Table {
    state: Snapshot,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    focus_pending: bool,
}
impl Default for Table {
    fn default() -> Self {
        let mut view = Projection::default();
        view.insert_text("Nom\tPrésence\nZoé\tOui\nNoël\tNon", 0);
        Self {
            state: Snapshot {
                view,
                columns: 2,
                selection: editor::cursor(0..0),
            },
            undo: vec![],
            redo: vec![],
            focus_pending: true,
        }
    }
}
impl Table {
    fn cells(&self) -> Vec<Range<usize>> {
        let mut start = 0;
        let mut result = vec![];
        for (index, character) in self.state.view.text.chars().enumerate() {
            if matches!(character, '\t' | '\n') {
                result.push(start..index);
                start = index + 1;
            }
        }
        result.push(start..self.state.view.len());
        result
    }
    fn valid(&self) -> bool {
        self.state
            .view
            .text
            .split('\n')
            .all(|row| row.split('\t').count() == self.state.columns)
    }
    fn navigate(&mut self, backward: bool) {
        let cells = self.cells();
        let at = cells
            .iter()
            .position(|range| {
                range.start <= self.state.selection.primary.index
                    && self.state.selection.primary.index <= range.end
            })
            .unwrap_or(0);
        let next = if backward {
            at.saturating_sub(1)
        } else {
            (at + 1).min(cells.len() - 1)
        };
        self.state.selection = editor::cursor(cells[next].clone());
        self.focus_pending = true;
    }
    fn remember(&mut self, before: Snapshot) {
        if before.view != self.state.view || before.columns != self.state.columns {
            self.undo.push(before);
            self.redo.clear();
        }
    }
    fn history(&mut self, undo: bool) {
        let previous = if undo {
            self.undo.pop()
        } else {
            self.redo.pop()
        };
        if let Some(previous) = previous {
            let current = std::mem::replace(&mut self.state, previous);
            if undo {
                self.redo.push(current);
            } else {
                self.undo.push(current);
            }
            self.focus_pending = true;
        }
    }
    fn reshape(&mut self, row: bool, insert: bool) {
        if !self.valid() {
            return;
        }
        let before = self.state.clone();
        let index = self
            .cells()
            .iter()
            .position(|r| {
                r.contains(&self.state.selection.primary.index)
                    || r.end == self.state.selection.primary.index
            })
            .unwrap_or(0);
        let mut rows: Vec<Vec<String>> = self
            .state
            .view
            .text
            .split('\n')
            .map(|row| row.split('\t').map(str::to_owned).collect())
            .collect();
        if row {
            let at = index / self.state.columns;
            if insert {
                rows.insert(at + 1, vec![String::new(); self.state.columns]);
            } else if rows.len() > 1 {
                rows.remove(at);
            }
        } else {
            let at = index % self.state.columns;
            if insert {
                for row in &mut rows {
                    row.insert(at + 1, String::new());
                }
                self.state.columns += 1;
            } else if self.state.columns > 1 {
                for row in &mut rows {
                    row.remove(at);
                }
                self.state.columns -= 1;
            }
        }
        let text = rows
            .iter()
            .map(|row| row.join("\t"))
            .collect::<Vec<_>>()
            .join("\n");
        self.state.view.replace_with(&text);
        self.state.selection = editor::cursor(0..0);
        self.remember(before);
        self.focus_pending = true;
    }
    fn source(&self) -> Option<String> {
        if !self.valid() {
            return None;
        }
        let cells = self
            .state
            .view
            .text
            .split(['\n', '\t'])
            .map(document::typst_string)
            .collect::<Vec<_>>()
            .join(",\n  ");
        Some(format!(
            "#table(columns: {},\n  {}\n)",
            self.state.columns, cells
        ))
    }
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        for (modifiers, backward) in [
            (egui::Modifiers::SHIFT, true),
            (egui::Modifiers::NONE, false),
        ] {
            if ui.input_mut(|input| input.consume_key(modifiers, egui::Key::Tab)) {
                ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                self.navigate(backward);
            }
        }
        let before = self.state.clone();
        let id = egui::Id::new(EDITOR);
        if self.focus_pending {
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
            state.cursor.set_char_range(Some(self.state.selection));
            state.store(ui.ctx(), id);
            ui.memory_mut(|memory| memory.request_focus(id));
            self.focus_pending = false;
        }
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _width: f32| {
            table_layout(ui, text.as_str())
        };
        let mut output = egui::TextEdit::multiline(&mut self.state.view)
            .id(id)
            .lock_focus(true)
            .desired_width(COLUMN_WIDTH * self.state.columns as f32)
            .desired_rows(8)
            .layouter(&mut layouter)
            .show(ui);
        if let Some(selection) = output.cursor_range {
            self.state.selection = selection;
        }
        output.state.clear_undoer();
        output.state.store(ui.ctx(), id);
        if output.response.changed() {
            self.remember(before);
        }
        for column in 0..=self.state.columns {
            let x = output.galley_pos.x + column as f32 * COLUMN_WIDTH;
            ui.painter().line_segment(
                [
                    egui::pos2(x, output.galley_pos.y),
                    egui::pos2(x, output.galley_pos.y + output.galley.size().y),
                ],
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
        }
    }
}
fn table_layout(ui: &egui::Ui, text: &str) -> std::sync::Arc<egui::Galley> {
    let font = egui::FontId::monospace(18.0);
    let format = egui::TextFormat {
        font_id: font.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::default();
    let mut width = 0.0;
    for character in text.chars() {
        let glyph_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, character));
        let gap = if character == '\t' {
            (COLUMN_WIDTH - width - glyph_width).max(0.0)
        } else {
            0.0
        };
        job.append(&character.to_string(), gap, format.clone());
        if matches!(character, '\n' | '\t') {
            width = 0.0;
        } else {
            width += glyph_width;
        }
    }
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

struct Diagnostic {
    table: Table,
    capture: Option<PathBuf>,
    frames: usize,
}
impl eframe::App for Diagnostic {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Prototype isolé — tableau et tampon actuel");
            ui.label("Tab / Maj+Tab : cellule suivante / précédente. La sélection native peut traverser les cellules.");
            ui.horizontal(|ui| {
                for (label, row, insert) in [("+ ligne", true, true), ("− ligne", true, false), ("+ colonne", false, true), ("− colonne", false, false)] {
                    if ui.button(label).clicked() { self.table.reshape(row, insert); }
                }
                if ui.button("Annuler").clicked() { self.table.history(true); }
                if ui.button("Rétablir").clicked() { self.table.history(false); }
            });
            ui.separator();
            self.table.editor_ui(ui);
            ui.separator();
            if self.table.valid() { ui.label("Structure rectangulaire conservée."); }
            else { ui.colored_label(egui::Color32::RED, "Structure rompue : une saisie native a supprimé ou ajouté un séparateur. Annuler pour revenir."); }
            ui.label(format!("Sélection Unicode : {:?}", self.table.state.selection.as_sorted_char_range()));
            ui.collapsing("Source Typst expérimental (texte simple)", |ui| { ui.monospace(self.table.source().unwrap_or_default()); });
            ui.label("Limites volontaires : pas de retours dans une cellule, pas de cellules fusionnées, pas d’intégration au logiciel.");
        });
        self.frames += 1;
        if let Some(path) = &self.capture {
            if self.frames == 4 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let screenshot = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = screenshot {
                let writer =
                    std::io::BufWriter::new(std::fs::File::create(path).expect("capture file"));
                let mut encoder =
                    png::Encoder::new(writer, image.width() as u32, image.height() as u32);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let bytes = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_srgba_unmultiplied())
                    .collect::<Vec<_>>();
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&bytes)
                    .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        }
    }
}
fn main() -> eframe::Result {
    let capture = std::env::args().nth(1).map(PathBuf::from);
    eframe::run_native(
        "Diagnostic du modèle d’édition",
        eframe::NativeOptions {
            persist_window: false,
            viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 600.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            editor::install_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_theme(egui::ThemePreference::Light);
            Ok(Box::new(Diagnostic {
                table: Table::default(),
                capture,
                frames: 0,
            }))
        }),
    )
}

#[cfg(test)]
mod tests;
