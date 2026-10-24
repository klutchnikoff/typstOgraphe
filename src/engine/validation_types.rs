//! Diagnostics lisibles par machine ; le détail des bibliothèques est informatif.
use super::RenderInfo;
use crate::{document_types::MetadataField, session::TargetGuard};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<MetadataField>,
}
impl Diagnostic {
    pub(super) fn error(code: &str, detail: impl ToString) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            detail: Some(detail.to_string()),
            field: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ValidationResult {
    pub guard: TargetGuard,
    pub valid: bool,
    pub ready_for_export: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompileResult {
    pub guard: TargetGuard,
    pub render: Option<RenderInfo>,
    pub diagnostics: Vec<Diagnostic>,
}
