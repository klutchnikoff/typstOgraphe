//! Interfaces Typst des capacités et paquets privés, copiés dans chaque document.
//! Le catalogue est partagé avec les créateurs de modèles ; aucun téléchargement.
use crate::errors::{CoreError, PackageError, StorageError};
mod embedded;
pub use embedded::validate as validate_embedded;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

pub type Dependencies = Arc<BTreeMap<String, Arc<[u8]>>>;
const INVENTORY: &str = "dependencies.json";

#[derive(RustEmbed)]
#[folder = "templates/packages/"]
struct Packages;
#[derive(RustEmbed)]
#[folder = "templates/capabilities/"]
struct Capabilities;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    version: u32,
    capabilities: Vec<Capability>,
    packages: Vec<Package>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    id: String,
    entrypoint: String,
    signature: String,
    packages: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    name: String,
    version: String,
    root: String,
    entrypoint: String,
    license: String,
    archive: String,
    sha256: String,
    dependencies: Vec<String>,
}
fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../templates/dependencies.json"))
            .expect("catalogue Typst embarqué valide")
    })
}

pub fn bundled() -> Dependencies {
    static FILES: OnceLock<Dependencies> = OnceLock::new();
    FILES
        .get_or_init(|| {
            let mut files: BTreeMap<_, Arc<[u8]>> = Packages::iter()
                .map(|name| {
                    let data = Packages::get(&name).expect("paquet embarqué").data;
                    (format!("packages/{name}"), Arc::from(data.as_ref()))
                })
                .collect();
            files.extend(Capabilities::iter().map(|name| {
                let data = Capabilities::get(&name).expect("capacité embarquée").data;
                (format!("capabilities/{name}"), Arc::from(data.as_ref()))
            }));
            files.insert(
                INVENTORY.into(),
                include_bytes!("../templates/dependencies.json")
                    .as_slice()
                    .into(),
            );
            Arc::new(files)
        })
        .clone()
}

pub fn valid_path(path: &str) -> bool {
    // Liste exacte, sans traversée de répertoire ni lecture du système de fichiers.
    bundled().contains_key(path)
}

/// Résoudre les capacités et les dépendances transitives à partir du catalogue,
/// sans condition particulière pour Datify ou pour une autre capacité.
fn select<'a>(paths: impl IntoIterator<Item = &'a str>) -> (Catalog, Dependencies) {
    let catalog = catalog();
    let mut capabilities = BTreeSet::new();
    let mut packages = BTreeSet::new();
    fn add_package(name: &str, catalog: &Catalog, selected: &mut BTreeSet<String>) {
        if !selected.insert(name.into()) {
            return;
        }
        let package = catalog
            .packages
            .iter()
            .find(|p| p.name == name)
            .expect("dépendance déclarée dans le catalogue");
        for dependency in &package.dependencies {
            add_package(dependency, catalog, selected);
        }
    }
    for path in paths {
        let path = path.trim_start_matches('/').trim_start_matches("./");
        for capability in &catalog.capabilities {
            if path == capability.entrypoint {
                capabilities.insert(capability.id.clone());
                for package in &capability.packages {
                    add_package(package, catalog, &mut packages);
                }
            }
        }
        for package in &catalog.packages {
            if path
                .strip_prefix(&package.root)
                .is_some_and(|suffix| suffix.starts_with('/'))
            {
                add_package(&package.name, catalog, &mut packages);
            }
        }
    }
    let selected = Catalog {
        version: catalog.version,
        capabilities: catalog
            .capabilities
            .iter()
            .filter(|c| capabilities.contains(&c.id))
            .cloned()
            .collect(),
        packages: catalog
            .packages
            .iter()
            .filter(|p| packages.contains(&p.name))
            .cloned()
            .collect(),
    };
    let mut files = BTreeMap::new();
    if !capabilities.is_empty() || !packages.is_empty() {
        for (path, data) in bundled().iter() {
            if selected.capabilities.iter().any(|c| c.entrypoint == *path)
                || selected.packages.iter().any(|p| {
                    path.strip_prefix(&p.root)
                        .is_some_and(|suffix| suffix.starts_with('/'))
                })
                || path == "packages/PROVENANCE.md"
            {
                files.insert(path.clone(), data.clone());
            }
        }
        files.insert(
            INVENTORY.into(),
            serde_json::to_vec_pretty(&selected)
                .expect("inventaire sérialisable")
                .into(),
        );
    }
    (selected, Arc::new(files))
}

fn references(template: &str) -> Vec<String> {
    fn visit(node: &typst_syntax::SyntaxNode, paths: &mut Vec<String>) {
        if let Some(value) = node.cast::<typst_syntax::ast::Str>() {
            paths.push(value.get().to_string());
        }
        for child in node.children() {
            visit(child, paths);
        }
    }
    let mut paths = Vec::new();
    visit(typst_syntax::Source::detached(template).root(), &mut paths);
    paths
}
pub fn for_template(template: &str) -> Dependencies {
    select(references(template).iter().map(String::as_str)).1
}

