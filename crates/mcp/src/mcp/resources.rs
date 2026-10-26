//! Ressources virtuelles uniquement ; aucun URI n’est converti en chemin disque.
use rmcp::model::Resource;

const RESOURCES: [(&str, &str, &str); 6] = [
    (
        "content-capabilities",
        "Structured content contracts",
        "Shared semantic capability definitions with explicit support per adapter; documentation does not imply execution.",
    ),
    (
        "guide",
        "Usage guide",
        "Session rules, current permissions and discovery workflow.",
    ),
    (
        "types",
        "Document types",
        "Fields, defaults, capabilities and brief interchangeable model cards.",
    ),
    (
        "commands",
        "Documented command catalogue",
        "Complete capability documentation, effective MCP schemas and current availability/authorization.",
    ),
    (
        "document",
        "Current document",
        "Coherent current snapshot: document reference, state, text, structure, metadata and content source. No model source or binary assets.",
    ),
    (
        "models",
        "Visual model catalogue",
        "Brief model cards for all types, with the active supplied model identified for the current type.",
    ),
];
pub fn list() -> Vec<Resource> {
    RESOURCES
        .into_iter()
        .map(|(id, name, description)| {
            Resource::new(format!("typstographe://{id}"), name)
                .with_description(description)
                .with_mime_type("application/json")
        })
        .collect()
}
pub fn known(uri: &str) -> bool {
    RESOURCES
        .iter()
        .any(|(id, _, _)| uri == format!("typstographe://{id}"))
}
