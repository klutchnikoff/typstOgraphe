use super::*;
use crate::engine::{
    Command as Core, ImageInput, ImageTargetParams, InsertImageParams, ReplaceImageParams,
    SelectTextParams, SelectionPart,
};

impl App {
    pub(super) fn choose_image(&mut self, replace: Option<u64>) {
        // Capturer la cible avant le dialogue natif, comme pour un appel différé.
        let target = replace.and_then(|id| self.document.engine.image_target(id));
        let insertion = self
            .document
            .engine
            .selection_target(editor::to_core(self.document.selection))
            .ok();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(crate::tr!("image-files"), &["png", "jpg", "jpeg", "svg"])
            .pick_file()
        {
            let image = ImageInput::Path { path };
            let command = if let Some(target) = target {
                Core::ReplaceImage(ReplaceImageParams { target, image })
            } else if replace.is_none() {
                let Some(target) = insertion else {
                    return;
                };
                Core::InsertImage(InsertImageParams {
                    target: target.into(),
                    image,
                    alt: String::new(),
                    caption: String::new(),
                })
            } else {
                return;
            };
            if let Err(error) = self.execute_document_command(command) {
                self.file.error = Some(Self::command_error_text(&error));
            }
        }
    }
    pub(super) fn import_image(
        &mut self,
        path: &std::path::Path,
        replace: Option<u64>,
    ) -> Result<(), String> {
        let image = ImageInput::Path { path: path.into() };
        let command = if let Some(id) = replace {
            let target = self
                .document
                .engine
                .image_target(id)
                .ok_or_else(|| crate::tr!("engine-unavailable"))?;
            Core::ReplaceImage(ReplaceImageParams { target, image })
        } else {
            Core::InsertImage(InsertImageParams {
                target: Default::default(),
                image,
                alt: String::new(),
                caption: String::new(),
            })
        };
        self.execute_document_command(command)
            .map(|_| ())
            .map_err(|error| Self::command_error_text(&error))
    }
    pub(super) fn remove_image(&mut self, id: u64) {
        if let Some(target) = self.document.engine.image_target(id) {
            self.run_document_command(Core::RemoveImage(ImageTargetParams { target }));
        }
    }
    pub(super) fn select_image_part(&mut self, id: u64, part: SelectionPart) {
        if let Some(target) = self.document.engine.image_target(id) {
            let target = self.document.engine.image_selection_target(target);
            self.run_document_command(Core::SelectText(SelectTextParams {
                target,
                part: Some(part),
            }));
        }
    }
    pub(super) fn open_image_info(&mut self, id: u64) {
        self.dialogs.image_target = self.document.engine.image_target(id);
        if self.dialogs.image_target.is_some() {
            self.dialogs
                .image_alt
                .clone_from(&self.document.engine.document().view.images[&id].alt);
            self.dialogs.image_info = Some(id);
        }
    }
}
