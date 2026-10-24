//! Navigation et Entrée dans les structures à annotation facultative.
//! La géométrie, le focus et les événements clavier restent dans l’adaptateur.
use super::*;
use crate::{annotations::OptionalAnnotation, document::BlockKind, session::Caret};

impl Engine {
    pub(super) fn resolve_annotation(
        &self,
        target: TextTarget,
    ) -> Result<(Selection, OptionalAnnotation), CommandError> {
        let selection = self.resolve(target)?;
        let annotation = self
            .session
            .document
            .view
            .annotation_at(selection.range())
            .ok_or_else(|| CommandError::new(ErrorCode::Unavailable, "annotation_required"))?;
        Ok((selection, annotation))
    }

    pub(super) fn annotation_destination(
        &self,
        target: TextTarget,
        part: SelectionPart,
    ) -> Result<Selection, CommandError> {
        let (_, annotation) = self.resolve_annotation(target)?;
        let range = match part {
            SelectionPart::Body => annotation.main_target(),
            SelectionPart::Annotation => annotation.content.start..annotation.content.start,
            _ => unreachable!("annotation part"),
        };
        Ok(Selection {
            secondary: Caret {
                index: range.start,
                prefer_next_row: true,
            },
            primary: Caret {
                index: range.end,
                prefer_next_row: true,
            },
        })
    }

    pub(super) fn check_annotation_split(&self, target: TextTarget) -> Result<(), CommandError> {
        let (selection, annotation) = self.resolve_annotation(target)?;
        let range = selection.range();
        if !annotation.enter_leaves(&range) {
            // Une sélection partielle ne doit jamais absorber la frontière de
            // l’attribution. La couverture de toute la citation reste remplaçable.
            if range.end > annotation.content.start.saturating_sub(1)
                && range != annotation.structure
            {
                return Err(CommandError::new(ErrorCode::Document, "protected_boundary"));
            }
            self.session.document.view.validate_deletion(&range)?;
        }
        Ok(())
    }

    pub(super) fn annotation_split_exits(&self, target: TextTarget) -> Result<bool, CommandError> {
        let (selection, annotation) = self.resolve_annotation(target)?;
        let range = selection.range();
        let view = &self.session.document.view;
        Ok(annotation.enter_leaves(&range)
            || (range.is_empty() && view.line_ranges()[view.line_at(range.start)].is_empty()))
    }

    pub(super) fn exit_annotation(&mut self, target: TextTarget) -> Result<(), CommandError> {
        let (selection, annotation) = self.resolve_annotation(target)?;
        let mut view = self.session.document.view.clone();
        let at = view.leave_annotation(&annotation);
        self.publish_block_edit(selection, view, at, true)
    }

    pub(super) fn split_annotation(&mut self, target: TextTarget) -> Result<(), CommandError> {
        let (selection, annotation) = self.resolve_annotation(target)?;
        let range = selection.range();
        if annotation.enter_leaves(&range) {
            return self.exit_annotation(target);
        }
        let mut view = self.session.document.view.clone();
        let line = view.line_at(range.start);
        let lines = view.line_ranges();
        let start = lines[line].start;
        let empty_line = range.is_empty() && lines[line].is_empty();
        let at = if empty_line {
            let group = view
                .quote_groups()
                .into_iter()
                .find(|group| group.body.contains(&line))
                .expect("validated quotation body");
            let anchor = if group.body.len() > 1 {
                view.delete_char_range(start - 1..start);
                view.blocks[group.attribution.expect("editable attribution") - 1] =
                    BlockKind::QuoteAttribution;
                start - 1
            } else {
                start
            };
            let annotation = view
                .annotation_at(anchor..anchor)
                .expect("quotation annotation");
            view.leave_annotation(&annotation)
        } else {
            view.delete_char_range(range.clone());
            view.insert_text("\n", range.start);
            range.start + 1
        };
        self.publish_block_edit(selection, view, at, empty_line)
    }

    pub(super) fn publish_block_edit(
        &mut self,
        before_selection: Selection,
        view: Projection,
        at: usize,
        prefer_next_row: bool,
    ) -> Result<(), CommandError> {
        let caret = Caret {
            index: at,
            prefer_next_row,
        };
        self.commit_projection(
            self.session.target_guard(),
            self.session.snapshot(before_selection),
            view,
            Selection {
                primary: caret,
                secondary: caret,
            },
            EditGroup::Discrete,
        )?;
        self.session.typing_group = false;
        Ok(())
    }
}
