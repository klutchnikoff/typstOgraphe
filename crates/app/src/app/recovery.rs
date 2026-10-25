use super::*;

impl App {
    pub(super) fn start_recovery(&mut self, directory: PathBuf, ctx: egui::Context) {
        match crate::recovery::Recovery::start(
            &directory,
            std::sync::Arc::new(move || ctx.request_repaint()),
        ) {
            Ok((recovery, entries)) => {
                self.recovery.worker = Some(recovery);
                self.recovery.open = !entries.is_empty();
                self.document.engine.configure_recovery(entries);
                self.refresh_recovery_list();
            }
            Err(error) => self.recovery.error = Some(crate::i18n::error(&error)),
        }
    }
    pub(super) fn clear_recovery(&mut self) {
        if let Some(recovery) = &mut self.recovery.worker
            && let Err(error) = recovery.clear()
        {
            self.recovery.error = Some(crate::i18n::error(&error));
        }
    }
    pub(super) fn checkpoint_recovery(&mut self) {
        if !self.document.open {
            return;
        }
        let dirty = self.is_dirty();
        if let Some(recovery) = &mut self.recovery.worker {
            for result in recovery.poll() {
                self.recovery.error = result.err().map(|e| crate::i18n::error(&e));
            }
            let session = &self.document.engine;
            let suggested = &self.recovery.recovered_name;
            if let Err(error) = recovery.checkpoint(session.revision(), dirty, || {
                let name = session
                    .file_path()
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .or_else(|| suggested.clone());
                (session.stored_document(), name)
            }) {
                self.recovery.error = Some(crate::i18n::error(&error));
            }
        }
    }
    pub(super) fn recovered_filename(&self) -> Option<String> {
        self.recovery.recovered_name.as_ref().map(|name| {
            let stem = std::path::Path::new(name)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            format!("{stem}-{}.typsto", crate::tr!("recovery-filename-suffix"))
        })
    }
    pub(super) fn refresh_recovery_list(&mut self) {
        let value = self
            .document
            .engine
            .query_call(crate::engine::Query::RecoveryList(crate::engine::Empty {}).to_call())
            .expect("recovery listing");
        self.recovery.entries =
            serde_json::from_value(value["entries"].clone()).expect("typed recovery entries");
    }
    pub(super) fn recover_document(&mut self, index: usize) {
        let Some(entry) = self.recovery.entries.get(index) else {
            return;
        };
        let name = entry
            .name
            .clone()
            .unwrap_or_else(|| crate::tr!("untitled-letter"));
        let command =
            crate::engine::Command::RestoreRecovery(crate::engine::RestoreRecoveryParams {
                guard: self.document.engine.target_guard(),
                reference: entry.reference,
                discard_changes: true,
            });
        match self.execute_document_command(command) {
            Ok(_) => {
                self.recovery.recovered_name = Some(name);
                if let Some(entry) = self.document.engine.take_recovered_entry()
                    && let Some(recovery) = &mut self.recovery.worker
                    && let Err(error) = recovery.adopt(entry)
                {
                    self.recovery.error = Some(crate::i18n::error(&error));
                }
                self.refresh_recovery_list();
                self.recovery.open = false;
                self.dialogs.message = Some(crate::tr!("recovery-restored"));
            }
            Err(error) => {
                self.file.error = Some(crate::tr!(
                    "recovery-load-failed",
                    error = Self::command_error_text(&error)
                ))
            }
        }
    }
    pub(super) fn recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery.open || self.file.pending_action.is_some() || self.file.error.is_some() {
            return;
        }
        use crate::ui_kit::{ButtonKind, Dialog, button, button_enabled};
        let mut choice = None;
        let mut later = false;
        let (title, help) = (crate::tr!("recovery-title"), crate::tr!("recovery-help"));
        let response = Dialog::new("recovery-documents", &title)
            .description(&help)
            .width(540.0)
            .show(
                ctx,
                |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(340.0)
                        .show(ui, |ui| {
                            for (index, entry) in self.recovery.entries.iter().enumerate() {
                                egui::Frame::new()
                                    .stroke(egui::Stroke::new(
                                        1.0_f32,
                                        ui.visuals().widgets.noninteractive.bg_stroke.color,
                                    ))
                                    .corner_radius(crate::theme::scale::RADIUS_CARD)
                                    .inner_margin(egui::Margin::same(10))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.strong(
                                            entry
                                                .name
                                                .clone()
                                                .unwrap_or_else(|| crate::tr!("untitled-letter")),
                                        );
                                        if let Some(error) = &entry.error {
                                            ui.colored_label(
                                                ui.visuals().error_fg_color,
                                                crate::tr!(
                                                    "recovery-load-failed",
                                                    error = entry
                                                        .cause
                                                        .as_ref()
                                                        .map(crate::i18n::error)
                                                        .unwrap_or_else(|| error.clone())
                                                ),
                                            );
                                        } else if let Some(date) = entry.saved_at {
                                            let date =
                                                time::OffsetDateTime::from_unix_timestamp(date)
                                                    .ok()
                                                    .and_then(|d| {
                                                        d.format(
                                                    &time::format_description::well_known::Rfc3339,
                                                )
                                                .ok()
                                                    })
                                                    .unwrap_or_default();
                                            ui.weak(crate::tr!("recovery-date", date = date));
                                        }
                                        ui.horizontal(|ui| {
                                            if button_enabled(
                                                ui,
                                                ButtonKind::Primary,
                                                entry.error.is_none(),
                                                crate::tr!("recovery-restore"),
                                            )
                                            .clicked()
                                            {
                                                choice = Some((index, true));
                                            }
                                            if button(
                                                ui,
                                                ButtonKind::Ghost,
                                                crate::tr!("recovery-delete"),
                                            )
                                            .clicked()
                                            {
                                                choice = Some((index, false));
                                            }
                                        });
                                    });
                            }
                        });
                },
                |ui| {
                    later =
                        button(ui, ButtonKind::Secondary, crate::tr!("recovery-later")).clicked();
                },
            );
        self.recovery.open = !later && !response.should_close();
        if let Some((index, restore)) = choice {
            if restore {
                self.request_action(PendingAction::Recover(index), ctx);
            } else {
                let reference = self.recovery.entries[index].reference;
                match self.document.engine.run_call(
                    crate::engine::Command::DeleteRecovery(crate::engine::DeleteRecoveryParams {
                        reference,
                    })
                    .to_call(),
                ) {
                    Ok(_) => self.refresh_recovery_list(),
                    Err(error) => {
                        self.file.error = Some(crate::tr!(
                            "recovery-delete-failed",
                            error = Self::command_error_text(&error)
                        ))
                    }
                }
                self.recovery.open &= !self.recovery.entries.is_empty();
            }
        }
    }
}
