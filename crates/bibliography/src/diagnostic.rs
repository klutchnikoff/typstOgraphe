use serde::{Deserialize, Serialize};

/// Stable machine-readable codes; adapters provide localized messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    InvalidIdentity,
    InvalidKey,
    DuplicateIdentity,
    DuplicateKey,
    InvalidSyntax,
    InvalidField,
    UnsupportedField,
    LimitExceeded,
    MissingRecord,
    StaleBase,
    Incomplete,
    ConversionReview,
    ConversionLoss,
    TypeMapping,
    UnresolvedConflict,
    InvalidDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Information,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Diagnostic {
    pub code: Code,
    pub severity: Severity,
    pub key: String,
    /// Native field or JSON pointer, when the diagnostic concerns a field.
    pub field: Option<String>,
    /// Technical detail, never a replacement for an adapter's localized label.
    pub detail: String,
}

#[derive(
    Clone, Debug, PartialEq, Eq, thiserror::Error, Serialize, Deserialize, schemars::JsonSchema,
)]
#[error("{code:?}: {detail}")]
pub struct Error {
    pub code: Code,
    pub key: Option<String>,
    pub field: Option<String>,
    pub detail: String,
}

impl Error {
    pub(crate) fn new(code: Code, detail: impl Into<String>) -> Self {
        Self {
            code,
            key: None,
            field: None,
            detail: detail.into(),
        }
    }

    pub(crate) fn at(mut self, key: &str, field: Option<&str>) -> Self {
        self.key = Some(key.into());
        self.field = field.map(Into::into);
        self
    }
}

pub type Result<T> = std::result::Result<T, Error>;
