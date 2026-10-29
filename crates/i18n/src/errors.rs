//! Mise en phrases commune des erreurs du moteur ; utilisable par tous les fronts.
use crate::protocol::{CommandError, ErrorCode};
use i18n_embed::fluent::FluentLanguageLoader;
use std::collections::HashMap;

pub fn command_error_with(loader: &FluentLanguageLoader, error: &CommandError) -> String {
    if let Some(content) = error.params.get("content").and_then(|v| {
        serde_json::from_value::<typstographe_core::content::contract::Error>(v.clone()).ok()
    }) {
        return super::content_errors::content_error_with(loader, &content);
    }

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
        return if matches!(
            error.reason(),
            "image_decode" | "image_io" | "asset_validation"
        ) {
            loader.get_args("engine-image-error", HashMap::from([("error", detail)]))
        } else {
            detail
        };
    }
    if matches!(error.reason(), "file_export" | "export_compile") {
        return error.params["detail"].as_str().unwrap_or("").into();
    }
    loader.get(match error.reason() {
        "image_too_large" => "image-too-large",
        "batch_step_limit" => "engine-batch-step-limit",
        "batch_size_limit" => "engine-batch-size-limit",
        "batch_step_forbidden" => "engine-batch-step-forbidden",
        "file_exists" => "file-already-exists",
        "file_locked" => "file-locked",
        "file_changed" => "file-changed",
        "file_read_only" | "read_only" => "file-read-only",
        "file_access" => "file-access-error",
        "unknown_command" => "engine-unknown-command",
        "cli_params_object" => "cli-params-object",
        "cli_params_too_large" => "cli-params-too-large",
        "cli_params_io" => "cli-params-io",
        "cli_params_conflict" => "cli-params-conflict",
        "cli_session_reference" => "cli-session-reference",
        "cli_call_family" => "cli-call-family",
        "cli_call_unavailable" => "cli-call-unavailable",
        "cli_target_options" => "cli-target-options",
        "cli_script_io" => "cli-script-io",
        "cli_script_too_large" => "engine-batch-size-limit",
        "cli_script_version" => "cli-script-version",
        "cli_script_save" => "cli-script-save",
        "cli_render_page" => "cli-render-page",
        "cli_render_io" => "cli-render-io",
        "mcp_file_access" => "mcp-file-access",
        "mcp_root" => "mcp-root",
        "mcp_read_only" => "mcp-read-only",
        "mcp_document_required" => "mcp-document-required",
        "mcp_document_foreign" => "mcp-document-foreign",
        "mcp_stdio_required" => "mcp-stdio-required",
        "unsaved_changes" => "unsaved-message",
        "creation_model" => "type-no-model",
        "export_required_fields" => "export-required-fields",
        "save_path_required" => "save-dialog",
        "image_format" => "image-format-unsupported",
        "image_svg" => "image-invalid-svg",
        "foreign_document" | "stale_revision" | "unknown_zone" | "unknown_block" => {
            "edit-stale-draft"
        }
        _ => match error.code {
            ErrorCode::InvalidParams => "engine-invalid-params",
            _ => "engine-unavailable",
        },
    })
}
