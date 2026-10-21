use crate::preferences::{EditorFont, EditorStyle};
use crate::{
    annotations::AnnotationKind,
    document::{BlockKind, InlineStyle, Projection},
};
use eframe::egui::{
    self, Color32, FontFamily, Stroke, TextBuffer, TextFormat,
    text::{CCursor, CCursorRange, Galley, LayoutJob},
};
use std::{ops::Range, sync::Arc};

pub const EDITOR_ID: &str = "corps-courrier";
const DESCRIPTION_LABEL_SCALE: f32 = 0.9;
const IMAGE_HEIGHT: f32 = 220.0;
const IMAGE_PADDING: f32 = 10.0;
const IMAGE_CAPTION_GAP: f32 = 8.0;
const ANNOTATION_SCALE: f32 = 0.9;

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
    let annotation = AnnotationKind::for_block(kind).is_some();
    let bold = style.bold || matches!(kind, BlockKind::Heading(1 | 2));
    let italic = style.italic || annotation || matches!(kind, BlockKind::Heading(3));
    let variant = match (bold, italic) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    };
    let size = match kind {
        BlockKind::Heading(level) => editor.size * 1.2_f32.powi(4 - i32::from(level)),
        _ if annotation => editor.size * ANNOTATION_SCALE,
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
        } else if annotation {
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
    let placeholder_space = placeholder.size().x + editor.size * DESCRIPTION_LABEL_SCALE * 0.35;
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
        if kind == BlockKind::Description && in_term {
            format.font_id.size *= DESCRIPTION_LABEL_SCALE;
        }
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
    let mut galley = if aligned
        .blocks
        .iter()
        .any(|k| matches!(k, BlockKind::Quote | BlockKind::Image(_)))
    {
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
    space_blocks(galley, &aligned.blocks, editor.size)
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
        if matches!(kind, BlockKind::Image(_)) && text.starts_with(crate::document::IMAGE_OBJECT) {
            paragraphs.push(layout_image(ui, local));
            start = end + 1;
            continue;
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

/// L’image occupe un espace au-dessus de sa légende. Son caractère objet est
/// invisible, avec son avance compensée au début de la légende : celle-ci reste ainsi
/// une vraie ligne native, même vide. Deux lignes partageant le même indice de
/// curseur perdaient cette distinction lors du clamp effectué par le TextEdit.
fn layout_image(ui: &egui::Ui, mut job: LayoutJob) -> Arc<Galley> {
    let width = job.wrap.max_width.max(1.0);
    job.wrap.max_width = (width - 2.0 * IMAGE_PADDING).max(1.0);
    let object = &mut job.sections[0];
    object.leading_space = -ui.fonts_mut(|fonts| {
        fonts.glyph_width(&object.format.font_id, crate::document::IMAGE_OBJECT)
    });
    object.format.color = Color32::TRANSPARENT;
    let mut caption = ui.fonts_mut(|f| f.layout_job(job));
    let inset = Arc::make_mut(&mut caption);
    let top = IMAGE_HEIGHT + IMAGE_CAPTION_GAP;
    for row in &mut inset.rows {
        row.pos.y += top;
        let row = Arc::make_mut(&mut row.row);
        // Déplacer les glyphes plutôt que row.pos.x garde les correspondances
        // entre clics et curseurs utilisées par egui.
        for glyph in &mut row.glyphs {
            glyph.pos.x += IMAGE_PADDING;
        }
        for vertex in &mut row.visuals.mesh.vertices {
            vertex.pos.x += IMAGE_PADDING;
        }
        row.visuals.mesh_bounds = row
            .visuals
            .mesh_bounds
            .translate(egui::vec2(IMAGE_PADDING, 0.0));
        row.size.x += IMAGE_PADDING;
    }
    inset.rect = inset.rect.translate(egui::vec2(IMAGE_PADDING, top));
    inset.rect.max.x = width;
    inset.mesh_bounds = inset.mesh_bounds.translate(egui::vec2(IMAGE_PADDING, top));
    caption
}

fn image_frame(galley: &Galley, range: Range<usize>) -> egui::Rect {
    let top =
        galley.pos_from_cursor(CCursor::new(range.start)).top() - IMAGE_HEIGHT - IMAGE_CAPTION_GAP;
    let mut end = CCursor::new(range.end);
    end.prefer_next_row = range.len() == 1;
    let bottom = galley.pos_from_cursor(end).bottom() + IMAGE_PADDING;
    egui::Rect::from_min_max(
        egui::pos2(0.0, top),
        egui::pos2(galley.job.wrap.max_width, bottom),
    )
}

#[derive(Clone, Copy)]
pub enum ImageAction {
    Select(usize),
    Replace,
    Information,
    Remove,
}

pub fn paint_images(
    ui: &mut egui::Ui,
    view: &Projection,
    output: &egui::text_edit::TextEditOutput,
    textures: &std::collections::HashMap<String, egui::TextureHandle>,
    selection: Range<usize>,
    editor: EditorStyle,
) -> Option<(u64, ImageAction)> {
    let mut action = None;
    let painter = ui.painter_at(output.text_clip_rect);
    for (line, range) in view.line_ranges().iter().enumerate() {
        let BlockKind::Image(id) = view.blocks[line] else {
            continue;
        };
        let Some(image) = view.images.get(&id) else {
            continue;
        };
        let rect =
            image_frame(&output.galley, range.clone()).translate(output.galley_pos.to_vec2());
        let image_area =
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), IMAGE_HEIGHT));
        if !rect.intersects(output.text_clip_rect) {
            continue;
        }
        let selected = selection.start <= range.end && selection.end >= range.start;
        painter.rect_stroke(
            rect,
            5.0,
            Stroke::new(
                if selected { 1.5_f32 } else { 0.75 },
                if selected {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().widgets.noninteractive.bg_stroke.color
                },
            ),
            egui::StrokeKind::Inside,
        );
        let inset = rect.width() * 0.1;
        painter.hline(
            rect.left() + inset..=rect.right() - inset,
            image_area.bottom() - 2.0,
            Stroke::new(
                0.75_f32,
                ui.visuals().weak_text_color().gamma_multiply(0.45),
            ),
        );
        if let Some(texture) = textures.get(&image.path) {
            let available = egui::Rect::from_min_max(
                image_area.min + egui::vec2(IMAGE_PADDING, 34.0),
                image_area.max - egui::vec2(IMAGE_PADDING, 14.0),
            );
            let native = texture.size_vec2();
            let factor = (available.width() / native.x).min(available.height() / native.y);
            let picture = egui::Rect::from_center_size(available.center(), native * factor);
            painter.image(
                texture.id(),
                picture,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let response = ui.interact(
            image_area.intersect(output.text_clip_rect),
            ui.id().with(("image", id)),
            // Réserver aussi le début d’un glissement : sinon le TextEdit situé
            // dessous déplace le curseur dès l’appui, avant le clic des boutons.
            egui::Sense::click_and_drag(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                true,
                if image.alt.is_empty() {
                    crate::tr!("image-label")
                } else {
                    image.alt.clone()
                },
            )
        });
        if response.clicked() {
            action = Some((id, ImageAction::Select(range.start)));
        }
        if selected {
            let mut x = rect.left() + 7.0;
            for (label, kind, width) in [
                (crate::tr!("image-replace"), ImageAction::Replace, 90.0),
                (
                    crate::tr!("image-information"),
                    ImageAction::Information,
                    112.0,
                ),
                (crate::tr!("image-remove"), ImageAction::Remove, 90.0),
            ] {
                let button = egui::Rect::from_min_size(
                    egui::pos2(x, rect.top() + 5.0),
                    egui::vec2(width, 24.0),
                );
                if button.intersects(output.text_clip_rect)
                    && ui.put(button, egui::Button::new(label).small()).clicked()
                {
                    action = Some((id, kind));
                }
                x += width + 5.0;
            }
        } else {
            painter.text(
                rect.min + egui::vec2(10.0, 8.0),
                egui::Align2::LEFT_TOP,
                crate::tr!("image-label"),
                editor.font.font_id(editor.size * 0.8, 0),
                ui.visuals().weak_text_color(),
            );
        }
    }
    action
}

fn description_placeholder(ui: &egui::Ui, editor: EditorStyle) -> Arc<Galley> {
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

fn block_spacing(kind: BlockKind, size: f32) -> (f32, f32) {
    match kind {
        BlockKind::Heading(1) => (size * 0.9, size * 0.55),
        BlockKind::Heading(2) => (size * 0.8, size * 0.35),
        BlockKind::Heading(3) => (size * 0.7, size * 0.3),
        BlockKind::Image(_) => (0.0, IMAGE_PADDING * 2.0),
        _ => (0.0, 0.0),
    }
}

fn space_blocks(mut galley: Arc<Galley>, blocks: &[BlockKind], size: f32) -> Arc<Galley> {
    if !blocks
        .iter()
        .any(|kind| matches!(kind, BlockKind::Heading(_) | BlockKind::Image(_)))
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
        let (before, after) = block_spacing(blocks[block], size);
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
    for (line, range) in ranges.iter().enumerate() {
        if matches!(view.blocks[line], BlockKind::Image(_)) {
            let rect =
                image_frame(&output.galley, range.clone()).translate(output.galley_pos.to_vec2());
            backgrounds.push(egui::Shape::rect_filled(
                rect,
                5.0,
                ui.visuals().faint_bg_color,
            ));
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
            annotation.kind.placeholder(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn image_layout_keeps_native_indices_wrapping_and_caption_inside_the_frame() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        for caption in [
            ", caption: [Une longue légende à lire et à sélectionner dans son intégralité.]",
            "",
        ] {
            let source =
                format!("= Visite\n\n#figure(image(\"images/parc.svg\"){caption})\n\nSuite.");
            let document = Document::from_source(&source).unwrap();
            let range = document.view.image_range(1).unwrap();
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                ctx.set_theme(theme);
                for width in [180.0, 500.0] {
                    let _ = ctx.run(egui::RawInput::default(), |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let galley = layout(
                                ui,
                                &document.view,
                                &document.view.text,
                                width,
                                EditorStyle::default(),
                            );
                            assert_eq!(
                                galley
                                    .rows
                                    .iter()
                                    .map(|r| r.glyphs.len() + usize::from(r.ends_with_newline))
                                    .sum::<usize>(),
                                document.view.len()
                            );
                            let mut caption = CCursor::new(range.start + 1);
                            caption.prefer_next_row = true;
                            let position = galley.pos_from_cursor(galley.clamp_cursor(&caption));
                            let frame = image_frame(&galley, range.clone());
                            assert!(position.top() >= frame.top() + IMAGE_HEIGHT);
                            assert!(position.height() < EditorStyle::default().size * 2.0);
                            let last = galley.pos_from_cursor(if range.len() == 1 {
                                caption
                            } else {
                                CCursor::new(range.end)
                            });
                            assert!(frame.contains(position.center()));
                            assert!(frame.contains(last.center()));
                            let next = galley.pos_from_cursor(CCursor::new(range.end + 1));
                            assert!(next.top() >= frame.bottom());
                            for index in range.start + 2..range.end {
                                let cursor = CCursor::new(index);
                                let rect = galley.pos_from_cursor(cursor);
                                assert_eq!(
                                    galley.cursor_from_pos(rect.center().to_vec2()).index,
                                    index
                                );
                            }
                        });
                    });
                }
            }
            assert_eq!(document.source(), source);
        }
    }

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
    fn image_captions_share_the_quote_attribution_style_without_altering_source() {
        let source = "#figure(image(\"images/parc.svg\"), caption: [Légende.])\n\n#quote(block: true, attribution: [Attribution.])[Citation.]";
        let document = Document::from_source(source).unwrap();
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    for font in EditorFont::ALL {
                        let editor = EditorStyle { font, size: 20.0 };
                        let galley = layout(ui, &document.view, &document.view.text, 500.0, editor);
                        let caption = format_at(&galley, "Légende");
                        let attribution = format_at(&galley, "Attribution");
                        assert_eq!(caption.font_id, attribution.font_id);
                        assert_eq!(caption.color, attribution.color);
                        assert_eq!(caption.font_id, font.font_id(18.0, 2));
                    }
                });
            });
        }
        assert_eq!(document.source(), source);
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
