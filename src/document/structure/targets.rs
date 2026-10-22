//! Positions logiques : les indices sont des valeurs scalaires Unicode locales
//! à une zone, jamais les octets Typst ni les sentinelles du tampon graphique.
use super::*;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextPosition {
    pub zone_id: ZoneId,
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Text(TextPosition),
    Before(BlockId),
    After(BlockId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

struct Placement<'a> {
    block: &'a Block,
    range: Range<usize>,
    zones: Vec<(&'a TextZone, Range<usize>)>,
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

    fn placements(&self) -> Vec<Placement<'_>> {
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
                    block,
                    range: start..cursor,
                    zones,
                }
            })
            .collect()
    }
}

fn place<'a>(
    zones: &mut Vec<(&'a TextZone, Range<usize>)>,
    cursor: &mut usize,
    zone: &'a TextZone,
) {
    let start = *cursor;
    *cursor += zone.view.len();
    zones.push((zone, start..*cursor));
}

fn position_at(placements: &[Placement<'_>], index: usize) -> Result<Position, TargetError> {
    for placement in placements {
        if index == placement.range.start
            && matches!(placement.block.content, BlockContent::Image { .. })
        {
            return Ok(Position::Before(placement.block.id));
        }
        for (zone, range) in &placement.zones {
            if range.start <= index && index <= range.end {
                return Ok(Position::Text(TextPosition {
                    zone_id: zone.id,
                    offset: index - range.start,
                }));
            }
        }
        if index == placement.range.end {
            return Ok(Position::After(placement.block.id));
        }
    }
    Err(TargetError::InvalidPosition)
}

fn resolve(placements: &[Placement<'_>], position: Position) -> Result<usize, TargetError> {
    match position {
        Position::Before(id) | Position::After(id) => {
            let placement = placements
                .iter()
                .find(|placement| placement.block.id == id)
                .ok_or(TargetError::UnknownBlock)?;
            Ok(if matches!(position, Position::Before(_)) {
                placement.range.start
            } else {
                placement.range.end
            })
        }
        Position::Text(position) => {
            let (zone, range) = placements
                .iter()
                .flat_map(|placement| &placement.zones)
                .find(|(zone, _)| zone.id == position.zone_id)
                .ok_or(TargetError::UnknownZone)?;
            if position.offset > zone.view.len() {
                return Err(TargetError::InvalidPosition);
            }
            Ok(range.start + position.offset)
        }
    }
}
