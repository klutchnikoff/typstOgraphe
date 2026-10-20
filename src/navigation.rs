//! Correspondances immuables du rendu : octets Typst, caractères egui et points
//! de page. Un instantané n'est utilisable que pour sa révision de document.
use crate::{
    document::SourceMap,
    rendering::{TypstWorld, file_id},
};
use std::ops::Range;
use typst::{
    World,
    layout::{Abs, Frame, FrameItem, Point},
    syntax::SyntaxKind,
};
use typst_layout::PagedDocument;

#[derive(Clone, Copy, Debug)]
pub struct PagePoint {
    pub page: usize,
    /// Coordonnées Typst en points, indépendantes de l'échelle d'affichage.
    pub point: [f32; 2],
}

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
    pub fn new(revision: u64, world: TypstWorld, document: PagedDocument, map: SourceMap) -> Self {
        let mut anchors = Vec::new();
        let mut frames = Vec::new();
        for (page, output) in document.pages().iter().enumerate() {
            collect_anchors(&world, &output.frame, page, &mut anchors);
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
        let mut highlights: Vec<SelectionHighlight> = Vec::new();
        for anchor in &self.anchors {
            if anchor.source.is_empty()
                || anchor.source.start >= source.end
                || anchor.source.end <= source.start
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

fn collect_anchors(world: &TypstWorld, frame: &Frame, page: usize, anchors: &mut Vec<Anchor>) {
    for &(mut pos, ref item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                let start = anchors.len();
                collect_anchors(world, &group.frame, page, anchors);
                for anchor in &mut anchors[start..] {
                    anchor.point = pos + anchor.point.transform(group.transform);
                    anchor.corners = anchor
                        .corners
                        .map(|point| pos + point.transform(group.transform));
                }
            }
            FrameItem::Image(_, size, span) if span.id() == Some(file_id("contenu.typ")) => {
                if let Ok(source) = world.source(file_id("contenu.typ"))
                    && let Some(node) = source.find(*span)
                {
                    anchors.push(Anchor {
                        source: node.range(),
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
                for glyph in &text.glyphs {
                    let width = glyph.x_advance.at(text.size);
                    let (span, offset) = glyph.span;
                    if span.id() == Some(file_id("contenu.typ"))
                        && let Ok(source) = world.source(file_id("contenu.typ"))
                        && let Some(node) = source.find(span)
                    {
                        let range = node.range();
                        let source =
                            if matches!(node.kind(), SyntaxKind::Text | SyntaxKind::MathText) {
                                let start = (range.start + usize::from(offset)).min(range.end);
                                start..(start + glyph.range().len()).min(range.end)
                            } else {
                                range
                            };
                        anchors.push(Anchor {
                            source,
                            page,
                            point: Point::new(pos.x + width / 4.0, pos.y - text.size / 2.0),
                            corners: {
                                let metrics = text.font.metrics();
                                let top = pos.y - metrics.ascender.at(text.size);
                                let bottom = pos.y - metrics.descender.at(text.size);
                                [
                                    Point::new(pos.x, top),
                                    Point::new(pos.x + width, top),
                                    Point::new(pos.x + width, bottom),
                                    Point::new(pos.x, bottom),
                                ]
                            },
                        });
                    }
                    pos.x += width;
                }
            }
            _ => {}
        }
    }
}
