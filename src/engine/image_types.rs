use super::{TargetGuard, TextTarget};
use crate::document::structure::BlockId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BlockTarget {
    pub guard: TargetGuard,
    pub block_id: BlockId,
}

/// A local image file or an embedded copy encoded as standard base64.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImageInput {
    Path {
        path: std::path::PathBuf,
    },
    Embedded {
        /// PNG, JPG, JPEG or SVG; normalized to lowercase on import.
        extension: String,
        /// Standard base64 without a data URL prefix.
        data: String,
    },
}
impl From<&crate::assets::ImageData> for ImageInput {
    fn from(image: &crate::assets::ImageData) -> Self {
        use base64::Engine;
        Self::Embedded {
            extension: image.extension.clone(),
            data: base64::prelude::BASE64_STANDARD.encode(&image.data),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InsertImageParams {
    #[serde(default)]
    pub target: TextTarget,
    pub image: ImageInput,
    #[serde(default)]
    pub alt: String,
    #[serde(default)]
    pub caption: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplaceImageParams {
    pub target: BlockTarget,
    pub image: ImageInput,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImageTargetParams {
    pub target: BlockTarget,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImageAltParams {
    pub target: BlockTarget,
    pub alt: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SignatureParams {
    pub guard: TargetGuard,
    pub image: Option<ImageInput>,
}
