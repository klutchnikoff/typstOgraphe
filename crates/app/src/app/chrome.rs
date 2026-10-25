use super::*;

impl App {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        if let Some(command) = crate::toolbar::show(
            ui,
            &self.document.engine.document().view,
            self.range(),
            &self.keymap,
            |command| self.command_enabled(command),
        ) {
            self.apply(command);
        }
    }
    pub(super) fn status_ui(&mut self, ui: &mut egui::Ui) {
        let target = self
            .document
            .engine
            .selection_target(editor::to_core(self.document.selection))
            .expect("published editor selection");
        let counts = self
            .document
            .engine
            .statistics(crate::engine::StatisticsParams {
                target: Some(target.into()),
            })
            .expect("published document statistics");
        let total = counts.document;
        let selected = counts.target;
        use crate::ui_kit::{NoticeKind, chip};
        let muted = crate::theme::Tokens::new(ui.visuals().dark_mode).muted;
        let stats = |ui: &mut egui::Ui, text: String, without_spaces: usize| {
            ui.label(egui::RichText::new(text).color(muted))
                .on_hover_text(crate::tr!(
                    "statistics-hint",
                    without_spaces = without_spaces
                ));
        };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            stats(
                ui,
                crate::tr!("document-stats", words = total.words, signs = total.signs),
                total.signs_without_spaces,
            );
            if let Some(selected) = selected {
                stats(
                    ui,
                    crate::tr!(
                        "selection-stats",
                        words = selected.words,
                        signs = selected.signs
                    ),
                    selected.signs_without_spaces,
                );
            }
            if self.preferences.spelling.enabled {
                let language = self.document.engine.metadata().options().language;
                if !crate::spelling::supported(language) {
                    ui.weak(crate::tr!(
                        "spelling-no-dictionary",
                        language = language.label()
                    ))
                    .on_hover_text(crate::tr!("spelling-dictionaries"));
                } else if let Some(error) = &self.spelling.checker.error {
                    chip(ui, NoticeKind::Error, crate::tr!("spelling-error"))
                        .on_hover_text(crate::i18n::error(error));
                } else if self.spelling.checker.ready {
                    let count = self.spelling.checker.issues.len();
                    let kind = if count == 0 {
                        NoticeKind::Neutral
                    } else {
                        NoticeKind::Error
                    };
                    let response = chip(ui, kind, crate::tr!("spelling-count", count = count))
                        .on_hover_text(crate::tr!("spelling-hint"));
                    if response.clicked() {
                        self.apply(Command::NextSpelling);
                    }
                } else {
                    ui.weak(crate::tr!("spelling-checking"));
                }
            }
            if let Some(session) = &self.assistance.session {
                let connected = session.connected();
                let label = if connected {
                    crate::tr!("assistance-connected")
                } else {
                    crate::tr!("assistance-ready")
                };
                let kind = if connected {
                    NoticeKind::Success
                } else {
                    NoticeKind::Info
                };
                if chip(ui, kind, label).clicked() {
                    self.apply(Command::Assistance);
                }
            }
            if let Some(message) = &self.dialogs.message {
                ui.add(egui::Label::new(message).truncate())
                    .on_hover_text(message);
            }
        });
    }
}
