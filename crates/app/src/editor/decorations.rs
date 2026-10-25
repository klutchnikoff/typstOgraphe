//! decorations : détail interne de l’adaptateur egui.
use super::*;
/// Le soulignement natif suit la hauteur de ligne explicite. Ici, il suit
/// le bas naturel de la police, sous les descendantes et au-dessus de l’interligne.
pub(super) fn link_underlines(view: &Projection, galley: &Galley) -> Vec<[egui::Pos2; 2]> {
    let mut lines = Vec::new();
    let mut character = 0;
    for row in &galley.rows {
        let mut run: Option<(&str, [egui::Pos2; 2])> = None;
        for glyph in &row.glyphs {
            let link = view
                .styles
                .get(character)
                .and_then(|style| style.link.as_deref());
            let left = row.pos.x + glyph.pos.x;
            let right = row.pos.x + glyph.max_x();
            let y = row.pos.y + glyph.pos.y - glyph.font_ascent + glyph.font_height;
            match (&mut run, link) {
                (Some((url, points)), Some(link)) if *url == link => {
                    points[1].x = right;
                    let y = points[0].y.max(y);
                    points[0].y = y;
                    points[1].y = y;
                }
                _ => {
                    if let Some((_, points)) = run.take() {
                        lines.push(points);
                    }
                    if let Some(link) = link {
                        run = Some((link, [egui::pos2(left, y), egui::pos2(right, y)]));
                    }
                }
            }
            character += 1;
        }
        if let Some((_, points)) = run {
            lines.push(points);
        }
        character += usize::from(row.ends_with_newline);
    }
    lines
}

