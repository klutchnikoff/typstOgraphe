use super::*;

impl App {
    pub(super) fn set_document_signature(
        &mut self,
        image: Option<&crate::assets::ImageData>,
    ) -> Result<(), String> {
        if !matches!(
            self.document.engine.metadata,
            crate::document_types::Metadata::Courrier(_)
        ) {
            return Ok(());
        }
        let current = self.document.engine.metadata.signature_path();
        if match (image, current) {
            (None, None) => true,
            (Some(image), Some(path)) => self
                .document
                .engine
                .document
                .assets
                .get(path)
                .is_some_and(|data| data == &image.data),
            _ => false,
        } {
            return Ok(());
        }
        let before = self.snapshot();
        let path =
            image.map(|image| image.insert(&mut self.document.engine.document.assets, "signature"));
        if let crate::document_types::Metadata::Courrier(meta) = &mut self.document.engine.metadata
        {
            meta.signature = path;
        }
        if image.is_some()
            && let Err(error) = self.check_asset_budget()
        {
            self.document.engine.document = before.document;
            self.document.engine.metadata = before.metadata;
            return Err(error);
        }
        self.remember(before, false);
        Ok(())
    }
    pub(super) fn use_profile_signature(&mut self) -> Result<(), String> {
        let image = self
            .profile_signature
            .clone()
            .filter(|_| self.document.open && self.preferences.profile.signature.is_some())
            .ok_or_else(|| crate::tr!("signature-profile-unavailable"))?;
        self.set_document_signature(Some(&image))
    }
    pub(super) fn choose_signature(&mut self, profile: bool) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(crate::tr!("signature-choose"))
            .add_filter(crate::tr!("image-files"), &["png", "jpg", "jpeg", "svg"])
            .pick_file()
        else {
            return;
        };
        if profile {
            let result = self
                .preferences_directory
                .as_deref()
                .ok_or_else(|| crate::tr!("profile-no-directory"))
                .and_then(|directory| crate::profile::Profile::import_signature(directory, &path));
            match result {
                Ok((name, image)) => {
                    self.preferences.profile.signature = Some(name);
                    self.profile_signature = Some(image);
                    self.dialogs.profile_error = None;
                }
                Err(error) => {
                    self.dialogs.profile_error =
                        Some(crate::tr!("profile-signature-error", error = error))
                }
            }
        } else {
            let result = crate::assets::ImageData::read(&path)
                .and_then(|image| self.set_document_signature(Some(&image)));
            if let Err(error) = result {
                self.file.error = Some(error);
            }
        }
    }
    pub(super) fn signature_preview(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        image: &crate::assets::ImageData,
    ) {
        if !self.image_textures.contains_key(key)
            && let Ok(thumbnail) = crate::assets::thumbnail(&image.data, &image.extension)
        {
            self.image_textures.insert(
                key.into(),
                ui.ctx().load_texture(key, thumbnail, Default::default()),
            );
        }
        if let Some(texture) = self.image_textures.get(key) {
            let size = texture.size_vec2();
            let scale = (220.0 / size.x).min(72.0 / size.y).min(1.0);
            egui::Frame::new()
                .fill(Color32::WHITE)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.add(egui::Image::new(texture).fit_to_exact_size(size * scale));
                });
        }
    }
    pub(super) fn document_signature_ui(&mut self, ui: &mut egui::Ui) {
        let path = self
            .document
            .engine
            .metadata
            .signature_path()
            .map(str::to_owned);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::tr!("signature-label"));
            if ui
                .button(if path.is_some() {
                    crate::tr!("image-replace")
                } else {
                    crate::tr!("signature-choose")
                })
                .clicked()
            {
                self.choose_signature(false);
            }
            if path.is_some() && ui.button(crate::tr!("image-remove")).clicked() {
                let _ = self.set_document_signature(None);
            }
            let available =
                self.preferences.profile.signature.is_some() && self.profile_signature.is_some();
            if ui
                .add_enabled(
                    available,
                    egui::Button::new(crate::tr!("signature-from-profile")),
                )
                .on_hover_text(crate::tr!("signature-from-profile-help"))
                .on_disabled_hover_text(crate::tr!("signature-profile-unavailable"))
                .clicked()
                && let Err(error) = self.use_profile_signature()
            {
                self.file.error = Some(error);
            }
        });
        if let Some(path) = self
            .document
            .engine
            .metadata
            .signature_path()
            .map(str::to_owned)
            && let Some(data) = self.document.engine.document.assets.get(&path).cloned()
        {
            let image = crate::assets::ImageData {
                extension: path.rsplit('.').next().unwrap().into(),
                data,
            };
            self.signature_preview(ui, &path, &image);
        }
        ui.weak(crate::tr!("signature-document-help"));
    }
}
