//! Codes de protocole et données ; aucune phrase Fluent dans le moteur.
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
    pub fn at_batch_step(self, index: usize, id: &str) -> Self {
        Self {
            code: self.code,
            params: json!({"reason":"batch_step","step":index,"command":id,"error":self}),
        }
    }
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

/// Formats shared by every adapter's explicit export operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Pdf,
    Typst,
    #[default]
    Both,
}

pub const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024;
