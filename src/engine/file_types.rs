//! Paramètres documentaires : chemins explicites, gardes et décision de remplacement.
use super::{ImageInput, TargetGuard};
use crate::document_types::{DocumentType, MetadataField};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitialValue {
    pub field: MetadataField,
    pub value: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewDocumentParams {
    pub guard: TargetGuard,
    #[serde(rename = "type")]
    pub document_type: DocumentType,
    pub model: String,
    #[serde(default)]
    pub initial_values: Vec<InitialValue>,
    #[serde(default)]
    pub signature: Option<ImageInput>,
    #[serde(default)]
    pub discard_changes: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenDocumentParams {
    pub guard: TargetGuard,
    pub path: PathBuf,
    #[serde(default)]
    pub discard_changes: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseDocumentParams {
    pub guard: TargetGuard,
    #[serde(default)]
    pub discard_changes: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveDocumentParams {
    pub guard: TargetGuard,
    #[serde(default)]
    pub path: Option<PathBuf>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveAsParams {
    pub guard: TargetGuard,
    pub path: PathBuf,
    #[serde(default)]
    pub overwrite: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Pdf,
    Typst,
    #[default]
    Both,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportParams {
    pub guard: TargetGuard,
    pub directory: PathBuf,
    #[serde(default)]
    pub format: ExportFormat,
    #[serde(default = "with_page_images")]
    pub page_images: bool,
}
fn with_page_images() -> bool {
    true
}
