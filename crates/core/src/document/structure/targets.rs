//! Positions logiques : les indices sont des valeurs scalaires Unicode locales
//! à une zone, jamais les octets Typst ni les sentinelles du tampon graphique.
use super::*;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextPosition {
    pub zone_id: ZoneId,
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "position",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Position {
    Text(TextPosition),
    Before(BlockId),
    After(BlockId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogicalSelection {
    pub anchor: Position,
    pub head: Position,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetError {
    InvalidPosition,
    UnknownBlock,
    UnknownZone,
    ForeignDocument,
    StaleRevision,
}

#[derive(Clone, Debug)]
struct Placement {
    id: BlockId,
    image: bool,
    range: Range<usize>,
    zones: Vec<(ZoneId, Range<usize>)>,
}
#[derive(Clone, Debug)]
pub(crate) struct PositionIndex {
    placements: Vec<Placement>,
}
impl PositionIndex {
    pub fn selection_at(
        &self,
        anchor: usize,
        head: usize,
    ) -> Result<LogicalSelection, TargetError> {
        Ok(LogicalSelection {
            anchor: position_at(&self.placements, anchor)?,
            head: position_at(&self.placements, head)?,
        })
    }
    pub fn resolve_selection(
        &self,
        selection: LogicalSelection,
    ) -> Result<(usize, usize), TargetError> {
        Ok((
            resolve(&self.placements, selection.anchor)?,
            resolve(&self.placements, selection.head)?,
        ))
    }
}

impl Structure {
    pub fn selection_at(
        &self,
        anchor: usize,
        head: usize,
    ) -> Result<LogicalSelection, TargetError> {
        let placements = self.placements();
        Ok(LogicalSelection {
            anchor: position_at(&placements, anchor)?,
            head: position_at(&placements, head)?,
        })
    }

    /// Résout les deux extrémités sans tronquer ni réparer une cible périmée.
    pub fn resolve_selection(
        &self,
        selection: LogicalSelection,
    ) -> Result<(usize, usize), TargetError> {
        let placements = self.placements();
        Ok((
            resolve(&placements, selection.anchor)?,
            resolve(&placements, selection.head)?,
        ))
    }

    pub(crate) fn position_index(&self) -> PositionIndex {
        PositionIndex {
            placements: self.placements(),
        }
    }

    fn placements(&self) -> Vec<Placement> {
        let mut cursor = 0;
        self.blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                cursor += usize::from(index > 0);
                let start = cursor;
                let mut zones = Vec::new();
                match &block.content {
                    BlockContent::Text { body, .. } => place(&mut zones, &mut cursor, body),
                    BlockContent::Description { term, definition } => {
                        place(&mut zones, &mut cursor, term);
                        cursor += 1; // frontière du terme
                        place(&mut zones, &mut cursor, definition);
                    }
                    BlockContent::Quote { body, attribution } => {
                        place(&mut zones, &mut cursor, body);
                        cursor += 1; // frontière de l’attribution
                        place(&mut zones, &mut cursor, attribution);
                    }
                    BlockContent::Image { caption, .. } => {
                        cursor += 1; // objet graphique, sans offset textuel public
                        place(&mut zones, &mut cursor, caption);
                    }
                }
                Placement {
                    id: block.id,
                    image: matches!(block.content, BlockContent::Image { .. }),
                    range: start..cursor,
                    zones,
                }
            })
            .collect()
    }
}

fn place(zones: &mut Vec<(ZoneId, Range<usize>)>, cursor: &mut usize, zone: &TextZone) {
    let start = *cursor;
    *cursor += zone.view.len();
    zones.push((zone.id, start..*cursor));
}

fn position_at(placements: &[Placement], index: usize) -> Result<Position, TargetError> {
    for placement in placements {
        if index == placement.range.start && placement.image {
            return Ok(Position::Before(placement.id));
        }
        for (zone, range) in &placement.zones {
            if range.start <= index && index <= range.end {
                return Ok(Position::Text(TextPosition {
                    zone_id: *zone,
                    offset: index - range.start,
                }));
            }
        }
        if index == placement.range.end {
            return Ok(Position::After(placement.id));
        }
    }
    Err(TargetError::InvalidPosition)
}

fn resolve(placements: &[Placement], position: Position) -> Result<usize, TargetError> {
    match position {
        Position::Before(id) | Position::After(id) => {
            let placement = placements
                .iter()
                .find(|placement| placement.id == id)
                .ok_or(TargetError::UnknownBlock)?;
            Ok(if matches!(position, Position::Before(_)) {
                placement.range.start
            } else {
                placement.range.end
            })
        }
        Position::Text(position) => {
            let (_, range) = placements
                .iter()
                .flat_map(|placement| &placement.zones)
                .find(|(zone, _)| *zone == position.zone_id)
                .ok_or(TargetError::UnknownZone)?;
            if position.offset > range.len() {
                return Err(TargetError::InvalidPosition);
            }
            Ok(range.start + position.offset)
        }
    }
}
