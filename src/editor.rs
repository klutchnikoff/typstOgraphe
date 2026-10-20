use crate::document::{BlockKind, InlineStyle, Projection};
use crate::preferences::{EditorFont, EditorStyle};
use eframe::egui::{
    self, Color32, FontFamily, Stroke, TextBuffer, TextFormat,
    text::{CCursor, CCursorRange, Galley, LayoutJob},
};
use std::{ops::Range, sync::Arc};

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

// Ces couleurs décrivent le sens du texte dans l’éditeur. Elles ne dépendent
// ni du type de document ni du modèle qui le compose dans Typst.
struct EditorColors {
    text: Color32,
    accent: Color32,
    bold: Color32,
    italic: Color32,
    term_fill: Color32,
}
impl EditorColors {
    fn from_visuals(visuals: &egui::Visuals) -> Self {
        if visuals.dark_mode {
            Self {
                text: visuals.text_color(),
                accent: Color32::from_rgb(148, 199, 224),
                bold: Color32::from_rgb(226, 154, 119),
                italic: Color32::from_rgb(139, 198, 158),
                term_fill: Color32::from_rgb(62, 43, 34),
            }
        } else {
            Self {
                text: visuals.text_color(),
                accent: Color32::from_rgb(40, 92, 119),
                bold: Color32::from_rgb(153, 67, 38),
                italic: Color32::from_rgb(37, 99, 74),
                term_fill: Color32::from_rgb(251, 237, 224),
            }
        }
    }
}

fn text_format(
    kind: BlockKind,
    style: &InlineStyle,
    colors: &EditorColors,
    editor: EditorStyle,
) -> TextFormat {
    let heading = matches!(kind, BlockKind::Heading(_));
    let bold = style.bold || matches!(kind, BlockKind::Heading(1 | 2));
    let italic =
        style.italic || matches!(kind, BlockKind::Heading(3) | BlockKind::QuoteAttribution);
    let variant = match (bold, italic) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    };
    let size = match kind {
        BlockKind::Heading(level) => editor.size * 1.2_f32.powi(4 - i32::from(level)),
        BlockKind::QuoteAttribution => editor.size * 0.9,
        _ => editor.size,
    };
    TextFormat {
        font_id: editor.font.font_id(size, variant),
        color: if heading || style.link.is_some() {
            colors.accent
        } else if style.bold {
            colors.bold
        } else if style.italic {
            colors.italic
        } else if kind == BlockKind::QuoteAttribution {
            colors.accent.gamma_multiply(0.85)
        } else {
            colors.text
        },
        line_height: Some(size * 1.55),
        ..Default::default()
    }
}

pub fn layout(
    ui: &egui::Ui,
    view: &Projection,
    text: &str,
    width: f32,
    editor: EditorStyle,
) -> Arc<Galley> {
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
    let colors = EditorColors::from_visuals(ui.visuals());
    let placeholder = description_placeholder(ui, editor);
    let placeholder_space = placeholder.size().x + editor.size * 0.35;
    let mut empty_terms = Vec::new();
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut line = 0;
    let mut at_line_start = true;
    let mut in_term = true;
    for (index, character) in text.chars().enumerate() {
        let style = &aligned.styles[index];
        let kind = aligned.blocks[line];
        let mut term_style = style.clone();
        if kind == BlockKind::Description && in_term && character != '\t' {
            term_style.bold = true;
        }
        let mut format = text_format(kind, &term_style, &colors, editor);
        if character == '\n' {
            format.line_height = Some(editor.size * 2.0);
        }
        let mut leading = if at_line_start && kind.is_list() {
            editor.size * 22.0 / 18.0
        } else {
            0.0
        };
        if at_line_start && kind == BlockKind::Description && character == '\t' {
            // Réserver l’espace du texte indicatif sans l’insérer dans le texte
            // éditable. Le séparateur garde un seul indice de caractère.
            leading += placeholder_space;
            empty_terms.push(index);
        }
        job.append(&character.to_string(), leading, format);
        at_line_start = character == '\n';
        if character == '\t' {
            in_term = false;
        }
        if character == '\n' {
            line += 1;
            in_term = true;
        }
    }
    if aligned.is_empty() {
        job.append(
            "",
            0.0,
            text_format(aligned.blocks[0], &aligned.typing_style, &colors, editor),
        );
    }
    let mut galley = if aligned.blocks.contains(&BlockKind::Quote) {
        layout_quote_paragraphs(ui, job, &aligned, editor)
    } else {
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    };
    if !empty_terms.is_empty() {
        let mut character = 0;
        for row in &mut Arc::make_mut(&mut galley).rows {
            for glyph in &mut Arc::make_mut(&mut row.row).glyphs {
                if empty_terms.contains(&character) {
                    // Le blanc de la tabulation couvre aussi l’étiquette vide :
                    // clics et curseurs restent gérés par le TextEdit standard.
                    glyph.pos.x -= placeholder_space;
                    glyph.advance_width += placeholder_space;
                }
                character += 1;
            }
            character += usize::from(row.ends_with_newline);
        }
    }
    space_headings(galley, &aligned.blocks, editor.size)
}

