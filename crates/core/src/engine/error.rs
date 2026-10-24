//! Codes de protocole et données ; aucune phrase Fluent dans le moteur.
use crate::document::{edits::EditError, structure::TargetError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unavailable,
    InvalidParams,
    NotFound,
    Io,
    Document,
    Template,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[error("{code:?}: {params}")]
pub struct CommandError {
    pub code: ErrorCode,
    pub params: Value,
}
impl CommandError {
    pub fn detail(code: ErrorCode, reason: &str, detail: impl std::fmt::Display) -> Self {
        Self {
            code,
            params: json!({"reason": reason, "detail": detail.to_string()}),
        }
    }
    pub fn with_cause(code: ErrorCode, reason: &str, cause: crate::errors::CoreError) -> Self {
        Self {
            code,
            params: json!({"reason":reason,"detail":cause.to_string(),"cause":cause}),
        }
    }
    pub fn new(code: ErrorCode, reason: &str) -> Self {
        Self {
            code,
            params: json!({"reason": reason}),
        }
    }
    pub fn invalid(error: impl std::fmt::Display) -> Self {
        Self {
            code: ErrorCode::InvalidParams,
            params: json!({"reason": "invalid_params", "detail": error.to_string()}),
        }
    }
    pub fn reason(&self) -> &str {
        self.params["reason"].as_str().unwrap_or("unknown")
    }
}
impl From<crate::assets::ImageError> for CommandError {
    fn from(error: crate::assets::ImageError) -> Self {
        use crate::assets::ImageError;
        let (code, reason) = match &error {
            ImageError::TooLarge => (ErrorCode::Document, "image_too_large"),
            ImageError::Unsupported => (ErrorCode::InvalidParams, "image_format"),
            ImageError::InvalidSvg => (ErrorCode::InvalidParams, "image_svg"),
            ImageError::Decode(_) => (ErrorCode::InvalidParams, "image_decode"),
            ImageError::Io(_) => (ErrorCode::Io, "image_io"),
        };
        Self::with_cause(code, reason, error.into())
    }
}
impl From<TargetError> for CommandError {
    fn from(error: TargetError) -> Self {
        Self::new(
            ErrorCode::Document,
            match error {
                TargetError::ForeignDocument => "foreign_document",
                TargetError::StaleRevision => "stale_revision",
                TargetError::InvalidPosition => "invalid_selection",
                TargetError::UnknownBlock => "unknown_block",
                TargetError::UnknownZone => "unknown_zone",
            },
        )
    }
}
impl From<EditError> for CommandError {
    fn from(error: EditError) -> Self {
        if let EditError::Target(error) = error {
            return error.into();
        }
        Self::new(
            ErrorCode::Document,
            match error {
                EditError::ProtectedBoundary => "protected_boundary",
                EditError::ReadOnly => "file_read_only",
                EditError::InvalidText => "invalid_text",
                EditError::StaleDraft => "stale_revision",
                EditError::InvalidSelection => "invalid_selection",
                _ => "invalid_projection",
            },
        )
    }
}
