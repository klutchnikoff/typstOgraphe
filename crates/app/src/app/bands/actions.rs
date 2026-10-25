use super::*;
use crate::engine::{Command as Core, TargetParams, TextTarget};
impl App {
    pub(in crate::app) fn apply_band_action(&mut self, action: BandAction, ctx: &egui::Context) {
        if matches!(action, BandAction::Close) {
            self.close_band(ctx);
            return;
        }
        let Some(band) = &self.bands.open else {
            return;
        };
        if self.document.engine.resolve_target(band.target).is_err() {
            let error = crate::tr!("edit-stale-draft");
            self.bands.open.as_mut().expect("open band").error = Some(error.clone());
            self.dialogs.message = Some(error);
            return;
        }
        let command = match (&band.content, &action) {
            (BandContent::Image { reference, alt }, BandAction::Apply) => {
                Some(Core::SetImageAlt(crate::engine::ImageAltParams {
                    target: *reference,
                    alt: alt.clone(),
                }))
            }
            (BandContent::List { kind }, BandAction::Apply) => {
                let p = TargetParams {
                    target: band.target.into(),
                };
                Some(match kind {
                    BlockKind::Bullet => Core::Bullet(p),
                    BlockKind::Numbered => Core::Numbered(p),
                    _ => Core::Description(p),
                })
            }
            (BandContent::Link { url }, BandAction::Apply) => {
                Some(Core::SetLink(crate::engine::LinkParams {
                    target: band.target.into(),
                    url: url.clone(),
                }))
            }
            (BandContent::Link { .. }, BandAction::Remove) => {
                Some(Core::RemoveLink(TargetParams {
                    target: band.target.into(),
                }))
            }
            (
                BandContent::Remove {
                    image: Some(reference),
                    ..
                },
                BandAction::Remove,
            ) => Some(Core::RemoveImage(crate::engine::ImageTargetParams {
                target: *reference,
            })),
            (BandContent::Remove { .. }, BandAction::Remove) => {
                // Normalise the now-empty first row as an ordinary paragraph too.
                // Both steps are one transaction and use the existing public commands.
                let guard = band.target.guard;
                Some(Core::Batch(crate::engine::BatchParams {
                    guard,
                    save: None,
                    steps: vec![
                        Core::Paragraph(TargetParams {
                            target: band.target.into(),
                        })
                        .to_call(),
                        Core::DeleteText(TargetParams {
                            target: TextTarget::Current {},
                        })
                        .to_call(),
                    ],
                }))
            }
            (BandContent::Spelling { issue }, BandAction::Correct(text)) => Some(
                Core::CorrectSpelling(crate::engine::SpellingCorrectionParams {
                    issue: issue.clone(),
                    replacement: text.clone(),
                }),
            ),
            (BandContent::Spelling { issue }, BandAction::Spelling(command)) => {
                let issue = issue.clone();
                let command = *command;
                self.apply_spelling_issue(command, issue);
                self.close_band(ctx);
                return;
            }
            (BandContent::Image { reference, .. }, BandAction::ReplaceImage) => {
                let id = self
                    .document
                    .engine
                    .document()
                    .view
                    .images
                    .keys()
                    .copied()
                    .find(|id| {
                        self.document
                            .engine
                            .image_target(*id)
                            .is_some_and(|t| t.block_id == reference.block_id)
                    });
                if let Some(id) = id {
                    let guard = self.document.engine.target_guard();
                    self.choose_image(Some(id));
                    if self.document.engine.target_guard() != guard {
                        self.close_band(ctx);
                    }
                }
                return;
            }
            _ => None,
        };
        let Some(command) = command else {
            return;
        };
        match self.execute_document_command(command) {
            Ok(_) => {
                self.spelling.target = None;
                self.close_band(ctx);
            }
            Err(error) => {
                let error = Self::command_error_text(&error);
                self.bands.open.as_mut().expect("open band").error = Some(error.clone());
                self.dialogs.message = Some(error);
            }
        }
    }
}
