//! Contenu structuré indépendant des bibliothèques graphiques.
pub mod api;
pub mod archive;
mod attributes;
pub mod availability;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod availability_tests;
pub mod catalogue;
pub mod commands;
pub mod contract;
pub mod editing;
pub mod editorial;
mod mapping;
pub mod project;
pub use mapping::source_mapped;
mod schema;
pub mod shared;
pub mod tables;
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
