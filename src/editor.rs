use crate::document::{BlockKind, Projection};
use crate::preferences::{EditorFont, EditorStyle};
use eframe::egui::{
    self, Color32, FontFamily, Stroke, TextBuffer, TextFormat,
    text::{CCursor, CCursorRange, LayoutJob},
};
use std::ops::Range;

pub const EDITOR_ID: &str = "corps-courrier";

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let bundled: Vec<_> = typst_assets::fonts().collect();
    // Ordre des variantes régulière, grasse, italique et grasse italique dans
    // les ressources de typst-assets 0.15.1, version fixée par Cargo.toml.
    for (family, faces) in [
        (EditorFont::Libertinus, [0, 1, 2, 3]),
        (EditorFont::ComputerModern, [9, 10, 11, 12]),
        (EditorFont::Monospace, [16, 13, 15, 14]),
    ] {
        for (variant, index) in faces.into_iter().enumerate() {
            let font_family = family.font_id(18.0, variant).family;
            let name = format!("{family:?}-{variant}");
            fonts.font_data.insert(
                name.clone(),
                egui::FontData::from_static(bundled[index]).into(),
            );
            let mut fallback = vec![name];
            fallback.extend(fonts.families[&FontFamily::Proportional].clone());
            fonts.families.insert(font_family, fallback);
        }
    }
    ctx.set_fonts(fonts);
}

pub fn layout(
    view: &Projection,
    text: &str,
    width: f32,
    color: Color32,
    strong_color: Color32,
    accent: Color32,
    editor: EditorStyle,
) -> LayoutJob {
    // Le TextEdit relance le layouter après ses mutations. Réaligner la copie
    // des styles sur le nouveau texte évite un décalage visuel pendant la saisie.
    let mut aligned = view.clone();
    if aligned.text != text {
        let old: Vec<_> = aligned.text.chars().collect();
        let new: Vec<_> = text.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        aligned.delete_char_range(prefix..old.len() - suffix);
        aligned.insert_text(
            &new[prefix..new.len() - suffix].iter().collect::<String>(),
            prefix,
        );
    }
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut line = 0;
    let mut at_line_start = true;
    for (index, character) in text.chars().enumerate() {
        let style = &aligned.styles[index];
        let kind = aligned.blocks[line];
        let heading = matches!(kind, BlockKind::Heading(_));
        let bold = style.bold || heading;
        let font_index = match (bold, style.italic) {
            (false, false) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (true, true) => 3,
        };
        let size = match kind {
            BlockKind::Heading(1) => editor.size * 4.0 / 3.0,
            BlockKind::Heading(2) => editor.size * 7.0 / 6.0,
            _ => editor.size,
        };
        let mut format = TextFormat {
            font_id: editor.font.font_id(size, font_index),
            color: if heading || style.link.is_some() {
                accent
            } else if style.bold {
                strong_color
            } else {
                color
            },
            line_height: Some(if character == '\n' {
                editor.size * 2.0
            } else {
                size * 1.55
            }),
            ..Default::default()
        };
        if style.link.is_some() {
            format.underline = Stroke::new(0.8_f32, accent);
        }
        let leading = if at_line_start && matches!(kind, BlockKind::Bullet | BlockKind::Numbered) {
            editor.size * 22.0 / 18.0
        } else {
            0.0
        };
        job.append(&character.to_string(), leading, format);
        at_line_start = character == '\n';
        if character == '\n' {
            line += 1;
        }
    }
    if aligned.is_empty() {
        job.append(
            "",
            0.0,
            TextFormat {
                font_id: editor.font.font_id(editor.size, 0),
                color,
                ..Default::default()
            },
        );
    }
    job
}

pub fn restore_cursor(ctx: &egui::Context, range: CCursorRange) {
    let id = egui::Id::new(EDITOR_ID);
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(range));
    state.clear_undoer();
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

pub fn cursor(range: Range<usize>) -> CCursorRange {
    CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end))
}

pub fn paint_list_markers(
    ui: &egui::Ui,
    view: &Projection,
    output: &egui::text_edit::TextEditOutput,
    editor: EditorStyle,
) {
    let mut number = 0;
    for (index, range) in view.line_ranges().iter().enumerate() {
        let kind = view.blocks[index];
        if kind == BlockKind::Numbered {
            number += 1;
        } else {
            number = 0;
        }
        let text = match kind {
            BlockKind::Bullet => "•".to_owned(),
            BlockKind::Numbered => format!("{number}."),
            _ => continue,
        };
        let position = output.galley.pos_from_cursor(CCursor::new(range.start));
        let point = output.galley_pos + egui::vec2(3.0, position.center().y);
        ui.painter_at(output.text_clip_rect).text(
            point,
            egui::Align2::LEFT_CENTER,
            text,
            editor.font.font_id(editor.size * 8.0 / 9.0, 0),
            ui.visuals().text_color(),
        );
    }
}
