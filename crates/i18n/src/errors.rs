//! Mise en phrases commune des erreurs du moteur ; utilisable par tous les fronts.
use crate::engine::{CommandError, ErrorCode};
use i18n_embed::fluent::FluentLanguageLoader;
use std::collections::HashMap;

pub fn command_error_with(loader: &FluentLanguageLoader, error: &CommandError) -> String {
    if error.reason() == "batch_step"
        && let Ok(cause) = serde_json::from_value::<CommandError>(error.params["error"].clone())
    {
        return loader.get_args(
            "engine-batch-step-error",
            HashMap::from([
                (
                    "step",
                    (error.params["step"].as_u64().unwrap_or(0) + 1).to_string(),
                ),
                ("error", command_error_with(loader, &cause)),
            ]),
        );
    }
    if let Some(cause) = error
        .params
        .get("cause")
        .and_then(|value| serde_json::from_value::<crate::errors::CoreError>(value.clone()).ok())
    {
        let detail = super::error_with(loader, &cause);
        return if error.reason() == "model_validation" {
            format!(
                "{}\n{detail}",
                error.params["signature"].as_str().unwrap_or("")
            )
        } else if matches!(
            error.reason(),
            "image_decode" | "image_io" | "asset_validation"
        ) {
            loader.get_args("engine-image-error", HashMap::from([("error", detail)]))
        } else {
            detail
        };
    }
    if error.reason() == "model_validation" {
        return format!(
            "{}\n{}",
            error.params["signature"].as_str().unwrap_or(""),
            error.params["detail"].as_str().unwrap_or("")
        );
    }
    if matches!(
        error.reason(),
        "file_open"
            | "file_save"
            | "file_document"
            | "file_export"
            | "export_compile"
            | "recovery_load"
            | "recovery_delete"
    ) {
        return error.params["detail"].as_str().unwrap_or("").into();
    }
    if matches!(
        error.reason(),
        "model_io" | "model_utf8" | "model_dependencies"
    ) {
        return error.params["detail"].as_str().unwrap_or("").into();
    }
    if matches!(
        error.reason(),
        "image_decode" | "image_io" | "image_base64" | "asset_validation"
    ) {
        return loader.get_args(
            "engine-image-error",
            HashMap::from([("error", error.params["detail"].as_str().unwrap_or(""))]),
        );
    }
    loader.get(match error.reason() {
        "image_too_large" => "image-too-large",
        "batch_step_limit" => "engine-batch-step-limit",
        "batch_resource_limit" => "engine-batch-resource-limit",
        "batch_size_limit" => "engine-batch-size-limit",
        "batch_step_forbidden" => "engine-batch-step-forbidden",
        "file_exists" => "file-already-exists",
        "file_locked" => "file-locked",
        "file_changed" => "file-changed",
        "file_read_only" => "file-read-only",
        "file_access" => "file-access-error",
        "unknown_command" => "engine-unknown-command",
        "cli_document_type" => "cli-unknown-type",
        "cli_model_required" => "cli-model-required",
        "cli_params_object" => "cli-params-object",
        "cli_params_too_large" => "cli-params-too-large",
        "cli_params_io" => "cli-params-io",
        "cli_params_conflict" => "cli-params-conflict",
        "cli_session_reference" => "cli-session-reference",
        "cli_call_family" => "cli-call-family",
        "cli_call_unavailable" => "cli-call-unavailable",
        "cli_selection_ambiguous" => "cli-selection-ambiguous",
        "cli_selection_missing" => "cli-selection-missing",
        "cli_block_missing" => "cli-block-missing",
        "cli_block_required" => "cli-block-required",
        "cli_spelling_missing" => "cli-spelling-missing",
        "cli_selection_required" => "cli-selection-required",
        "cli_target_unsupported" => "cli-target-unsupported",
        "unsaved_changes" => "unsaved-message",
        "creation_model" => "type-no-model",
        "export_required_fields" => "export-required-fields",
        "save_path_required" => "save-dialog",
        "model_too_large" => "model-too-large",
        "image_format" => "image-format-unsupported",
        "image_svg" => "image-invalid-svg",
        "document_too_large" => "storage-letter-too-large",
        "protected_boundary" => "edit-protected-boundary",
        "invalid_text" => "edit-invalid-text",
        "search_too_large" => "search-too-large",
        "search_one_line" => "search-one-line",
        "search_mismatch" => "edit-stale-draft",
        "spelling_mismatch" | "spelling_language" => "edit-stale-draft",
        "spelling_dictionary" => "spelling-error",
        "foreign_document" | "stale_revision" | "unknown_zone" | "unknown_block" => {
            "edit-stale-draft"
        }
        "invalid_selection" | "invalid_projection" => "edit-invalid-draft",
        "select_link" => "select-link",
        "select_link_remove" => "select-link-remove",
        "link_one_paragraph" => "link-one-paragraph",
        "invalid_url" => "link-invalid-url",
        "bullet_marker_invalid" => "bullet-marker-invalid",
        "bullet_marker_policy" => "model-bullet-marker-not-allowed",
        "model_fixed_page" => "model-fixed-page",
        _ => match error.code {
            ErrorCode::InvalidParams => "engine-invalid-params",
            _ => "engine-unavailable",
        },
    })
}
