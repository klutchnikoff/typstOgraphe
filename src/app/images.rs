use super::*;

impl App {
    pub(super) fn choose_image(&mut self, replace: Option<u64>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(crate::tr!("image-files"), &["png", "jpg", "jpeg", "svg"])
            .pick_file()
            && let Err(error) = self.import_image(&path, replace)
        {
            self.file.error = Some(error);
        }
    }
    pub(super) fn import_image(
        &mut self,
        path: &std::path::Path,
        replace: Option<u64>,
    ) -> Result<(), String> {
        let image = crate::assets::ImageData::read(path)?;
        let before = self.snapshot();
        let asset_path = image.insert(&mut self.session.document.assets, "image");
        let at = if let Some(id) = replace {
            let Some(image) = self.session.document.view.images.get_mut(&id) else {
                self.session.document = before.document;
                return Ok(());
            };
            image.path = asset_path;
            self.session.document.view.image_range(id).unwrap().start
        } else {
            let range = self.range();
            self.session.document.view.delete_char_range(range.clone());
            self.session.document.view.insert_image(
                range.start,
                ImageBlock {
                    path: asset_path,
                    alt: String::new(),
                },
            )
        };
        if let Err(error) = self.check_asset_budget() {
            self.session.document = before.document;
            return Err(error);
        }
        self.focus_annotation_target(at + 1..at + 1);
        self.remember(before, false);
        Ok(())
    }
    pub(super) fn check_asset_budget(&self) -> Result<(), String> {
        let dependency_bytes = self
            .session
            .dependencies
            .values()
            .map(|v| v.len())
            .sum::<usize>();
        if self
            .session
            .document
            .assets
            .values()
            .map(|v| v.len())
            .sum::<usize>()
            + dependency_bytes
            <= crate::assets::MAX_DOCUMENT_BYTES as usize
        {
            return Ok(());
        }
        let mut document = self.session.document.clone();
        document.sync_source();
        let retained = crate::rendering::retained_assets(
            document.source(),
            &self.session.metadata,
            &self.session.template,
            &document.assets,
            &self.session.dependencies,
        )?;
        if retained.values().map(|v| v.len()).sum::<usize>() + dependency_bytes
            > crate::assets::MAX_DOCUMENT_BYTES as usize
        {
            return Err(crate::tr!("storage-letter-too-large"));
        }
        Ok(())
    }
    pub(super) fn remove_image(&mut self, id: u64) {
        if let Some(range) = self.session.document.view.image_range(id) {
            let before = self.snapshot();
            let end = (range.end + 1).min(self.session.document.view.len());
            self.session
                .document
                .view
                .delete_char_range(range.start..end);
            self.selection = editor::cursor(range.start..range.start);
            self.force_selection = true;
            self.remember(before, false);
        }
    }
}
