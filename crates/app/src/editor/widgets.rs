//! Visual envelopes only. No sentinel, persisted state or independent text buffer.
use super::*;
pub const WIDGET_HEADER: f32 = 30.0;
const FOOTER: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetKind {
    List,
    Image(u64),
    Quote,
    Postscript,
    Attachment,
}
impl WidgetKind {
    pub fn label(self) -> String {
        crate::i18n::text(match self {
            Self::List => "widget-list",
            Self::Image(_) => "image-label",
            Self::Quote => "widget-quote",
            Self::Postscript => "widget-postscript",
            Self::Attachment => "widget-attachments",
        })
    }
    pub fn configurable(self) -> bool {
        matches!(self, Self::List | Self::Image(_))
    }
}
#[derive(Clone, Debug)]
pub struct Widget {
    pub kind: WidgetKind,
    pub lines: Range<usize>,
    pub range: Range<usize>,
}
/// Où s'insère le bandeau : sous l'en-tête d'un widget (paramètres, suppression) ou
/// au-dessus de la ligne visuelle qui contient l'ancre (orthographe, lien).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandPlacement {
    AfterHeader,
    AboveRow,
}
#[derive(Clone, Copy)]
pub struct BandInset {
    pub anchor: usize,
    /// Fin de la cible (mot, passage) : sert à la pointe de l'orthographe.
    pub end: usize,
    pub height: f32,
    pub placement: BandPlacement,
}

pub fn widgets(view: &Projection) -> Vec<Widget> {
    let ranges = view.line_ranges();
    let mut result = Vec::new();
    let mut at = 0;
    while at < view.blocks.len() {
        let first = at;
        let kind = match view.blocks[at] {
            k if k.is_list() => WidgetKind::List,
            BlockKind::Image(id) => WidgetKind::Image(id),
            BlockKind::Quote => WidgetKind::Quote,
            BlockKind::Postscript => WidgetKind::Postscript,
            BlockKind::Attachment => WidgetKind::Attachment,
            _ => {
                at += 1;
                continue;
            }
        };
        at += 1;
        while at < view.blocks.len()
            && match kind {
                WidgetKind::List => view.blocks[at].is_list(),
                WidgetKind::Quote => matches!(
                    view.blocks[at],
                    BlockKind::Quote | BlockKind::QuoteAttribution
                ),
                WidgetKind::Postscript => view.blocks[at] == BlockKind::Postscript,
                WidgetKind::Attachment => view.blocks[at] == BlockKind::Attachment,
                WidgetKind::Image(_) => false,
            }
        {
            at += 1;
        }
        result.push(Widget {
            kind,
            lines: first..at,
            range: ranges[first].start..ranges[at - 1].end,
        });
    }
    result
}
pub fn widget_rect(galley: &Galley, widget: &Widget) -> egui::Rect {
    widget_rect_with(galley, widget, 0.0)
}
/// Rectangle du cadre ; `extra_top` ajoute la hauteur d'un bandeau inséré sous l'en-tête.
pub fn widget_rect_with(galley: &Galley, widget: &Widget, extra_top: f32) -> egui::Rect {
    let mut first = CCursor::new(widget.range.start);
    first.prefer_next_row = true;
    let mut last = CCursor::new(widget.range.end);
    last.prefer_next_row = widget.range.is_empty()
        || matches!(widget.kind, WidgetKind::Image(_)) && widget.range.len() == 1;
    let mut top = galley.pos_from_cursor(first).top();
    if matches!(widget.kind, WidgetKind::Image(_)) {
        top -= IMAGE_HEIGHT + IMAGE_CAPTION_GAP;
    }
    egui::Rect::from_min_max(
        egui::pos2(0.0, top - WIDGET_HEADER - extra_top),
        egui::pos2(
            galley.job.wrap.max_width,
            galley.pos_from_cursor(last).bottom() + FOOTER,
        ),
    )
}
/// Hauteur du bandeau à ajouter en haut du cadre : le bandeau est dans le cadre quand
/// il se trouve sous l'en-tête, avant la première ligne du widget.
pub fn band_extra(galley: &Galley, widget: &Widget, band: Option<BandInset>) -> f32 {
    let Some(band) = band else { return 0.0 };
    if band.anchor < widget.range.start || band.anchor > widget.range.end {
        return 0.0;
    }
    match band.placement {
        BandPlacement::AfterHeader => band.height,
        BandPlacement::AboveRow => {
            let mut first = CCursor::new(widget.range.start);
            first.prefer_next_row = true;
            let mut at = CCursor::new(band.anchor);
            at.prefer_next_row = true;
            let same = (galley.pos_from_cursor(first).top() - galley.pos_from_cursor(at).top())
                .abs()
                < 0.5;
            if same { band.height } else { 0.0 }
        }
    }
}
/// Le widget qui contient l'ancre du bandeau, s'il y en a un.
pub fn band_host(view: &Projection, band: BandInset) -> Option<Widget> {
    let anchor = band.anchor.min(view.len());
    widgets(view)
        .into_iter()
        .find(|w| w.range.start <= anchor && anchor <= w.range.end)
}
pub(super) fn reserve(
    mut galley: Arc<Galley>,
    view: &Projection,
    band: Option<BandInset>,
) -> Arc<Galley> {
    let groups = widgets(view);
    if groups.is_empty() && band.is_none() {
        return galley;
    }
    let band_line = band
        .filter(|b| b.placement == BandPlacement::AfterHeader)
        .map(|b| view.line_at(b.anchor.min(view.len())));
    let composed = Arc::make_mut(&mut galley);
    let band_row = band
        .filter(|b| b.placement == BandPlacement::AboveRow)
        .map(|b| {
            let (mut chars, last) = (0, composed.rows.len().saturating_sub(1));
            for (index, row) in composed.rows.iter().enumerate() {
                chars += row.glyphs.len() + usize::from(row.ends_with_newline);
                if b.anchor < chars || index == last {
                    return index;
                }
            }
            0
        });
    let mut line = 0;
    let mut first = true;
    let mut offset = 0.0;
    let mut bounds = egui::Rect::NOTHING;
    let last = composed.rows.len().saturating_sub(1);
    for (index, row) in composed.rows.iter_mut().enumerate() {
        if band_row == Some(index) {
            offset += band.expect("band row").height;
        }
        if first {
            if band_line == Some(line) {
                offset += band.expect("band line").height;
            }
            if groups.iter().any(|w| w.lines.start == line) {
                offset += WIDGET_HEADER;
            }
        }
        row.pos.y += (offset * composed.pixels_per_point).round() / composed.pixels_per_point;
        bounds = bounds.union(row.visuals.mesh_bounds.translate(row.pos.to_vec2()));
        first = row.ends_with_newline;
        if first || index == last {
            if groups.iter().any(|w| w.lines.end == line + 1) {
                offset += FOOTER;
            }
            line += 1;
        }
    }
    composed.rect.max.y += (offset * composed.pixels_per_point).round() / composed.pixels_per_point;
    composed.mesh_bounds = bounds;
    galley
}
