use super::*;

impl App {
    pub(super) fn start_recovery(&mut self, directory: PathBuf, ctx: egui::Context) {
        match crate::recovery::Recovery::start(&directory, ctx) {
            Ok((recovery, entries)) => {
                self.recovery.worker = Some(recovery);
                self.recovery.open = !entries.is_empty();
                self.recovery.entries = entries;
            }
            Err(error) => self.recovery.error = Some(error),
        }
    }
    pub(super) fn clear_recovery(&mut self) {
        if let Some(recovery) = &mut self.recovery.worker
            && let Err(error) = recovery.clear()
        {
            self.recovery.error = Some(error);
        }
    }
    pub(super) fn checkpoint_recovery(&mut self) {
        if !self.document_open {
            return;
        }
        let dirty = self.is_dirty();
        if let Some(recovery) = &mut self.recovery.worker {
            for result in recovery.poll() {
                self.recovery.error = result.err();
            }
            let session = &self.session;
            let suggested = &self.recovery.recovered_name;
            if let Err(error) = recovery.checkpoint(session.revision, dirty, || {
                let name = session
                    .file_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .or_else(|| suggested.clone());
                (session.stored_document(), name)
            }) {
                self.recovery.error = Some(error);
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
    pub(super) fn recover_document(&mut self, index: usize) {
        let Some(entry) = self.recovery.entries.get(index) else {
            return;
        };
        let name = entry.name();
        let restored = entry.load().and_then(|stored| {
            Document::from_source(&stored.source).map(|document| (stored, document))
        });
        match restored.and_then(|(stored, document)| self.install_document(stored, document, None))
        {
            Ok(()) => {
                let entry = self.recovery.entries.remove(index);
                self.session.recovered = true;
                self.recovery.recovered_name = Some(name);
                if let Some(recovery) = &mut self.recovery.worker
                    && let Err(error) = recovery.adopt(entry)
                {
                    self.recovery.error = Some(error);
                }
                self.recovery.open = false;
                self.message = Some(crate::tr!("recovery-restored"));
            }
            Err(error) => self.file_error = Some(crate::tr!("recovery-load-failed", error = error)),
        }
    }
    pub(super) fn recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery.open || self.pending_action.is_some() || self.file_error.is_some() {
            return;
        }
        let mut open = true;
        let mut choice = None;
        let mut later = false;
        egui::Window::new(crate::tr!("recovery-title"))
            .id(egui::Id::new("recovery-documents"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.label(crate::tr!("recovery-help"));
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        for (index, entry) in self.recovery.entries.iter().enumerate() {
                            ui.separator();
                            ui.strong(entry.name());
                            match &entry.info {
                                Ok(info) => {
                                    ui.weak(crate::tr!("recovery-date", date = info.date()));
                                }
                                Err(error) => {
                                    ui.colored_label(
                                        Color32::LIGHT_RED,
                                        crate::tr!("recovery-load-failed", error = error.as_str()),
                                    );
                                }
                            }
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(
                                        entry.info.is_ok(),
                                        egui::Button::new(crate::tr!("recovery-restore")),
                                    )
                                    .clicked()
                                {
                                    choice = Some((index, true));
                                }
                                if ui.button(crate::tr!("recovery-delete")).clicked() {
                                    choice = Some((index, false));
                                }
                            });
                        }
                    });
                ui.separator();
                later = ui.button(crate::tr!("recovery-later")).clicked();
            });
        self.recovery.open = open && !later;
        if let Some((index, restore)) = choice {
            if restore {
                self.request_action(PendingAction::Recover(index), ctx);
            } else {
                match self.recovery.entries[index].delete() {
                    Ok(()) => {
                        self.recovery.entries.remove(index);
                    }
                    Err(error) => {
                        self.file_error = Some(crate::tr!("recovery-delete-failed", error = error))
                    }
                }
                self.recovery.open &= !self.recovery.entries.is_empty();
            }
        }
    }
}
