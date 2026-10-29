//! Shared headless contract. Adapters supply permissions, paths and presentation;
//! all document edits, queries, contexts and persistence use the structured owner.
mod catalogue;
mod examples;
mod queries;
mod render;
mod session;
pub use super::commands::MAX_BATCH_STEPS;
use super::contract::Error;
use crate::protocol::{CommandError, ErrorCode};
pub use catalogue::{catalogue, describe, is_query, type_description, types};
pub use queries::run as query_state;
pub use queries::{find, resolve_call, zones};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
pub use session::{Save, Session};
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub id: String,
    #[serde(default = "empty")]
    pub params: Value,
}
fn empty() -> Value {
    json!({})
}
pub fn content_error(error: Error) -> CommandError {
    CommandError {
        code: ErrorCode::Document,
        params: json!({"reason":"structured_content","content":error}),
    }
}
pub fn error_value(error: &CommandError) -> Value {
    if let Some(content) = error.params.get("content") {
        content.clone()
    } else {
        json!({"code":error.code,"params":error.params})
    }
}
pub fn invalid(parameter: &str) -> Error {
    Error::InvalidParameters {
        parameter: parameter.into(),
    }
}
pub fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, CommandError> {
    serde_json::from_value(value).map_err(CommandError::invalid)
}
fn archive_error(error: super::archive::Error) -> CommandError {
    match error {
        super::archive::Error::Content(e) => content_error(e),
        super::archive::Error::Access(e) => e,
        super::archive::Error::Core(e) => {
            let code = if matches!(
                e,
                crate::errors::CoreError::StorageError(
                    crate::errors::StorageError::OpenFailed { .. }
                        | crate::errors::StorageError::ReplaceFailed { .. }
                        | crate::errors::StorageError::TemporaryFailed { .. }
                )
            ) {
                ErrorCode::Io
            } else {
                ErrorCode::Document
            };
            CommandError::with_cause(code, "structured_archive", e)
        }
    }
}
pub fn check_params(id: &str, params: &Value) -> Result<(), CommandError> {
    static VALIDATORS: std::sync::OnceLock<
        std::collections::BTreeMap<String, jsonschema::Validator>,
    > = std::sync::OnceLock::new();
    let validators = VALIDATORS.get_or_init(|| {
        catalogue()["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|spec| {
                (
                    spec["id"].as_str().unwrap().into(),
                    jsonschema::validator_for(&spec["params_schema"]).expect("shared API schema"),
                )
            })
            .collect()
    });
    let validator = validators
        .get(id)
        .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
    validator
        .validate(params)
        .map_err(|_| content_error(invalid(id)))
}
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
