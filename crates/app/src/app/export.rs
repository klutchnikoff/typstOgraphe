//! Export hors du thread graphique, à partir d’un instantané préparé par le moteur.
use super::*;

impl App {
    pub(super) fn export_document(
        &mut self,
        params: crate::engine::ExportParams,
        ctx: &egui::Context,
    ) -> bool {
        self.cancel_ime();
        if self.file.export_task.is_some() {
            return false;
        }
        let task = match self.document.engine.prepare_export(params) {
            Ok(task) => task,
            Err(error) => {
                self.dialogs.message = Some(Self::command_error_text(&error));
                return false;
            }
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("typst-export".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let result = task.execute();
                let _ = sender.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => {
                self.file.export_task = Some(receiver);
                self.file.export_open = false;
                self.dialogs.message = Some(crate::tr!("export-pending"));
                true
            }
            Err(error) => {
                self.dialogs.message = Some(error.to_string());
                false
            }
        }
    }
    pub(super) fn poll_export(&mut self) {
        let Some(receiver) = &self.file.export_task else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.file.export_task = None;
                self.dialogs.message = Some(crate::tr!("renderer-stopped"));
                return;
            }
        };
        self.file.export_task = None;
        self.dialogs.message = Some(match result {
            Ok(value) => crate::tr!(
                "letter-exported",
                path = value["directory"].as_str().unwrap_or_default()
            ),
            Err(error) => Self::command_error_text(&error),
        });
    }
}
