use super::*;
use crate::{
    document::{BlockKind, Mark},
    document_types::DocumentType,
    session::Caret,
};

impl Engine {
    pub(super) fn check_guard(&self, guard: TargetGuard) -> Result<(), CommandError> {
        let current = self.session.target_guard();
        if guard.document_id != current.document_id {
            return Err(CommandError::new(ErrorCode::Document, "foreign_document"));
        }
        if guard.revision != current.revision {
            return Err(CommandError::new(ErrorCode::Document, "stale_revision"));
        }
        Ok(())
    }

    pub(super) fn resolve(&self, target: TextTarget) -> Result<Selection, CommandError> {
        let selection = match target {
            TextTarget::Current {} => self.selection,
            TextTarget::All {} => Selection {
                primary: Caret {
                    index: self.session.document.view.len(),
                    ..Default::default()
                },
                secondary: Caret::default(),
            },
            TextTarget::Selection { guard, selection } => {
                let (anchor, head) = self
                    .session
                    .resolve_target(SelectionTarget { guard, selection })?;
                Selection {
                    primary: Caret {
                        index: head,
                        ..Default::default()
                    },
                    secondary: Caret {
                        index: anchor,
                        ..Default::default()
                    },
                }
            }
        };
        if selection.range().end > self.session.document.view.len() {
            return Err(CommandError::new(ErrorCode::Document, "invalid_selection"));
        }
        Ok(selection)
    }

