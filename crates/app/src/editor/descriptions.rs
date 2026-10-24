//! descriptions : détail interne de l’adaptateur egui.
use super::*;
pub(super) fn description_placeholder(ui: &egui::Ui, editor: EditorStyle) -> Arc<Galley> {
    ui.painter().layout_no_wrap(
        crate::tr!("description-placeholder"),
        editor
            .font
            .font_id(editor.size * DESCRIPTION_LABEL_SCALE, 1),
        EditorColors::from_visuals(ui.visuals())
            .bold
            .gamma_multiply(0.7),
    )
}

pub(super) struct DescriptionBox {
    pub(super) rect: egui::Rect,
    pub(super) placeholder: Option<egui::Pos2>,
    pub(super) start: usize,
}

pub(super) fn description_boxes(
    view: &Projection,
    galley: &Galley,
    placeholder_width: f32,
    size: f32,
) -> Vec<DescriptionBox> {
    let mut boxes = Vec::new();
    for (line, range) in view.line_ranges().iter().enumerate() {
        if view.blocks[line] != BlockKind::Description {
            continue;
        }
        let end = view
            .text
            .chars()
            .skip(range.start)
            .take(range.len())
            .position(|c| c == '\t')
            .map_or(range.end, |offset| range.start + offset);
        let mut character = 0;
        for row in &galley.rows {
            let row_end = character + row.glyphs.len();
            let empty = end == range.start;
            let first = range.start.max(character);
            let last = end.min(row_end);
            let glyphs = if empty && (character..row_end).contains(&range.start) {
                &row.glyphs[range.start - character..range.start - character + 1]
            } else if first < last {
                &row.glyphs[first - character..last - character]
            } else {
                &[]
            };
            if let Some(first) = glyphs.first() {
                let left = row.pos.x + first.pos.x;
                let right = if empty {
                    left + placeholder_width
                } else {
                    row.pos.x + glyphs.last().unwrap().max_x()
                };
                let top = glyphs
                    .iter()
                    .map(|g| row.pos.y + g.pos.y - g.font_ascent)
                    .fold(f32::INFINITY, f32::min);
                let bottom = glyphs
                    .iter()
                    .map(|g| row.pos.y + g.pos.y - g.font_ascent + g.font_height)
                    .fold(f32::NEG_INFINITY, f32::max);
                boxes.push(DescriptionBox {
                    rect: egui::Rect::from_min_max(
                        egui::pos2(left, top),
                        egui::pos2(right, bottom),
                    )
                    .expand2(egui::vec2(
                        size * DESCRIPTION_LABEL_SCALE * 0.28,
                        size * DESCRIPTION_LABEL_SCALE * 0.12,
                    )),
                    placeholder: empty.then_some(egui::pos2(left, top)),
                    start: range.start,
                });
            }
            character = row_end + usize::from(row.ends_with_newline);
        }
    }
    boxes
}