pub fn validate(template: &str, files: &Dependencies) -> Result<(), CoreError> {
    for (name, data) in files.iter() {
        if !valid_path(name) {
            return Err(CoreError::from(StorageError::UnknownEntry));
        }
        if data.len() > crate::assets::MAX_IMAGE_BYTES {
            return Err(CoreError::from(StorageError::EntryTooLarge {
                name: (name.as_str()).to_string(),
            }));
        }
    }
    // Même une erreur dans le modèle ne permet pas d’oublier les locales ou wrappers.
    let paths = references(template);
    let (expected, required) = select(paths.iter().chain(files.keys()).map(String::as_str));
    for name in required.keys() {
        if !files.contains_key(name) {
            return Err(CoreError::from(PackageError::ResourceMissing {
                path: (name.as_str()).to_string(),
            }));
        }
    }
    if !files.is_empty() {
        let inventory = files
            .get(INVENTORY)
            .and_then(|bytes| serde_json::from_slice::<Catalog>(bytes).ok());
        if inventory.as_ref() != Some(&expected) {
            return Err(CoreError::from(PackageError::InventoryInvalid));
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn capability_catalog_resolves_transitive_packages_and_exports_an_exact_inventory() {
        let template = "#import \"capabilities/dates.typ\": format-date";
        let files = for_template(template);
        validate(template, &files).unwrap();
        let inventory: Catalog = serde_json::from_slice(&files[INVENTORY]).unwrap();
        assert_eq!(inventory.capabilities.len(), 1);
        assert_eq!(inventory.capabilities[0].id, "dates");
        assert_eq!(
            inventory
                .packages
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["datify", "datify-core"]
        );
        for package in &inventory.packages {
            assert!(files.contains_key(&format!("{}/{}", package.root, package.entrypoint)));
            assert!(files.contains_key(&format!("{}/LICENSE", package.root)));
        }
        assert!(files.contains_key("capabilities/dates.typ"));
        assert!(
            for_template("// \"capabilities/dates.typ\"\n#let courrier(body) = body").is_empty()
        );
        let core = for_template("#import \"packages/datify-core-2.1.0/src/main.typ\": *");
        let inventory: Catalog = serde_json::from_slice(&core[INVENTORY]).unwrap();
        assert!(inventory.capabilities.is_empty());
        assert_eq!(inventory.packages.len(), 1);
        assert_eq!(inventory.packages[0].name, "datify-core");
        validate("", &core).unwrap();
        assert!(!core.contains_key("capabilities/dates.typ"));
        assert!(!core.contains_key("packages/datify-1.3.0/src/main.typ"));
    }

    #[test]
    fn provided_models_use_capability_interfaces_without_importing_vendor_packages() {
        for model in crate::document_types::models::ALL {
            for path in references(model.source) {
                assert!(!path.starts_with("packages/"), "{}", path);
            }
            let files = for_template(model.source);
            validate(model.source, &files).unwrap();
        }
    }
    #[test]
    fn dates_public_contract_accepts_fixed_dates_and_free_text_and_returns_content() {
        let template = r#"
#import "capabilities/dates.typ": format-date
#let courrier(..args) = {
  for value in ("", "mars 2026", "today", "2024-02-29", datetime(year: 2026, month: 10, day: 1)) {
    assert(type(format-date(value)) == content)
    assert(type(format-date(value, lang: "en")) == content)
  }
  format-date("2026-10-01")
}
"#;
        let mut world = crate::rendering::TypstWorld::default();
        world.set_dependencies(&for_template(template));
        let metadata =
            crate::document_types::Metadata::blank(crate::document_types::DocumentType::Courrier);
        world
            .compile_with_template("", &metadata, template)
            .unwrap();
        for invalid in ["2026-02-30", "2026-13-01"] {
            assert!(
                world
                    .compile_with_template("", &metadata, &template.replace("2024-02-29", invalid))
                    .is_err()
            );
        }
    }
    #[test]
    fn native_correspondence_capability_does_not_pull_in_unrelated_packages() {
        let template = "#import \"capabilities/correspondence.typ\": postscript, attachment";
        let files = for_template(template);
        validate(template, &files).unwrap();
        let inventory: Catalog = serde_json::from_slice(&files[INVENTORY]).unwrap();
        assert_eq!(inventory.capabilities[0].id, "correspondence");
        assert!(inventory.packages.is_empty());
        assert!(files.contains_key("capabilities/correspondence.typ"));
        assert!(!files.contains_key("capabilities/dates.typ"));
        assert!(
            !files
                .keys()
                .any(|name| name.starts_with("packages/datify-"))
        );
    }
}
