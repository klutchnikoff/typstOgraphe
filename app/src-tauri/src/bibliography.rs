//! Read-only bibliography index and model policy. Every mutation stays in the document/Yjs path.
use crate::error::CommandError;
use typstographe_core::{
    bibliography::{self, Entry, Record},
    content::Node,
    document_types::Metadata,
};
#[tauri::command]
pub fn bibliography_search(files: Vec<Record>, query: String) -> Result<Vec<Entry>, CommandError> {
    let entries = bibliography::index(&files)?;
    Ok(bibliography::search(&entries, &query))
}
#[tauri::command]
pub fn bibliography_index(files: Vec<Record>) -> Result<Vec<Entry>, CommandError> {
    bibliography::index(&files).map_err(CommandError::Message)
}
#[tauri::command]
pub fn validate_bibliography(document: Node, metadata: Metadata) -> Result<(), CommandError> {
    metadata.validate()?;
    bibliography::validate_document(&document, &metadata).map_err(CommandError::Message)
}
#[tauri::command]
pub fn model_policy(
    metadata: Metadata,
    manifest: String,
) -> Result<serde_json::Value, CommandError> {
    typstographe_core::document_types::model_options::describe(&manifest, &metadata)
        .map_err(CommandError::Message)
}

/// Read-only adapter to the shared bibliography query contract. No filesystem access.
#[tauri::command]
pub fn bibliography_query(
    metadata: Metadata,
    operation: String,
    document: Option<Node>,
    params: serde_json::Value,
) -> Result<serde_json::Value, CommandError> {
    use typstographe_core::content::{
        api,
        contract::{Settings, State},
    };
    if !operation.starts_with("bibliography.") || !api::is_query(&operation) {
        return Err(CommandError::Message(
            "Unsupported bibliography query".into(),
        ));
    }
    api::check_params(&operation, &params).map_err(CommandError::from)?;
    metadata.validate()?;
    let kind = metadata.document_type();
    let state = State::new(
        document.unwrap_or_else(|| {
            serde_json::from_value(
                serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}),
            )
            .unwrap()
        }),
        Settings {
            metadata,
            template: kind.default_model(),
            model_manifest: kind.default_model_manifest().into(),
            model_id: kind.models().next().unwrap().model.id().into(),
        },
    );
    api::query_state(&state, &Default::default(), &operation, &params)
        .map_err(|e| CommandError::from(api::content_error(e)))
}
