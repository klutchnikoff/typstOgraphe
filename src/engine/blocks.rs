//! Règles de navigation et de scission des structures simples.
use super::*;
use crate::{document::BlockKind, session::Caret};
use std::ops::Range;

impl Engine {
    pub(super) fn relative_destination(
        &self,
        target: TextTarget,
        part: SelectionPart,
    ) -> Result<Selection, CommandError> {
        if matches!(part, SelectionPart::Body | SelectionPart::Annotation) {
            return self.annotation_destination(target, part);
        }
        let selection = self.resolve(target)?;
        let (row, separator) = self.description_row(selection)?;
        let caret = Caret {
            index: if part == SelectionPart::Term {
                row.start
            } else {
                separator + 1
            },
            prefer_next_row: false,
        };
        Ok(Selection {
            primary: caret,
            secondary: caret,
        })
    }

    pub(super) fn description_row(
        &self,
        selection: Selection,
    ) -> Result<(Range<usize>, usize), CommandError> {
        let view = &self.session.document.view;
        let range = selection.range();
        let line = view.line_at(range.start);
        let row = view.line_ranges()[line].clone();
        if view.blocks[line] != BlockKind::Description || range.end > row.end {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "description_required",
            ));
        }
        let separator = view
            .text
            .chars()
            .skip(row.start)
            .take(row.len())
            .position(|c| c == '\t')
            .ok_or_else(|| CommandError::new(ErrorCode::Document, "invalid_projection"))?;
        Ok((row.clone(), row.start + separator))
    }

    pub(super) fn check_block_split(&self, target: TextTarget) -> Result<(), CommandError> {
        let selection = self.resolve(target)?;
        let view = &self.session.document.view;
        if view.annotation_at(selection.range()).is_some() {
            return self.check_annotation_split(target);
        }
        let kind = view.blocks[view.line_at(selection.range().start)];
        if !matches!(
            kind,
            BlockKind::Description | BlockKind::Bullet | BlockKind::Numbered
        ) && !kind.is_letter_note()
        {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "structured_block_required",
            ));
        }
        view.validate_deletion(&selection.range())?;
        Ok(())
    }

    pub(super) fn check_block_exit(&self, target: TextTarget) -> Result<(), CommandError> {
        let selection = self.resolve(target)?;
        if self
            .session
            .document
            .view
            .annotation_at(selection.range())
            .is_some()
        {
            self.resolve_annotation(target)?;
        } else {
            self.note_end(selection)?;
        }
        Ok(())
    }

    fn note_end(&self, selection: Selection) -> Result<usize, CommandError> {
        let view = &self.session.document.view;
        let range = selection.range();
        let line = view.line_at(range.start);
        let group = view
            .letter_note_groups()
            .into_iter()
            .find(|(kind, rows)| kind.is_letter_note() && rows.contains(&line))
            .ok_or_else(|| {
                CommandError::new(ErrorCode::Unavailable, "structured_block_required")
            })?;
        let end = view.line_ranges()[group.1.end - 1].end;
        if range.end > end {
            return Err(CommandError::new(ErrorCode::Document, "protected_boundary"));
        }
        Ok(end)
    }

    pub(super) fn block_split_reveals(&self, target: TextTarget) -> Result<bool, CommandError> {
        let selection = self.resolve(target)?;
        let view = &self.session.document.view;
        if view.annotation_at(selection.range()).is_some() {
            return self.annotation_split_exits(target);
        }
        let line = view.line_at(selection.range().start);
        Ok(view.blocks[line].is_letter_note()
            && selection.range().is_empty()
            && view.line_ranges()[line].is_empty())
    }

    pub(super) fn exit_block(&mut self, target: TextTarget) -> Result<(), CommandError> {
        let selection = self.resolve(target)?;
        if self
            .session
            .document
            .view
            .annotation_at(selection.range())
            .is_some()
        {
            return self.exit_annotation(target);
        }
        let end = self.note_end(selection)?;
        let mut view = self.session.document.view.clone();
        view.insert_text("\n", end);
        let line = view.line_at(end + 1);
        view.blocks[line] = BlockKind::Paragraph;
        self.publish_block_edit(selection, view, end + 1, true)
    }

    pub(super) fn split_block(&mut self, target: TextTarget) -> Result<(), CommandError> {
        let selection = self.resolve(target)?;
        let range = selection.range();
        if self
            .session
            .document
            .view
            .annotation_at(range.clone())
            .is_some()
        {
            return self.split_annotation(target);
        }
        let mut view = self.session.document.view.clone();
        let line = view.line_at(range.start);
        let row = view.line_ranges()[line].clone();
        let kind = view.blocks[line];
        let empty = if kind == BlockKind::Description {
            view.text
                .chars()
                .skip(row.start)
                .take(row.len())
                .all(char::is_whitespace)
        } else {
            row.is_empty()
        };
        let at = if range.is_empty() && empty {
            if kind == BlockKind::Description {
                view.delete_char_range(row.clone());
            }
            view.set_block(row.start..row.start, BlockKind::Paragraph);
            row.start
        } else {
            let in_definition = kind == BlockKind::Description
                && self
                    .description_row(selection)
                    .is_ok_and(|(_, separator)| range.start > separator);
            if !range.is_empty() {
                view.typing_style = view.style_at(range.start);
            }
            view.delete_char_range(range.clone());
            view.insert_text(if in_definition { "\n\t" } else { "\n" }, range.start);
            range.start + 1
        };
        self.publish_block_edit(selection, view, at, kind.is_letter_note() && empty)
    }
}