    pub fn available(&self, command: &Command) -> Availability {
        match self.check(command) {
            Ok(()) => Availability::Enabled,
            Err(reason) => Availability::Disabled { reason },
        }
    }
    fn check(&self, command: &Command) -> Result<(), CommandError> {
        if let Some(target) = command.target() {
            self.resolve(target)?;
        }
        self.check_search(command)?;
        self.check_spelling(command)?;
        self.check_metadata(command)?;
        let rule = Command::command_specs()
            .iter()
            .find(|spec| spec.id.as_str() == command.id())
            .expect("registered command")
            .availability;
        if rule == AvailabilityRule::Letter
            && self.session.metadata.document_type() != DocumentType::Courrier
        {
            return Err(CommandError::new(ErrorCode::Unavailable, "document_type"));
        }
        if (rule == AvailabilityRule::Undo && self.session.undo.is_empty())
            || (rule == AvailabilityRule::Redo && self.session.redo.is_empty())
        {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                if rule == AvailabilityRule::Undo {
                    "undo_empty"
                } else {
                    "redo_empty"
                },
            ));
        }
        let reason = match command {
            Command::InsertImage(params) => {
                self.session
                    .document
                    .view
                    .validate_object_insertion(&self.resolve(params.target)?.range())?;
                if params
                    .caption
                    .contains(['\n', '\r', '\t', crate::document::IMAGE_OBJECT])
                {
                    return Err(CommandError::new(ErrorCode::InvalidParams, "invalid_text"));
                }
                return Ok(());
            }
            Command::ReplaceImage(ReplaceImageParams { target, .. })
            | Command::RemoveImage(ImageTargetParams { target })
            | Command::SetImageAlt(ImageAltParams { target, .. }) => {
                self.resolve_image(*target)?;
                return Ok(());
            }
            Command::SetSignature(params) => {
                self.check_signature(params)?;
                return Ok(());
            }
            Command::SelectText(params) if params.part.is_some() => {
                self.relative_destination(params.target, params.part.unwrap())?;
                return Ok(());
            }
            Command::SplitBlock(params) => {
                self.check_block_split(params.target)?;
                return Ok(());
            }
            Command::ExitBlock(params) => {
                self.check_block_exit(params.target)?;
                return Ok(());
            }
            Command::SetLink(params) => {
                let selection = self.resolve(params.target)?;
                if selection.range().is_empty() {
                    "select_link"
                } else if !self
                    .session
                    .document
                    .view
                    .is_single_paragraph(selection.range())
                {
                    "link_one_paragraph"
                } else {
                    return Ok(());
                }
            }
            Command::RemoveLink(params) if self.resolve(params.target)?.range().is_empty() => {
                "select_link_remove"
            }
            _ => return Ok(()),
        };
        Err(CommandError::new(ErrorCode::Unavailable, reason))
    }

    pub fn run(&mut self, command: Command) -> Result<Outcome, CommandError> {
        self.check(&command)?;
        let reveal = match &command {
            Command::InsertImage(_)
            | Command::ReplaceImage(_)
            | Command::FindNext(_)
            | Command::FindPrevious(_)
            | Command::NextSpelling(_)
            | Command::PreviousSpelling(_)
            | Command::ReplaceCurrent(_) => true,
            Command::SplitBlock(params) => self.block_split_reveals(params.target)?,
            Command::ExitBlock(_) => true,
            Command::SelectText(SelectTextParams {
                part: Some(SelectionPart::Body | SelectionPart::Annotation),
                ..
            }) => true,
            _ => false,
        };
        let revision = self.session.revision;
        let value = self.execute(command)?;
        let changed = revision != self.session.revision;
        Ok(Outcome {
            changed,
            guard: self.session.target_guard(),
            selection: self.session.selection_target(self.selection).ok(),
            effects: [
                changed.then_some(Effect::Render),
                reveal.then_some(Effect::RevealSelection),
            ]
            .into_iter()
            .flatten()
            .collect(),
            value,
        })
    }

    fn execute(&mut self, command: Command) -> Result<Option<Value>, CommandError> {
        match command {
            Command::SetMetadata(p) => self.set_metadata(p)?,
            Command::SetOption(p) => self.set_option(p)?,
            Command::NextSpelling(p) => self.navigate_spelling(p, false)?,
            Command::PreviousSpelling(p) => self.navigate_spelling(p, true)?,
            Command::CorrectSpelling(p) => self.correct_spelling(p)?,
            Command::IgnoreSpelling(p) => self.ignore_spelling(p)?,
            Command::ResetSpelling(_) => self.reset_spelling()?,
            Command::FindNext(params) => return self.navigate_search(params, false).map(Some),
            Command::FindPrevious(params) => return self.navigate_search(params, true).map(Some),
            Command::ReplaceCurrent(params) => {
                return self.replace_search_current(params).map(Some);
            }
            Command::ReplaceAll(params) => return self.replace_search_all(params).map(Some),
            Command::InsertImage(params) => self.insert_image(params)?,
            Command::ReplaceImage(params) => self.replace_image(params)?,
            Command::RemoveImage(params) => self.remove_image(params)?,
            Command::SetImageAlt(params) => self.set_image_alt(params)?,
            Command::SetSignature(params) => self.set_signature(params)?,
            Command::SelectText(params) => {
                let selection = if let Some(part) = params.part {
                    self.relative_destination(params.target, part)?
                } else {
                    self.resolve(params.target)?
                };
                self.select_resolved(selection);
            }
            Command::SplitBlock(params) => self.split_block(params.target)?,
            Command::ExitBlock(params) => self.exit_block(params.target)?,
            Command::InsertText(params) => {
                if params.input_rules == InputRules::Typing && self.transform_input(&params)? {
                    return Ok(None);
                }
                let selection = self.resolve(params.target)?;
                let target = self.session.selection_target(selection)?;
                self.selection = self.session.replace_text(target, &params.text)?;
            }
            Command::DeleteText(params) => {
                self.execute(Command::InsertText(InsertTextParams {
                    target: params.target,
                    text: String::new(),
                    input_rules: InputRules::Literal,
                }))?;
            }
            Command::Bold(params) => self.mark(params, Mark::Bold)?,
            Command::Italic(params) => self.mark(params, Mark::Italic)?,
            Command::Clear(params) => {
                self.edit_target(params.target, |view, range| view.clear_marks(range))?
            }
            Command::SetLink(params) => {
                let url = params.url.trim();
                if !(url.starts_with("https://")
                    || url.starts_with("http://")
                    || url.starts_with("mailto:"))
                    || url.chars().any(char::is_whitespace)
                {
                    return Err(CommandError::new(ErrorCode::InvalidParams, "invalid_url"));
                }
                self.edit_target(params.target, |view, range| {
                    view.set_link(range, Some(url.into()))
                })?;
            }
            Command::RemoveLink(params) => {
                self.edit_target(params.target, |view, range| view.set_link(range, None))?
            }
            Command::Undo(_) | Command::Redo(_) => {
                self.selection = self
                    .session
                    .history(matches!(command, Command::Undo(_)), self.selection)
                    .expect("history availability checked");
            }
            Command::Postscript(_) | Command::Attachment(_) => {
                let kind = if matches!(command, Command::Postscript(_)) {
                    BlockKind::Postscript
                } else {
                    BlockKind::Attachment
                };
                let mut at = 0;
                self.selection = self.session.edit(self.selection, |view, _| {
                    at = view.insert_letter_note(kind);
                })?;
                self.selection.primary.index = at;
                self.selection.secondary.index = at;
                self.selection.primary.prefer_next_row = true;
                self.selection.secondary.prefer_next_row = true;
                self.session.typing_group = false;
            }
            other => {
                let (params, kind) = match other {
                    Command::Paragraph(p) => (p, BlockKind::Paragraph),
                    Command::Heading1(p) => (p, BlockKind::Heading(1)),
                    Command::Heading2(p) => (p, BlockKind::Heading(2)),
                    Command::Heading3(p) => (p, BlockKind::Heading(3)),
                    Command::Bullet(p) => (p, BlockKind::Bullet),
                    Command::Numbered(p) => (p, BlockKind::Numbered),
                    Command::Description(p) => (p, BlockKind::Description),
                    Command::Quote(p) => (p, BlockKind::Quote),
                    _ => unreachable!("remaining block commands"),
                };
                self.edit_target(params.target, |view, range| view.set_block(range, kind))?;
            }
        }
        Ok(None)
    }
    pub(super) fn select_resolved(&mut self, selection: Selection) {
        self.selection = selection;
        let range = selection.range();
        self.session.document.view.typing_style =
            self.session.document.view.style_at(if range.is_empty() {
                selection.primary.index.saturating_sub(1)
            } else {
                range.start
            });
        self.session.typing_group = false;
    }
    fn mark(&mut self, params: StyleParams, mark: Mark) -> Result<(), CommandError> {
        self.edit_target(params.target, |view, range| match params.value {
            StyleValue::Toggle => view.toggle_mark(range, mark),
            StyleValue::On => view.set_mark(range, mark, true),
            StyleValue::Off => view.set_mark(range, mark, false),
        })
    }
    fn edit_target(
        &mut self,
        target: TextTarget,
        edit: impl FnOnce(&mut Projection, std::ops::Range<usize>),
    ) -> Result<(), CommandError> {
        let selection = self.resolve(target)?;
        self.selection = self.session.edit(selection, edit)?;
        Ok(())
    }
    pub(super) fn resolve_query_target(
        &self,
        target: TextTarget,
    ) -> Result<Selection, CommandError> {
        self.resolve(target)
    }
}