/// Chaque paragraphe est composé avec sa largeur disponible, puis assemblé avec
/// l’API egui. Les lignes repliées des citations gardent ainsi le même retrait.
fn layout_quote_paragraphs(
    ui: &egui::Ui,
    job: LayoutJob,
    view: &Projection,
    editor: EditorStyle,
) -> Arc<Galley> {
    let job = Arc::new(job);
    let mut paragraphs = Vec::new();
    let mut start = 0;
    for (line, text) in job.text.split('\n').enumerate() {
        let end = start + text.len();
        let kind = view.blocks[line];
        let indent = if matches!(kind, BlockKind::Quote | BlockKind::QuoteAttribution) {
            editor.size * 1.2
        } else {
            0.0
        };
        let mut local = LayoutJob {
            text: text.into(),
            wrap: job.wrap.clone(),
            ..Default::default()
        };
        local.wrap.max_width = (job.wrap.max_width - indent).max(1.0);
        for section in &job.sections {
            let first = section.byte_range.start.max(start);
            let last = section.byte_range.end.min(end);
            if first < last {
                let mut section = section.clone();
                section.byte_range = first - start..last - start;
                local.sections.push(section);
            }
        }
        if local.sections.is_empty() {
            local.append(
                "",
                0.0,
                text_format(
                    kind,
                    &view.typing_style,
                    &EditorColors::from_visuals(ui.visuals()),
                    editor,
                ),
            );
        }
        let mut galley = ui.fonts_mut(|fonts| fonts.layout_job(local));
        let composed = Arc::make_mut(&mut galley);
        for row in &mut composed.rows {
            let row = Arc::make_mut(&mut row.row);
            for glyph in &mut row.glyphs {
                glyph.pos.x += indent;
            }
            for vertex in &mut row.visuals.mesh.vertices {
                vertex.pos.x += indent;
            }
            row.visuals.mesh_bounds = row.visuals.mesh_bounds.translate(egui::vec2(indent, 0.0));
            row.size.x += indent;
        }
        composed.rect.max.x += indent;
        composed.mesh_bounds = composed.mesh_bounds.translate(egui::vec2(indent, 0.0));
        paragraphs.push(galley);
        start = end + 1;
    }
    Arc::new(Galley::concat(job, &paragraphs, ui.pixels_per_point()))
}

fn description_placeholder(ui: &egui::Ui, editor: EditorStyle) -> Arc<Galley> {
    ui.painter().layout_no_wrap(
        crate::tr!("description-placeholder"),
        editor.font.font_id(editor.size, 1),
        EditorColors::from_visuals(ui.visuals())
            .bold
            .gamma_multiply(0.7),
    )
}

struct DescriptionBox {
    rect: egui::Rect,
    placeholder: Option<egui::Pos2>,
    start: usize,
}

fn description_boxes(
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
                    .expand2(egui::vec2(size * 0.28, size * 0.12)),
                    placeholder: empty.then_some(egui::pos2(left, top)),
                    start: range.start,
                });
            }
            character = row_end + usize::from(row.ends_with_newline);
        }
    }
    boxes
}

