//! layout : détail interne de l’adaptateur egui.
use super::*;
// Ces couleurs décrivent le sens du texte dans l’éditeur. Elles ne dépendent
// ni du type de document ni du modèle qui le compose dans Typst.
pub(super) struct EditorColors {
    pub(super) text: Color32,
    pub(super) accent: Color32,
    pub(super) bold: Color32,
    pub(super) italic: Color32,
    pub(super) term_fill: Color32,
}
impl EditorColors {
    pub(super) fn from_visuals(visuals: &egui::Visuals) -> Self {
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
                accent: Color32::from_rgb(25, 110, 180),
                bold: Color32::from_rgb(186, 78, 33),
                italic: Color32::from_rgb(22, 128, 91),
                term_fill: Color32::from_rgb(251, 237, 224),
            }
        }
    }
}

pub(super) fn text_format(
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
    layout_with_search(ui, view, text, width, editor, &[], None)
}

pub fn layout_with_search(
    ui: &egui::Ui,
    view: &Projection,
    text: &str,
    width: f32,
    editor: EditorStyle,
    matches: &[Range<usize>],
    active: Option<usize>,
) -> Arc<Galley> {
    layout_with_context(
        ui,
        view,
        text,
        width,
        editor,
        LayoutContext {
            matches,
            active,
            band: None,
        },
    )
}
#[derive(Default)]
pub struct LayoutContext<'a> {
    pub matches: &'a [Range<usize>],
    pub active: Option<usize>,
    pub band: Option<BandInset>,
}
pub fn layout_with_context(
    ui: &egui::Ui,
    view: &Projection,
    text: &str,
    width: f32,
    editor: EditorStyle,
    context: LayoutContext<'_>,
) -> Arc<Galley> {
    let LayoutContext {
        matches,
        active,
        band,
    } = context;
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
    let mut search_index = 0;
    for (index, character) in text.chars().enumerate() {
        let style = &aligned.styles[index];
        let kind = aligned.blocks[line];
        let mut term_style = style.clone();
        if kind == BlockKind::Description && in_term && character != '\t' {
            term_style.bold = true;
        }
        let mut format = text_format(kind, &term_style, &colors, editor);
        while matches
            .get(search_index)
            .is_some_and(|range| range.end <= index)
        {
            search_index += 1;
        }
        if text == view.text
            && matches
                .get(search_index)
                .is_some_and(|range| range.contains(&index))
        {
            format.background = match (ui.visuals().dark_mode, active == Some(search_index)) {
                (false, false) => Color32::from_rgb(255, 238, 171),
                (false, true) => Color32::from_rgb(255, 209, 102),
                (true, false) => Color32::from_rgb(83, 69, 29),
                (true, true) => Color32::from_rgb(116, 87, 30),
            };
        }
        if kind == BlockKind::Description && in_term {
            format.font_id.size *= DESCRIPTION_LABEL_SCALE;
        }
        let mut leading = 0.0;
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
    let mut galley = if aligned.blocks.iter().any(|k| {
        matches!(k, BlockKind::Quote | BlockKind::Image(_)) || k.is_letter_note() || k.is_list()
    }) {
        layout_paragraphs(ui, job, &aligned, editor)
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
    let galley = space_blocks(galley, &aligned.blocks, editor.size);
    widgets::reserve(galley, &aligned, band)
}

/// Chaque paragraphe est composé avec sa largeur disponible, puis assemblé avec
/// l’API egui. Même vide ou repliée, une ligne garde le retrait de son bloc.
pub(super) fn layout_paragraphs(
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
        let indent = if kind.is_letter_note() {
            editor.size * 4.0
        } else if kind.is_list() {
            editor.size * 22.0 / 18.0
        } else if matches!(kind, BlockKind::Quote | BlockKind::QuoteAttribution) {
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
            if row.glyphs.is_empty() {
                row.pos.x += indent;
            }
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
pub(super) fn layout_image(ui: &egui::Ui, mut job: LayoutJob) -> Arc<Galley> {
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

pub(super) fn block_spacing(kind: BlockKind, size: f32) -> (f32, f32) {
    match kind {
        BlockKind::Heading(1) => (size * 0.9, size * 0.55),
        BlockKind::Heading(2) => (size * 0.8, size * 0.35),
        BlockKind::Heading(3) => (size * 0.7, size * 0.3),
        BlockKind::Image(_) => (0.0, IMAGE_PADDING * 2.0),
        _ => (0.0, 0.0),
    }
}

pub(super) fn space_blocks(
    mut galley: Arc<Galley>,
    blocks: &[BlockKind],
    size: f32,
) -> Arc<Galley> {
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
