use super::*;
use std::ops::Range;

pub(super) fn blocks(view: &Projection) -> Vec<Block> {
    let ranges = view.line_ranges();
    let mut blocks = Vec::new();
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
        let content = match kind {
            BlockKind::Description => description(view, body),
            BlockKind::Quote => {
                let attribution = if view.blocks.get(line) == Some(&BlockKind::QuoteAttribution) {
                    line += 1;
                    ranges[line - 1].clone()
                } else {
                    body.end..body.end
                };
                BlockContent::Quote {
                    body: zone(view, ZoneRole::Body, body),
                    attribution: zone(view, ZoneRole::Attribution, attribution),
                }
            }
            BlockKind::Image(resource) => BlockContent::Image {
                resource,
                image: view.images[&resource].clone(),
                caption: zone(view, ZoneRole::Caption, body.start + 1..body.end),
            },
            _ => BlockContent::Text {
                kind,
                body: zone(view, ZoneRole::Body, body),
            },
        };
        blocks.push(Block {
            id: BlockId(fresh_id()),
            content,
            lines: view.line_ids[start..line].to_vec(),
        });
    }
    blocks
}

fn description(view: &Projection, body: Range<usize>) -> BlockContent {
    let separator = view
        .text
        .chars()
        .skip(body.start)
        .take(body.len())
        .position(|c| c == '\t');
    let at = separator.map_or(body.end, |at| body.start + at);
    BlockContent::Description {
        term: zone(view, ZoneRole::Term, body.start..at),
        definition: zone(
            view,
            ZoneRole::Definition,
            (at + usize::from(separator.is_some()))..body.end,
        ),
    }
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
