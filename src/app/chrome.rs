use super::*;

impl App {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        if let Some(command) =
            crate::toolbar::show(ui, &self.session.document.view, self.range(), &self.keymap)
        {
            self.apply(command);
        }
    }
    pub(super) fn status_ui(&mut self, ui: &mut egui::Ui) {
        let range = self.range();
        let (total, selected) =
            self.statistics
                .counts(self.session.revision, &self.session.document.view, range);
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::tr!(
                "document-stats",
                words = total.words,
                signs = total.signs
            ))
            .on_hover_text(crate::tr!(
                "statistics-hint",
                without_spaces = total.signs_without_spaces
            ));
            if let Some(selected) = selected {
                ui.separator();
                ui.label(crate::tr!(
                    "selection-stats",
                    words = selected.words,
                    signs = selected.signs
                ))
                .on_hover_text(crate::tr!(
                    "statistics-hint",
                    without_spaces = selected.signs_without_spaces
                ));
            }
            if self.preferences.spelling.enabled {
                ui.separator();
                let language = self.session.metadata.options().language;
                if !crate::spelling::supported(language) {
                    ui.weak(crate::tr!(
                        "spelling-no-dictionary",
                        language = language.label()
                    ))
                    .on_hover_text(crate::tr!("spelling-dictionaries"));
                } else if let Some(error) = &self.spelling.checker.error {
                    ui.colored_label(ui.visuals().error_fg_color, crate::tr!("spelling-error"))
                        .on_hover_text(error);
                } else if self.spelling.checker.ready {
                    let response = ui.add(
                        egui::Button::new(crate::tr!(
                            "spelling-count",
                            count = self.spelling.checker.issues.len()
                        ))
                        .frame(false),
                    );
                    let clicked = response
                        .on_hover_text(crate::tr!("spelling-hint"))
                        .clicked();
                    if clicked {
                        self.apply(Command::NextSpelling);
                    }
                } else {
                    ui.weak(crate::tr!("spelling-checking"));
                }
            }
            if let Some(message) = &self.message {
                ui.separator();
                ui.add(egui::Label::new(message).truncate())
                    .on_hover_text(message);
            }
        });
    }
}
