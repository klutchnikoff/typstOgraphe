//! Validation des frontières avant toute normalisation ou publication.
use super::{
    BlockKind, IMAGE_OBJECT, Projection,
    structure::{ZoneRole, layout},
};
use std::{collections::BTreeSet, ops::Range};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidSelection,
    ProtectedBoundary,
    InvalidText,
    InvalidProjection,
    StaleDraft,
    Target(super::structure::TargetError),
}

fn ordinary(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Paragraph | BlockKind::Heading(_) | BlockKind::Bullet | BlockKind::Numbered
    )
}

fn covers(range: &Range<usize>, block: &Range<usize>) -> bool {
    range.start <= block.start && block.end <= range.end
}

impl Projection {
    pub fn validate_deletion(&self, range: &Range<usize>) -> Result<(), EditError> {
        self.validate_edit()?;
        if range.start > range.end || range.end > self.len() {
            return Err(EditError::InvalidSelection);
        }
        if range.is_empty() {
            return Ok(());
        }
        let blocks = layout(self);
        if blocks
            .iter()
            .flat_map(|block| &block.zones)
            .any(|(_, zone)| zone.start <= range.start && range.end <= zone.end)
        {
            return Ok(());
        }
        for block in &blocks {
            if range.start < block.range.end
                && block.range.start < range.end
                && !ordinary(block.kind)
                && !covers(range, &block.range)
            {
                return Err(EditError::ProtectedBoundary);
            }
        }
        for pair in blocks.windows(2) {
            let separator = pair[0].range.end;
            if range.start <= separator && separator < range.end {
                for block in pair {
                    if !ordinary(block.kind) && !covers(range, &block.range) {
                        return Err(EditError::ProtectedBoundary);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn validate_text_input(&self, at: usize, text: &str) -> Result<(), EditError> {
        self.validate_edit()?;
        if at > self.len() {
            return Err(EditError::InvalidSelection);
        }
        if text.contains(IMAGE_OBJECT) {
            return Err(EditError::InvalidText);
        }
        let blocks = layout(self);
        let restricted = blocks
            .iter()
            .flat_map(|block| &block.zones)
            .any(|(role, range)| {
                range.start <= at
                    && at <= range.end
                    && matches!(
                        role,
                        ZoneRole::Term
                            | ZoneRole::Definition
                            | ZoneRole::Caption
                            | ZoneRole::Attribution
                    )
            });
        if restricted && text.contains(['\t', '\n', '\r']) {
            return Err(EditError::InvalidText);
        }
        Ok(())
    }

    pub fn validate_object_insertion(&self, range: &Range<usize>) -> Result<(), EditError> {
        self.validate_deletion(range)?;
        for block in layout(self) {
            let intersects = if range.is_empty() {
                block.range.start < range.start && range.start < block.range.end
            } else {
                range.start < block.range.end && block.range.start < range.end
            };
            if intersects && !ordinary(block.kind) && !covers(range, &block.range) {
                return Err(EditError::ProtectedBoundary);
            }
        }
        Ok(())
    }

    pub(super) fn prepare_deletion(&mut self, range: &Range<usize>) -> bool {
        if self.edit_error.is_some() {
            return false;
        }
        if let Err(error) = self.validate_deletion(range) {
            self.edit_error = Some(error);
            return false;
        }
        if !range.is_empty() {
            if range.start == 0
                && range.end == self.len()
                && !(self.blocks.len() == 1 && ordinary(self.blocks[0]))
            {
                self.blocks.fill(BlockKind::Paragraph);
            } else {
                for block in layout(self) {
                    if !ordinary(block.kind) && covers(range, &block.range) {
                        self.blocks[block.lines].fill(BlockKind::Paragraph);
                    }
                }
            }
        }
        true
    }

    pub fn validate_edit(&self) -> Result<(), EditError> {
        if let Some(error) = self.edit_error {
            return Err(error);
        }
        let ranges = self.line_ranges();
        if self.text.chars().count() != self.styles.len()
            || ranges.len() != self.blocks.len()
            || self.line_ids.len() != self.blocks.len()
        {
            return Err(EditError::InvalidProjection);
        }
        let characters: Vec<_> = self.text.chars().collect();
        let mut images = BTreeSet::new();
        for (line, range) in ranges.iter().enumerate() {
            if let BlockKind::Image(id) = self.blocks[line]
                && (characters.get(range.start) != Some(&IMAGE_OBJECT)
                    || !self.images.contains_key(&id)
                    || !images.insert(id))
            {
                return Err(EditError::InvalidProjection);
            }
            for at in range.clone() {
                if characters[at] == IMAGE_OBJECT
                    && !(at == range.start && matches!(self.blocks[line], BlockKind::Image(_)))
                {
                    return Err(EditError::InvalidProjection);
                }
            }
        }
        Ok(())
    }
}

/// Tampon de texte visible, partagé par l'adaptateur egui et les remplacements
/// explicites. Une première erreur condamne tout le brouillon, pas son seul geste.
pub struct TextInput {
    pub view: Projection,
}
impl TextInput {
    pub fn insert_text(&mut self, text: &str, at: usize) -> usize {
        if self.view.edit_error.is_some() {
            return 0;
        }
        if let Err(error) = self.view.validate_text_input(at, text) {
            self.view.edit_error = Some(error);
            return 0;
        }
        self.view.insert_text(text, at)
    }
    pub fn delete_char_range(&mut self, range: Range<usize>) {
        if !range.is_empty() && range.start < self.view.len() {
            self.view.typing_style = self.view.style_at(range.start);
        }
        self.view.delete_char_range(range);
    }
    pub fn replace_with(&mut self, text: &str) {
        self.delete_char_range(0..self.view.len());
        self.insert_text(text, 0);
    }
}
