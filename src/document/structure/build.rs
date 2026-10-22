use super::*;
use std::ops::Range;

pub(crate) struct Layout {
    pub kind: BlockKind,
    pub lines: Range<usize>,
    pub range: Range<usize>,
    pub zones: Vec<(ZoneRole, Range<usize>)>,
}

/// Même découpage pour les blocs et la validation, y compris un brouillon qui
/// n'a pas encore reçu ses séparateurs facultatifs de normalisation.
pub(crate) fn layout(view: &Projection) -> Vec<Layout> {
    let ranges = view.line_ranges();
    let mut result = Vec::new();
    let mut line = 0;
    while line < ranges.len() {
        let start = line;
        let kind = view.blocks[line];
        line += 1;
        if matches!(kind, BlockKind::Quote | BlockKind::Postscript) {
            while view.blocks.get(line) == Some(&kind) {
                line += 1;
            }
        } else if kind == BlockKind::Paragraph && !ranges[start].is_empty() {
            while view.blocks.get(line) == Some(&kind) && !ranges[line].is_empty() {
                line += 1;
            }
        }
        let body = ranges[start].start..ranges[line - 1].end;
        let zones = match kind {
            BlockKind::Description => {
                let separator = view
                    .text
                    .chars()
                    .skip(body.start)
                    .take(body.len())
                    .position(|c| c == '\t');
                let at = separator.map_or(body.end, |at| body.start + at);
                vec![
                    (ZoneRole::Term, body.start..at),
                    (
                        ZoneRole::Definition,
                        (at + usize::from(separator.is_some()))..body.end,
                    ),
                ]
            }
            BlockKind::Quote => {
                let attribution = if view.blocks.get(line) == Some(&BlockKind::QuoteAttribution) {
                    line += 1;
                    ranges[line - 1].clone()
                } else {
                    body.end..body.end
                };
                vec![
                    (ZoneRole::Body, body.clone()),
                    (ZoneRole::Attribution, attribution),
                ]
            }
            BlockKind::Image(_) => vec![(ZoneRole::Caption, body.start + 1..body.end)],
            _ => vec![(ZoneRole::Body, body)],
        };
        result.push(Layout {
            kind,
            lines: start..line,
            range: ranges[start].start..ranges[line - 1].end,
            zones,
        });
    }
    result
}

pub(super) fn blocks(view: &Projection) -> Vec<Block> {
    layout(view)
        .into_iter()
        .map(|item| {
            let mut zones = item
                .zones
                .into_iter()
                .map(|(role, range)| zone(view, role, range));
            let first = zones.next().expect("block text zone");
            let content = match item.kind {
                BlockKind::Description => BlockContent::Description {
                    term: first,
                    definition: zones.next().unwrap(),
                },
                BlockKind::Quote => BlockContent::Quote {
                    body: first,
                    attribution: zones.next().unwrap(),
                },
                BlockKind::Image(resource) => BlockContent::Image {
                    resource,
                    image: view.images[&resource].clone(),
                    caption: first,
                },
                kind => BlockContent::Text { kind, body: first },
            };
            Block {
                id: BlockId(fresh_id()),
                content,
                lines: view.line_ids[item.lines].to_vec(),
            }
        })
        .collect()
}

fn zone(view: &Projection, role: ZoneRole, range: Range<usize>) -> TextZone {
    let text = view.text[view.byte_index(range.start)..view.byte_index(range.end)].to_owned();
    let lines = text.matches('\n').count() + 1;
    TextZone {
        id: ZoneId(fresh_id()),
        role,
        view: Projection {
            text,
            styles: view.styles[range.clone()].to_vec(),
            blocks: vec![BlockKind::Paragraph; lines],
            line_ids: view.line_ids[view.line_at(range.start)..=view.line_at(range.end)].to_vec(),
            ..Projection::default()
        },
    }
}