pub fn paint_decorations(
    ui: &egui::Ui,
    view: &Projection,
    output: &egui::text_edit::TextEditOutput,
    editor: EditorStyle,
    background: egui::layers::ShapeIdx,
) -> Option<usize> {
    let painter = ui.painter_at(output.text_clip_rect);
    let colors = EditorColors::from_visuals(ui.visuals());
    let placeholder = description_placeholder(ui, editor);
    let boxes = description_boxes(view, &output.galley, placeholder.size().x, editor.size);
    let mut backgrounds = Vec::new();
    let mut clicked_term = None;
    let ranges = view.line_ranges();
    for widget in widgets(view) {
        let rect = widget_rect(&output.galley, &widget).translate(output.galley_pos.to_vec2());
        backgrounds.push(egui::Shape::rect_filled(
            rect,
            5.0,
            ui.visuals().faint_bg_color,
        ));
        backgrounds.push(egui::Shape::rect_stroke(
            rect,
            5.0,
            Stroke::new(
                0.75_f32,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ),
            egui::StrokeKind::Inside,
        ));
    }
    for (line, range) in ranges.iter().enumerate() {
        if range.is_empty()
            && line > 0
            && line + 1 < ranges.len()
            && matches!(
                view.blocks[line],
                BlockKind::Paragraph | BlockKind::Quote | BlockKind::Postscript
            )
        {
            let mut cursor = CCursor::new(range.start);
            cursor.prefer_next_row = true;
            let row = output.galley.pos_from_cursor(cursor);
            let marker = painter.layout_no_wrap(
                "¶".into(),
                editor.font.font_id(editor.size * 0.75, 0),
                ui.visuals().weak_text_color(),
            );
            backgrounds.push(egui::Shape::galley(
                output.galley_pos + egui::vec2(row.left(), row.center().y - marker.size().y / 2.0),
                marker,
                ui.visuals().weak_text_color(),
            ));
        }
    }
    for (kind, group) in view.letter_note_groups() {
        for line in group.clone() {
            let mut cursor = CCursor::new(ranges[line].start);
            cursor.prefer_next_row = true;
            let row = output.galley.pos_from_cursor(cursor);
            let label = if line == group.start {
                if kind == BlockKind::Postscript {
                    "P.-S."
                } else {
                    "P.J."
                }
            } else if kind == BlockKind::Attachment {
                "•"
            } else {
                ""
            };
            let label = painter.layout_no_wrap(
                label.into(),
                editor.font.font_id(editor.size * 0.8, 1),
                ui.visuals().weak_text_color(),
            );
            backgrounds.push(egui::Shape::galley(
                output.galley_pos + egui::vec2(editor.size * 0.35, row.top()),
                label,
                ui.visuals().weak_text_color(),
            ));
            if ranges[line].is_empty() && (kind == BlockKind::Attachment || group.len() == 1) {
                let hint = painter.layout_no_wrap(
                    if kind == BlockKind::Postscript {
                        crate::tr!("postscript-placeholder")
                    } else {
                        crate::tr!("attachment-placeholder")
                    },
                    editor.font.font_id(editor.size * ANNOTATION_SCALE, 2),
                    ui.visuals().weak_text_color(),
                );
                backgrounds.push(egui::Shape::galley(
                    output.galley_pos + row.min.to_vec2(),
                    hint,
                    ui.visuals().weak_text_color(),
                ));
            }
        }
    }
    for group in view.quote_groups() {
        let first = output
            .galley
            .pos_from_cursor(CCursor::new(ranges[group.body.start].start));
        let last_line = group.attribution.unwrap_or(group.body.end - 1);
        let last = output
            .galley
            .pos_from_cursor(CCursor::new(ranges[last_line].end));
        let x = output.galley_pos.x + editor.size * 0.15;
        let top = output.galley_pos.y + first.top() + editor.size * 0.1;
        let bottom = output.galley_pos.y + last.bottom() - editor.size * 0.1;
        backgrounds.push(egui::Shape::rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x, top),
                egui::pos2(x + editor.size * 0.25, bottom),
            ),
            1.0,
            colors.accent,
        ));
    }
    for annotation in view
        .annotations()
        .into_iter()
        .filter(|annotation| annotation.content.is_empty())
    {
        let mut cursor = CCursor::new(annotation.content.start);
        cursor.prefer_next_row = true;
        let point = output.galley.pos_from_cursor(cursor);
        let hint = painter.layout_no_wrap(
            crate::i18n::text(annotation.kind.placeholder_key()),
            editor.font.font_id(editor.size * ANNOTATION_SCALE, 2),
            ui.visuals().weak_text_color(),
        );
        // Tous les textes indicatifs restent derrière le curseur et la sélection.
        backgrounds.push(egui::Shape::galley(
            output.galley_pos + point.min.to_vec2(),
            hint,
            ui.visuals().weak_text_color(),
        ));
    }
    for label in boxes {
        let rect = label.rect.translate(output.galley_pos.to_vec2());
        backgrounds.push(egui::Shape::rect_filled(rect, 3.0, colors.term_fill));
        backgrounds.push(egui::Shape::rect_stroke(
            rect,
            3.0,
            Stroke::new(0.75_f32, colors.bold.gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        ));
        if let Some(position) = label.placeholder {
            backgrounds.push(egui::Shape::galley(
                output.galley_pos + position.to_vec2(),
                placeholder.clone(),
                colors.bold,
            ));
            if output.response.clicked()
                && output
                    .response
                    .interact_pointer_pos()
                    .is_some_and(|pos| rect.contains(pos))
            {
                clicked_term = Some(label.start);
            }
        }
    }
    painter.set(background, egui::Shape::Vec(backgrounds));
    let stroke = Stroke::new(0.8_f32, EditorColors::from_visuals(ui.visuals()).accent);
    for points in link_underlines(view, &output.galley) {
        painter.line_segment(
            points.map(|point| output.galley_pos + point.to_vec2()),
            stroke,
        );
    }
    let mut number = 0;
    for (index, range) in view.line_ranges().iter().enumerate() {
        let kind = view.blocks[index];
        if kind == BlockKind::Numbered {
            number += 1;
        } else {
            number = 0;
        }
        if kind == BlockKind::Heading(1) {
            let end = output.galley.pos_from_cursor(CCursor::new(range.end));
            let y = output.galley_pos.y + end.bottom() + editor.size * 0.12;
            let right = output
                .response
                .rect
                .right()
                .min(output.text_clip_rect.right());
            ui.painter_at(output.text_clip_rect).hline(
                output.galley_pos.x..=right,
                y,
                Stroke::new(
                    1.0 / ui.pixels_per_point(),
                    EditorColors::from_visuals(ui.visuals()).accent,
                ),
            );
        }
        let text = match kind {
            BlockKind::Bullet | BlockKind::Description => "•".to_owned(),
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
    clicked_term
}