fn heading_spacing(kind: BlockKind, size: f32) -> (f32, f32) {
    match kind {
        BlockKind::Heading(1) => (size * 0.9, size * 0.55),
        BlockKind::Heading(2) => (size * 0.8, size * 0.35),
        BlockKind::Heading(3) => (size * 0.7, size * 0.3),
        _ => (0.0, 0.0),
    }
}

fn space_headings(mut galley: Arc<Galley>, blocks: &[BlockKind], size: f32) -> Arc<Galley> {
    if !blocks
        .iter()
        .any(|kind| matches!(kind, BlockKind::Heading(_)))
    {
        return galley;
    }
    // Déplacer les lignes composées conserve les caractères et les curseurs
    // natifs d’egui. Le cache des polices garde sa composition d’origine.
    let spaced = Arc::make_mut(&mut galley);
    let mut block = 0;
    let mut first_row = true;
    let mut offset = 0.0;
    let mut mesh_bounds = egui::Rect::NOTHING;
    let last_row = spaced.rows.len().saturating_sub(1);
    for (index, row) in spaced.rows.iter_mut().enumerate() {
        let (before, after) = heading_spacing(blocks[block], size);
        if first_row && block > 0 {
            offset += before;
        }
        row.pos.y += (offset * spaced.pixels_per_point).round() / spaced.pixels_per_point;
        mesh_bounds = mesh_bounds.union(row.visuals.mesh_bounds.translate(row.pos.to_vec2()));
        first_row = row.ends_with_newline;
        if first_row || index == last_row {
            offset += after;
            block += 1;
        }
    }
    spaced.rect.max.y += (offset * spaced.pixels_per_point).round() / spaced.pixels_per_point;
    spaced.mesh_bounds = mesh_bounds;
    galley
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

/// Le soulignement natif suit la hauteur de ligne explicite. Ici, il suit
/// le bas naturel de la police, sous les descendantes et au-dessus de l’interligne.
fn link_underlines(view: &Projection, galley: &Galley) -> Vec<[egui::Pos2; 2]> {
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
        if let Some(line) = group.attribution
            && ranges[line].is_empty()
        {
            let point = output
                .galley
                .pos_from_cursor(CCursor::new(ranges[line].start));
            let hint = painter.layout_no_wrap(
                crate::tr!("quote-attribution-placeholder"),
                editor.font.font_id(editor.size * 0.9, 2),
                ui.visuals().weak_text_color(),
            );
            backgrounds.push(egui::Shape::galley(
                output.galley_pos + point.min.to_vec2(),
                hint,
                ui.visuals().weak_text_color(),
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn quote_wrapping_keeps_the_indent_and_native_cursors_for_body_and_attribution() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let mut document = Document::from_source("Avant.\n\n#quote(block: true, attribution: [Service des équipements, _rapport_])[Une citation assez longue pour occuper plusieurs lignes dans une colonne étroite, avec du *gras*.\n\nUn second paragraphe.]\n\n#quote(block: true)[Sans attribution.]\n\nAprès.").unwrap();
        let original = document.clone();
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            for width in [180.0, 500.0] {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let view = document.view.clone();
                        let editor = EditorStyle::default();
                        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, _| {
                            layout(ui, &view, text.as_str(), width, editor)
                        };
                        let background = ui.painter().add(egui::Shape::Noop);
                        let output = egui::TextEdit::multiline(&mut document.view)
                            .desired_width(width)
                            .layouter(&mut layouter)
                            .show(ui);
                        assert_eq!(output.galley.text(), view.text);
                        let mut line = 0;
                        for row in &output.galley.rows {
                            if matches!(
                                view.blocks[line],
                                BlockKind::Quote | BlockKind::QuoteAttribution
                            ) {
                                assert!((row.x_offset(0) - editor.size * 1.2).abs() <= 1.0);
                                assert!(row.rect().right() <= width + 1.0);
                            } else {
                                assert_eq!(row.pos.x, 0.0);
                            }
                            line += usize::from(row.ends_with_newline);
                        }
                        for (index, c) in view.text.chars().enumerate() {
                            if c.is_whitespace() {
                                continue;
                            }
                            let rect = output.galley.pos_from_cursor(CCursor::new(index));
                            assert_eq!(
                                output.galley.cursor_from_pos(rect.center().to_vec2()).index,
                                index,
                                "{c} at {index}"
                            );
                        }
                        paint_decorations(ui, &document.view, &output, editor, background);
                    });
                });
            }
        }
        assert_eq!(document, original);
    }

    #[test]
    fn description_boxes_and_placeholders_keep_native_cursor_indices_and_source() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let mut document = Document::from_source(
            "/ Échéance: Le vendredi.\n/ : Un texte dont le terme est vide.\n/ : ",
        )
        .unwrap();
        let original = document.clone();
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            for width in [200.0, 500.0] {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let view = document.view.clone();
                        let editor = EditorStyle::default();
                        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, _| {
                            layout(ui, &view, text.as_str(), width, editor)
                        };
                        let background = ui.painter().add(egui::Shape::Noop);
                        let output = egui::TextEdit::multiline(&mut document.view)
                            .desired_width(width)
                            .layouter(&mut layouter)
                            .show(ui);
                        let placeholder = description_placeholder(ui, editor);
                        let boxes = description_boxes(
                            &document.view,
                            &output.galley,
                            placeholder.size().x,
                            editor.size,
                        );
                        assert_eq!(
                            boxes
                                .iter()
                                .filter(|label| label.placeholder.is_some())
                                .count(),
                            2
                        );
                        assert_eq!(output.galley.text(), document.view.text);
                        for label in boxes.iter().filter(|label| label.placeholder.is_some()) {
                            let before = output.galley.pos_from_cursor(CCursor::new(label.start));
                            let after =
                                output.galley.pos_from_cursor(CCursor::new(label.start + 1));
                            assert!(before.left() <= label.rect.left() + editor.size * 0.3);
                            if (before.top() - after.top()).abs() < 0.1 {
                                assert!(after.left() - before.left() > placeholder.size().x);
                            }
                        }
                        for (index, c) in document.view.text.chars().enumerate() {
                            if c.is_whitespace() {
                                continue;
                            }
                            let rect = output.galley.pos_from_cursor(CCursor::new(index));
                            assert_eq!(
                                output.galley.cursor_from_pos(rect.center().to_vec2()).index,
                                index
                            );
                        }
                        paint_decorations(ui, &document.view, &output, editor, background);
                    });
                });
            }
        }
        assert_eq!(document, original);
        document.sync_source();
        assert!(!document.source().contains("Description"));
        assert_eq!(
            Document::from_source(document.source()).unwrap().view,
            document.view
        );
    }

    fn format_at<'a>(galley: &'a Galley, text: &str) -> &'a TextFormat {
        let byte = galley.job.text.find(text).unwrap();
        &galley
            .job
            .sections
            .iter()
            .find(|s| s.byte_range.contains(&byte))
            .unwrap()
            .format
    }

    #[test]
    fn semantic_styles_follow_the_theme_and_editor_size_without_changing_typst() {
        let document = Document::from_source("= Premier titre\n\nUn *gras*, une _italique_, #strong[#emph[combinaison]] et #link(\"https://example.org\")[*lien*].\n\n== Deuxième titre\n\n=== Troisième titre\n").unwrap();
        let original = document.clone();
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    for font in EditorFont::ALL {
                        for size in [12.0, 24.0] {
                            let galley = layout(
                                ui,
                                &document.view,
                                &document.view.text,
                                500.0,
                                EditorStyle { font, size },
                            );
                            let bold = format_at(&galley, "gras");
                            let italic = format_at(&galley, "italique");
                            let combined = format_at(&galley, "combinaison");
                            let h1 = format_at(&galley, "Premier titre");
                            let h2 = format_at(&galley, "Deuxième titre");
                            let h3 = format_at(&galley, "Troisième titre");
                            let link = format_at(&galley, "lien");
                            assert!(bold.color.r() > bold.color.g());
                            assert!(italic.color.g() > italic.color.r());
                            assert_eq!(combined.color, bold.color);
                            assert_eq!(combined.font_id, font.font_id(size, 3));
                            assert_eq!(link.color, h1.color);
                            assert!(!link_underlines(&document.view, &galley).is_empty());
                            assert_eq!(h1.color, h2.color);
                            assert_eq!(h2.color, h3.color);
                            for (format, factor) in [(h1, 1.728), (h2, 1.44), (h3, 1.2)] {
                                assert!((format.font_id.size - factor * size).abs() < 0.001);
                            }
                            assert_eq!(h3.font_id.family, font.font_id(size, 2).family);
                            assert_eq!(galley.job.text, document.view.text);
                        }
                    }
                });
            });
        }
        assert_eq!(document, original);
        let h3_start = document.view.text[..document.view.text.find("Troisième").unwrap()]
            .chars()
            .count();
        assert!(document.view.styles[h3_start..].iter().all(|s| !s.italic));
    }

    #[test]
    fn wrapped_heading_rule_and_spacing_preserve_cursor_positions_and_text() {
        let mut document = Document::from_source("= Un titre suffisamment long pour occuper plusieurs lignes dans une colonne étroite\n\nParagraphe après le titre.\n\n== Deuxième section\n\nTexte suivant.").unwrap();
        let original = document.clone();
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let editor = EditorStyle::default();
        let mut expected_rule = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 1100.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let view = document.view.clone();
                    let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, width| {
                        layout(ui, &view, text.as_str(), width, editor)
                    };
                    let background = ui.painter().add(egui::Shape::Noop);
                    let output = egui::TextEdit::multiline(&mut document.view)
                        .desired_width(280.0)
                        .layouter(&mut layouter)
                        .show(ui);
                    let ranges = document.view.line_ranges();
                    let end = output
                        .galley
                        .layout_from_cursor(CCursor::new(ranges[0].end));
                    assert!(
                        end.row > 0,
                        "le titre doit se répartir sur plusieurs lignes"
                    );
                    let title_bottom = output
                        .galley
                        .pos_from_cursor(CCursor::new(ranges[0].end))
                        .bottom();
                    let body_top = output
                        .galley
                        .pos_from_cursor(CCursor::new(ranges[1].start))
                        .top();
                    assert!(body_top - title_bottom > editor.size * 0.4);
                    for word in [
                        "suffisamment",
                        "étroite",
                        "Paragraphe",
                        "Deuxième",
                        "suivant",
                    ] {
                        let byte = document.view.text.find(word).unwrap();
                        let index = document.view.text[..byte].chars().count() + 2;
                        let rect = output.galley.pos_from_cursor(CCursor::new(index));
                        assert_eq!(
                            output.galley.cursor_from_pos(rect.center().to_vec2()).index,
                            index
                        );
                    }
                    expected_rule = Some((
                        output.galley_pos.x,
                        output
                            .response
                            .rect
                            .right()
                            .min(output.text_clip_rect.right()),
                        output.galley_pos.y + title_bottom + editor.size * 0.12,
                    ));
                    paint_decorations(ui, &document.view, &output, editor, background);
                    // Le cache brut ne doit pas accumuler les espacements du rendu.
                    let repeated = layout(ui, &document.view, &document.view.text, 280.0, editor);
                    let again = layout(ui, &document.view, &document.view.text, 280.0, editor);
                    assert_eq!(repeated.rect, again.rect);
                });
            },
        );
        let (left, right, y) = expected_rule.unwrap();
        let rules: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::LineSegment { points, .. }
                    if (points[0].y - y).abs() < 0.01 && (points[1].y - y).abs() < 0.01 =>
                {
                    Some(points)
                }
                _ => None,
            })
            .collect();
        assert_eq!(rules.len(), 1);
        assert!((rules[0][0].x - left).abs() < 0.01);
        assert!((rules[0][1].x - right).abs() < 0.01);
        assert_eq!(document, original);
    }
}
