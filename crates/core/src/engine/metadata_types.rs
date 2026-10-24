use super::*;
use crate::{
    document::{DocumentLanguage, PaperFormat},
    document_types::MetadataField,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataInput {
    #[default]
    Discrete,
    Typing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetMetadataParams {
    pub guard: TargetGuard,
    pub field: MetadataField,
    pub value: String,
    #[serde(default)]
    pub input: MetadataInput,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "key",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DocumentOption {
    Paper(PaperFormat),
    Landscape(bool),
    Language(DocumentLanguage),
    BulletMarker(Option<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetOptionParams {
    pub guard: TargetGuard,
    pub option: DocumentOption,
}
