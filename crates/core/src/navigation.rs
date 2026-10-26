//! Correspondances immuables du rendu : octets Typst, caractères egui et points
//! de page. Un instantané n'est utilisable que pour sa révision de document.
use crate::{
    document::SourceMap,
    rendering::{TypstWorld, file_id},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Range};
use typst::{
    World,
    layout::{Abs, Frame, FrameItem, Point},
    syntax::{Source, Span, SyntaxKind},
};
use typst_layout::PagedDocument;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PagePoint {
    pub page: usize,
    /// Coordonnées Typst en points, indépendantes de l'échelle d'affichage.
    pub point: [f32; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SelectionHighlight {
    pub page: usize,
    /// Coins dans l'ordre haut gauche, haut droit, bas droit, bas gauche,
    /// après les transformations du modèle, en points Typst.
    pub corners: [[f32; 2]; 4],
}

struct Anchor {
    source: Range<usize>,
    point: Point,
    page: usize,
    corners: [Point; 4],
}

pub struct RenderSync {
    pub revision: u64,
    world: TypstWorld,
    document: PagedDocument,
    map: SourceMap,
    anchors: Vec<Anchor>,
    frames: Vec<Frame>,
}

impl RenderSync {
    pub(crate) fn page_png(
        &self,
        index: usize,
    ) -> Result<Option<Vec<u8>>, crate::errors::CoreError> {
        crate::rendering::page_png(&self.document, index)
    }

    pub fn new(revision: u64, world: TypstWorld, document: PagedDocument, map: SourceMap) -> Self {
        let mut anchors = Vec::new();
        let mut frames = Vec::new();
        let mut source = AnchorSource {
            source: world.source(file_id("contenu.typ")).ok(),
            nodes: HashMap::new(),
        };
        for (page, output) in document.pages().iter().enumerate() {
            collect_anchors(&mut source, &output.frame, page, &mut anchors);
            let mut frame = output.frame.clone();
            // Le double-clic sur le texte d'un lien doit rejoindre l'éditeur.
            // Les annotations de lien sont conservées dans le PDF et le rendu.
            remove_links(&mut frame);
            frames.push(frame);
        }
        Self {
            revision,
            world,
            document,
            map,
            anchors,
            frames,
        }
    }

    pub fn page_count(&self) -> usize {
        self.frames.len()
    }

    pub fn page_size(&self, page: usize) -> Option<[f32; 2]> {
        let size = self.frames.get(page)?.size();
        Some([size.x.to_pt() as f32, size.y.to_pt() as f32])
    }

    pub fn forward(&self, character: usize) -> Option<PagePoint> {
        let byte = self.map.to_source(character);
        let anchor = self.anchors.iter().min_by_key(|anchor| {
            if byte < anchor.source.start {
                anchor.source.start - byte
            } else if byte >= anchor.source.end {
                byte.saturating_sub(anchor.source.end) + 1
            } else {
                0
            }
        })?;
        Some(PagePoint {
            page: anchor.page,
            point: [anchor.point.x.to_pt() as f32, anchor.point.y.to_pt() as f32],
        })
    }

    pub fn backward(&self, location: PagePoint) -> Option<usize> {
        let frame = self.frames.get(location.page)?;
        let point = Point::new(
            Abs::pt(location.point[0].into()),
            Abs::pt(location.point[1].into()),
        );
        match typst_ide::jump_from_click_in_frame(&self.world, &self.document, frame, point)? {
            typst_ide::Jump::File(id, byte) if id == file_id("contenu.typ") => {
                Some(self.map.to_visible(byte))
            }
            _ => None,
        }
    }

    pub fn selection_highlights(&self, characters: Range<usize>) -> Vec<SelectionHighlight> {
        let start = characters.start.min(self.map.characters.len());
        let end = characters.end.min(self.map.characters.len());
        if start >= end {
            return Vec::new();
        }
        let source = self.map.characters[start].start..self.map.characters[end - 1].end;
        self.highlights_for_sources(&[source])
    }

    /// Carte structurée : les annotations peuvent précéder leur corps dans le source.
    pub fn selection_highlights_exact(&self, characters: Range<usize>) -> Vec<SelectionHighlight> {
        let mut ranges = self.map.characters[characters.start.min(self.map.characters.len())
            ..characters
                .end
                .min(self.map.characters.len())
                .max(characters.start.min(self.map.characters.len()))]
            .to_vec();
        ranges.sort_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = vec![];
        for range in ranges {
            if range.is_empty() {
                continue;
            }
            if let Some(last) = merged.last_mut().filter(|last| range.start <= last.end) {
                last.end = last.end.max(range.end)
            } else {
                merged.push(range)
            }
        }
        self.highlights_for_sources(&merged)
    }
    fn highlights_for_sources(&self, sources: &[Range<usize>]) -> Vec<SelectionHighlight> {
        let mut highlights: Vec<SelectionHighlight> = Vec::new();
        for anchor in &self.anchors {
            let index = sources.partition_point(|range| range.end <= anchor.source.start);
            if anchor.source.is_empty()
                || sources
                    .get(index)
                    .is_none_or(|range| range.start >= anchor.source.end)
            {
                continue;
            }
            let corners = anchor
                .corners
                .map(|point| [point.x.to_pt() as f32, point.y.to_pt() as f32]);
            // Réunir les cellules contiguës d'une ligne évite les jointures
            // visibles et conserve les espaces justifiés dans la sélection.
            if let Some(previous) = highlights.last_mut()
                && previous.page == anchor.page
                && adjacent(previous.corners[1], corners[0])
                && adjacent(previous.corners[2], corners[3])
            {
                previous.corners[1] = corners[1];
                previous.corners[2] = corners[2];
            } else {
                highlights.push(SelectionHighlight {
                    page: anchor.page,
                    corners,
                });
            }
        }
        highlights
    }
}

fn adjacent(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
}

fn remove_links(frame: &mut Frame) {
    frame.retain(|item| {
        if let FrameItem::Group(group) = item {
            remove_links(&mut group.frame);
        }
        !matches!(item, FrameItem::Link(..))
    });
}

/// Many glyphs share a syntax span. Resolve each span once in this immutable
/// compilation, including failed lookups. Never reuse it across source revisions.
struct AnchorSource {
    source: Option<Source>,
    nodes: HashMap<Span, Option<(Range<usize>, SyntaxKind)>>,
}
impl AnchorSource {
    fn find(&mut self, span: Span) -> Option<(Range<usize>, SyntaxKind)> {
        let source = self.source.as_ref()?;
        if span.id() != Some(source.id()) {
            return None;
        }
        self.nodes
            .entry(span)
            .or_insert_with(|| source.find(span).map(|node| (node.range(), node.kind())))
            .clone()
    }
}

fn collect_anchors(
    source: &mut AnchorSource,
    frame: &Frame,
    page: usize,
    anchors: &mut Vec<Anchor>,
) {
    for &(mut pos, ref item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                let start = anchors.len();
                collect_anchors(source, &group.frame, page, anchors);
                for anchor in &mut anchors[start..] {
                    anchor.point = pos + anchor.point.transform(group.transform);
                    anchor.corners = anchor
                        .corners
                        .map(|point| pos + point.transform(group.transform));
                }
            }
            FrameItem::Image(_, size, span) => {
                if let Some((range, _)) = source.find(*span) {
                    anchors.push(Anchor {
                        source: range,
                        page,
                        point: pos + Point::new(size.x / 2.0, size.y / 2.0),
                        corners: [
                            pos,
                            pos + Point::new(size.x, Abs::zero()),
                            pos + size.to_point(),
                            pos + Point::new(Abs::zero(), size.y),
                        ],
                    });
                }
            }
            FrameItem::Text(text) => {
                let metrics = text.font.metrics();
                let top = pos.y - metrics.ascender.at(text.size);
                let bottom = pos.y - metrics.descender.at(text.size);
                for glyph in &text.glyphs {
                    let width = glyph.x_advance.at(text.size);
                    let (span, offset) = glyph.span;
                    if let Some((range, kind)) = source.find(span) {
                        let glyph_source =
                            if matches!(kind, SyntaxKind::Text | SyntaxKind::MathText) {
                                let start = (range.start + usize::from(offset)).min(range.end);
                                start..(start + glyph.range().len()).min(range.end)
                            } else {
                                range
                            };
                        anchors.push(Anchor {
                            source: glyph_source,
                            page,
                            point: Point::new(pos.x + width / 4.0, pos.y - text.size / 2.0),
                            corners: [
                                Point::new(pos.x, top),
                                Point::new(pos.x + width, top),
                                Point::new(pos.x + width, bottom),
                                Point::new(pos.x, bottom),
                            ],
                        });
                    }
                    pos.x += width;
                }
            }
            _ => {}
        }
    }
}
