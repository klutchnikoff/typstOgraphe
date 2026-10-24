//! geometry : détail interne de l’adaptateur egui.
use super::*;
pub fn restore_cursor(ctx: &egui::Context, range: CCursorRange) {
    store_cursor(ctx, range);
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(EDITOR_ID)));
}

pub fn store_cursor(ctx: &egui::Context, range: CCursorRange) {
    let id = egui::Id::new(EDITOR_ID);
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(range));
    state.clear_undoer();
    state.store(ctx, id);
}

/// Rectangles des résultats, fondés sur les glyphes natifs : précis aussi pour
/// les lignes repliées, les descriptions et les légendes d’images décalées.
pub fn range_rects(galley: &Galley, matches: &[Range<usize>]) -> Vec<(usize, egui::Rect)> {
    glyph_range_rects(galley, matches, false)
}

pub(super) fn glyph_range_rects(
    galley: &Galley,
    matches: &[Range<usize>],
    font_box: bool,
) -> Vec<(usize, egui::Rect)> {
    let mut result = Vec::new();
    let mut character = 0;
    let mut index = 0;
    for row in &galley.rows {
        let mut run: Option<(usize, egui::Rect)> = None;
        for glyph in &row.glyphs {
            while matches
                .get(index)
                .is_some_and(|range| range.end <= character)
            {
                index += 1;
            }
            if matches
                .get(index)
                .is_some_and(|range| range.contains(&character))
            {
                let mut rect = glyph.logical_rect().translate(row.pos.to_vec2());
                if font_box {
                    rect.max.y = rect.min.y + glyph.font_height;
                }
                match &mut run {
                    Some((previous, bounds)) if *previous == index => *bounds = bounds.union(rect),
                    _ => {
                        if let Some(run) = run.take() {
                            result.push(run);
                        }
                        run = Some((index, rect));
                    }
                }
            } else if let Some(run) = run.take() {
                result.push(run);
            }
            character += 1;
        }
        if let Some(run) = run {
            result.push(run);
        }
        character += usize::from(row.ends_with_newline);
    }
    result
}

/// Décoration du galley : ne change ni le contenu, ni les styles, ni le curseur.
pub fn paint_spelling(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    ranges: &[Range<usize>],
) {
    let painter = ui.painter_at(output.text_clip_rect);
    let color = if ui.visuals().dark_mode {
        Color32::from_rgb(245, 122, 122)
    } else {
        Color32::from_rgb(198, 42, 49)
    };
    for (_, rect) in glyph_range_rects(&output.galley, ranges, true) {
        let rect = rect.translate(output.galley_pos.to_vec2());
        if !rect.intersects(output.text_clip_rect) {
            continue;
        }
        let steps = (rect.width() / 2.0).ceil().max(1.0) as usize;
        let points = (0..=steps)
            .map(|step| {
                egui::pos2(
                    rect.left() + rect.width() * step as f32 / steps as f32,
                    rect.bottom() + if step % 2 == 0 { 0.0 } else { 1.5 },
                )
            })
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(1.1_f32, color)));
    }
}

pub fn cursor(range: Range<usize>) -> CCursorRange {
    CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end))
}
