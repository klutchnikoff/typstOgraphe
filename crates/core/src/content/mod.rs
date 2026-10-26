//! Contenu structuré indépendant des bibliothèques graphiques.
pub mod archive;
pub mod catalogue;
pub mod legacy;
mod mapping;
pub use mapping::source_mapped;
pub mod operations;
mod schema;
pub use schema::{Mark, Node, source};

/// Conserver seulement les ressources du contenu, de la signature et du modèle.
pub fn retained_assets(
    document: &Node,
    metadata: &crate::document_types::Metadata,
    template: &str,
    assets: &crate::assets::Assets,
    dependencies: &crate::packages::Dependencies,
) -> Result<crate::assets::Assets, String> {
    fn paths(node: &Node, required: &mut std::collections::BTreeSet<String>) {
        if node.kind == "figure"
            && let Some(path) = node
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("src"))
                .and_then(serde_json::Value::as_str)
        {
            required.insert(path.into());
        }
        for child in &node.content {
            paths(child, required)
        }
    }
    let mut required = std::collections::BTreeSet::new();
    paths(document, &mut required);
    let source = source(document, assets)?;
    crate::rendering::retained_assets_for_paths(
        &source,
        metadata,
        template,
        assets,
        dependencies,
        required,
    )
    .map_err(|e| e.to_string())
}
