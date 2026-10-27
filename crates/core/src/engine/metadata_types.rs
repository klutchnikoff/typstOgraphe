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
    #[schemars(schema_with = "metadata_value_schema")]
    pub value: serde_json::Value,
    #[serde(default)]
    pub input: MetadataInput,
}

pub(super) fn metadata_value_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "description": "Text for scalar fields; an array of typed records for collection fields. Discover each collection's record schema in document_types.describe / structured_metadata.",
        "anyOf": [{"type":"string"}, {"type":"array", "maxItems":100, "items":{"type":"object"}}]
    })
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
