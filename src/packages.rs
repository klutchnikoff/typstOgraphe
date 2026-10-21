//! Dépendances Typst fournies avec l’application, puis copiées dans le document.
//! Aucun téléchargement ni recours au cache de paquets de la machine.
use rust_embed::RustEmbed;
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

pub type Dependencies = Arc<BTreeMap<String, Arc<[u8]>>>;

#[derive(RustEmbed)]
#[folder = "templates/packages/"]
struct Bundled;

pub fn bundled() -> Dependencies {
    static FILES: OnceLock<Dependencies> = OnceLock::new();
    FILES
        .get_or_init(|| {
            Arc::new(
                Bundled::iter()
                    .map(|name| {
                        let data = Bundled::get(&name).expect("ressource embarquée").data;
                        (format!("packages/{name}"), Arc::from(data.as_ref()))
                    })
                    .collect(),
            )
        })
        .clone()
}

pub fn valid_path(path: &str) -> bool {
    path.strip_prefix("packages/")
        .is_some_and(|name| Bundled::get(name).is_some())
}

/// Le catalogue embarqué contient actuellement Datify et sa dépendance complète.
/// Repérer les chaînes Typst évite de traiter les mentions dans les commentaires.
fn required(template: &str) -> bool {
    fn visit(node: &typst_syntax::SyntaxNode) -> bool {
        if let Some(value) = node.cast::<typst_syntax::ast::Str>() {
            let value = value.get();
            let path = value.trim_start_matches('/').trim_start_matches("./");
            if path.starts_with("packages/datify-") {
                return true;
            }
        }
        node.children().any(visit)
    }
    visit(typst_syntax::Source::detached(template).root())
}

pub fn for_template(template: &str) -> Dependencies {
    if required(template) {
        bundled()
    } else {
        Default::default()
    }
}

/// Conserver les exemplaires du document lors de l’import d’un modèle dérivé.
pub fn with_existing(template: &str, existing: &Dependencies) -> Dependencies {
    let mut files = for_template(template);
    if !files.is_empty() && !existing.is_empty() {
        for (name, data) in Arc::make_mut(&mut files) {
            if let Some(saved) = existing.get(name) {
                *data = saved.clone();
            }
        }
    }
    files
}

pub fn validate(template: &str, files: &Dependencies) -> Result<(), String> {
    for (name, data) in files.iter() {
        if !valid_path(name) {
            return Err(crate::tr!("storage-unknown-entry"));
        }
        if data.len() > crate::assets::MAX_IMAGE_BYTES {
            return Err(crate::tr!("storage-entry-too-large", name = name.as_str()));
        }
    }
    // Les locales inutilisées aujourd’hui doivent rester disponibles demain.
    if required(template) || !files.is_empty() {
        for name in bundled().keys() {
            if !files.contains_key(name) {
                return Err(crate::tr!("package-resource-missing", path = name.as_str()));
            }
        }
    }
    Ok(())
}
