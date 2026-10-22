//! Blocs et zones éditables reconstruits depuis la projection Typst.
//!
//! Le tampon plat reste l’adaptateur de saisie pendant la migration. Ses ancres
//! suivent les éditions, sans rapprochement par le texte ni par les indices.
//! Les identités ne sont jamais sérialisées et ne participent pas à l’état modifié.
use super::{BlockKind, ImageBlock, InlineStyle, Projection};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicU64, Ordering},
};

#[path = "structure/build.rs"]
mod build;
#[path = "structure/flatten.rs"]
mod flatten;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "structure/tests.rs"]
mod tests;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn fresh_id() -> u64 {
    NEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("document identity space exhausted")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LineId(u64);
impl LineId {
    pub(super) fn new() -> Self {
        Self(fresh_id())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockId(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ZoneId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ZoneRole {
    Body,
    Term,
    Definition,
    Attribution,
    Caption,
}

#[derive(Clone, Debug)]
pub struct TextZone {
    pub id: ZoneId,
    pub role: ZoneRole,
    /// Projection locale : caractères, styles et retours simples uniquement.
    pub view: Projection,
}

#[derive(Clone, Debug)]
pub enum BlockContent {
    Text {
        kind: BlockKind,
        body: TextZone,
    },
    Description {
        term: TextZone,
        definition: TextZone,
    },
    Quote {
        body: TextZone,
        attribution: TextZone,
    },
    Image {
        resource: u64,
        image: ImageBlock,
        caption: TextZone,
    },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: BlockId,
    pub content: BlockContent,
    lines: Vec<LineId>,
}

#[derive(Clone, Debug, Default)]
pub struct Structure {
    pub blocks: Vec<Block>,
    typing_style: InlineStyle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockFamily {
    Text,
    Postscript,
    Attachment,
    Description,
    Quote,
    Image,
}

impl BlockContent {
    fn family(&self) -> BlockFamily {
        match self {
            Self::Text {
                kind: BlockKind::Postscript,
                ..
            } => BlockFamily::Postscript,
            Self::Text {
                kind: BlockKind::Attachment,
                ..
            } => BlockFamily::Attachment,
            Self::Text { .. } => BlockFamily::Text,
            Self::Description { .. } => BlockFamily::Description,
            Self::Quote { .. } => BlockFamily::Quote,
            Self::Image { .. } => BlockFamily::Image,
        }
    }

    fn zones(&self) -> Vec<&TextZone> {
        match self {
            Self::Text { body, .. } => vec![body],
            Self::Description { term, definition } => vec![term, definition],
            Self::Quote { body, attribution } => vec![body, attribution],
            Self::Image { caption, .. } => vec![caption],
        }
    }
}

impl Structure {
    pub fn rebuild(&mut self, view: &Projection) {
        let mut used = BTreeSet::new();
        let mut used_zones = BTreeSet::new();
        let mut next = Vec::new();
        let owners: BTreeMap<_, _> = self
            .blocks
            .iter()
            .flat_map(|block| block.lines.iter().map(move |line| (*line, block)))
            .collect();
        let zone_owners: BTreeMap<_, _> = self
            .blocks
            .iter()
            .flat_map(|block| {
                block.content.zones().into_iter().flat_map(|zone| {
                    zone.view
                        .line_ids
                        .iter()
                        .map(move |line| ((zone.role, *line), zone))
                })
            })
            .collect();
        for mut block in build::blocks(view) {
            let previous = block
                .lines
                .iter()
                .filter_map(|line| owners.get(line))
                .find(|old| {
                    !used.contains(&old.id) && old.content.family() == block.content.family()
                });
            if let Some(previous) = previous {
                block.id = previous.id;
            }
            for zone in block.zones_mut() {
                if let Some(old) = zone
                    .view
                    .line_ids
                    .iter()
                    .filter_map(|line| zone_owners.get(&(zone.role, *line)))
                    .find(|old| !used_zones.contains(&old.id))
                {
                    zone.id = old.id;
                }
                used_zones.insert(zone.id);
            }
            used.insert(block.id);
            next.push(block);
        }
        self.blocks = next;
        self.typing_style = view.typing_style.clone();
    }
}

impl Block {
    fn zones_mut(&mut self) -> Vec<&mut TextZone> {
        match &mut self.content {
            BlockContent::Text { body, .. } => vec![body],
            BlockContent::Description { term, definition } => vec![term, definition],
            BlockContent::Quote { body, attribution } => vec![body, attribution],
            BlockContent::Image { caption, .. } => vec![caption],
        }
    }
}
