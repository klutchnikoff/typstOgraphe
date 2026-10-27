//! Read-only bibliography index and model policy. Every mutation stays in the document/Yjs path.
use typstographe_core::{
    bibliography::{self, Entry, ReferenceFile},
    content::Node,
    document_types::Metadata,
};
#[tauri::command]
pub fn bibliography_search(files: Vec<ReferenceFile>, query: String) -> Result<Vec<Entry>, String> {
    let entries = bibliography::index(&files)?;
    Ok(bibliography::search(&entries, &query))
}
#[tauri::command]
pub fn bibliography_index(files: Vec<ReferenceFile>) -> Result<Vec<Entry>, String> {
    bibliography::index(&files)
}
#[tauri::command]
pub fn validate_bibliography(document: Node, metadata: Metadata) -> Result<(), String> {
    metadata.validate()?;
    bibliography::validate_document(&document, &metadata)
}
#[tauri::command]
pub fn model_policy(
    template: String,
) -> Result<typstographe_core::document_types::ModelPresentation, String> {
    if template.len() > 2 * 1024 * 1024 {
        return Err("Model too large".into());
    }
    typstographe_core::rendering::TypstWorld::default()
        .model_presentation(&template)
        .map_err(|e| e.to_string())
}
