//! Immutable image bytes prepared privately, then admitted with the whole batch.
use crate::{
    assets::{self, Assets},
    content::{commands::ImageResource, contract::Error},
};
use sha2::{Digest, Sha256};

fn invalid(reason: impl Into<String>) -> Error {
    Error::InvalidDocument {
        reason: reason.into(),
    }
}
impl ImageResource {
    /// No filesystem or session side effects. Hosts only choose/read the input file.
    pub fn prepare(extension: &str, data: Vec<u8>) -> Result<Self, Error> {
        let extension = extension.to_ascii_lowercase();
        let extension = if extension == "jpeg" {
            "jpg"
        } else {
            &extension
        };
        assets::decode(&data, extension).map_err(|e| invalid(e.to_string()))?;
        let hash: String = Sha256::digest(&data)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self {
            path: format!("images/{}.{}", &hash[..24], extension),
            data,
        })
    }
}
pub(super) fn add(assets: &mut Assets, resource: &ImageResource) -> Result<(), Error> {
    if !assets::valid_path(&resource.path) {
        return Err(invalid("invalid_resource_path"));
    }
    if let Some(before) = assets.get(&resource.path) {
        return if before.as_ref() == resource.data {
            Ok(())
        } else {
            Err(invalid("immutable_resource"))
        };
    }
    assets::decode(
        &resource.data,
        resource.path.rsplit('.').next().unwrap_or(""),
    )
    .map_err(|e| invalid(e.to_string()))?;
    let total = assets
        .values()
        .try_fold(resource.data.len(), |size, data| {
            size.checked_add(data.len())
        })
        .ok_or(Error::StateLimitExceeded)?;
    if total > assets::MAX_DOCUMENT_BYTES as usize {
        return Err(Error::StateLimitExceeded);
    }
    assets.insert(resource.path.clone(), resource.data.clone().into());
    Ok(())
}
