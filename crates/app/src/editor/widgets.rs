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
#[derive(Clone, Copy)]
pub struct BandInset {
    pub anchor: usize,
    pub height: f32,
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
        egui::pos2(0.0, top - WIDGET_HEADER),
        egui::pos2(
            galley.job.wrap.max_width,
            galley.pos_from_cursor(last).bottom() + FOOTER,
        ),
    )
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
    let band_line = band.map(|b| view.line_at(b.anchor.min(view.len())));
    let composed = Arc::make_mut(&mut galley);
    let mut line = 0;
    let mut first = true;
    let mut offset = 0.0;
    let mut bounds = egui::Rect::NOTHING;
    let last = composed.rows.len().saturating_sub(1);
    for (index, row) in composed.rows.iter_mut().enumerate() {
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
