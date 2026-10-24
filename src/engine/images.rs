//! Préparation des ressources puis publication atomique, sans dialogue ni profil global.
use super::*;
use crate::{
    assets::{ImageData, MAX_IMAGE_BYTES},
    document::{
        ImageBlock,
        structure::{BlockContent, LogicalSelection, targets::Position},
    },
    document_types::Metadata,
    session::Caret,
};

impl ImageInput {
    pub(super) fn read(&self) -> Result<ImageData, CommandError> {
        match self {
            Self::Path { path } => Ok(ImageData::read(path)?),
            Self::Embedded { extension, data } => {
                use base64::Engine;
                if data.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 {
                    return Err(crate::assets::ImageError::TooLarge.into());
                }
                let data = base64::prelude::BASE64_STANDARD
                    .decode(data)
                    .map_err(|error| {
                        CommandError::detail(ErrorCode::InvalidParams, "image_base64", error)
                    })?;
                let extension = extension.to_ascii_lowercase();
                let extension = if extension == "jpeg" {
                    "jpg".into()
                } else {
                    extension
                };
                crate::assets::decode(&data, &extension)?;
                Ok(ImageData {
                    extension,
                    data: data.into(),
                })
            }
        }
    }
}

impl Engine {
    /// Pont temporaire pour les identifiants de ressources du peintre egui.
    pub(crate) fn image_target(&self, resource: u64) -> Option<BlockTarget> {
        self.session
            .document
            .structure
            .blocks
            .iter()
            .find_map(|block| match &block.content {
                BlockContent::Image { resource: id, .. } if *id == resource => Some(BlockTarget {
                    guard: self.session.target_guard(),
                    block_id: block.id,
                }),
                _ => None,
            })
    }

    pub(crate) fn image_selection_target(&self, target: BlockTarget) -> TextTarget {
        SelectionTarget {
            guard: target.guard,
            selection: LogicalSelection {
                anchor: Position::Before(target.block_id),
                head: Position::Before(target.block_id),
            },
        }
        .into()
    }

    pub(super) fn resolve_image(
        &self,
        target: BlockTarget,
    ) -> Result<(u64, std::ops::Range<usize>), CommandError> {
        self.resolve(self.image_selection_target(target))?;
        let resource = self
            .session
            .document
            .structure
            .blocks
            .iter()
            .find_map(|block| {
                if block.id == target.block_id
                    && let BlockContent::Image { resource, .. } = &block.content
                {
                    return Some(*resource);
                }
                None
            })
            .ok_or_else(|| CommandError::new(ErrorCode::Unavailable, "image_required"))?;
        let range = self
            .session
            .document
            .view
            .image_range(resource)
            .ok_or_else(|| CommandError::new(ErrorCode::Document, "invalid_projection"))?;
        Ok((resource, range))
    }

    pub(super) fn check_signature(&self, params: &SignatureParams) -> Result<(), CommandError> {
        self.check_guard(params.guard)
    }

    pub(super) fn insert_image(&mut self, params: InsertImageParams) -> Result<(), CommandError> {
        let selection = self.resolve(params.target)?;
        let image = params.image.read()?;
        let before = self.session.snapshot(selection);
        let mut candidate = before.clone();
        let path = image.insert(&mut candidate.document.assets, "image");
        let view = &mut candidate.document.view;
        view.delete_char_range(selection.range());
        let at = view.insert_image(
            selection.range().start,
            ImageBlock {
                path,
                alt: params.alt,
            },
        );
        view.insert_text(&params.caption, at + 1);
        self.publish_resource(before, candidate, collapsed(at + 1, true), true)
    }

    pub(super) fn replace_image(&mut self, params: ReplaceImageParams) -> Result<(), CommandError> {
        let (id, range) = self.resolve_image(params.target)?;
        let image = params.image.read()?;
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        let path = image.insert(&mut candidate.document.assets, "image");
        candidate.document.view.images.get_mut(&id).unwrap().path = path;
        self.publish_resource(before, candidate, collapsed(range.start + 1, true), true)
    }

    pub(super) fn remove_image(&mut self, params: ImageTargetParams) -> Result<(), CommandError> {
        let (_, range) = self.resolve_image(params.target)?;
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        let end = (range.end + 1).min(candidate.document.view.len());
        candidate.document.view.delete_char_range(range.start..end);
        self.publish_resource(before, candidate, collapsed(range.start, false), false)
    }

    pub(super) fn set_image_alt(&mut self, params: ImageAltParams) -> Result<(), CommandError> {
        let (id, _) = self.resolve_image(params.target)?;
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        candidate.document.view.images.get_mut(&id).unwrap().alt = params.alt;
        self.publish_resource(before, candidate, self.selection, false)
    }

    pub(super) fn set_signature(&mut self, params: SignatureParams) -> Result<(), CommandError> {
        let image = params.image.as_ref().map(ImageInput::read).transpose()?;
        let current = self.session.metadata.signature_path();
        if match (&image, current) {
            (None, None) => true,
            (Some(image), Some(path)) => self
                .session
                .document
                .assets
                .get(path)
                .is_some_and(|data| data == &image.data),
            _ => false,
        } {
            return Ok(());
        }
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        if let Metadata::Courrier(meta) = &mut candidate.metadata {
            meta.signature = image
                .as_ref()
                .map(|image| image.insert(&mut candidate.document.assets, "signature"));
        }
        self.publish_resource(before, candidate, self.selection, image.is_some())
    }

    fn publish_resource(
        &mut self,
        before: Snapshot,
        candidate: Snapshot,
        selection: Selection,
        budget: bool,
    ) -> Result<(), CommandError> {
        if budget {
            super::resources::check_budget(&candidate)?;
        }
        self.selection = self.session.commit_snapshot(
            self.session.target_guard(),
            before,
            candidate,
            selection,
            EditGroup::Discrete,
        )?;
        Ok(())
    }
}

fn collapsed(index: usize, prefer_next_row: bool) -> Selection {
    let caret = Caret {
        index,
        prefer_next_row,
    };
    Selection {
        primary: caret,
        secondary: caret,
    }
}
