//! Validate transport inventory integrity, independently of the runtime catalogue.
use super::{Catalog, Dependencies, INVENTORY, references};
use crate::errors::{CoreError, PackageError, StorageError};
use std::collections::BTreeSet;

fn relative(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 512
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
        && !path.contains(':')
}
pub(crate) fn valid_path(path: &str) -> bool {
    relative(path)
        && (path == INVENTORY || path.starts_with("packages/") || path.starts_with("capabilities/"))
}
fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        && !matches!(value, "." | "..")
}
fn invalid() -> CoreError {
    PackageError::InventoryInvalid.into()
}

pub fn validate(template: &str, files: &Dependencies) -> Result<(), CoreError> {
    for (path, bytes) in files.iter() {
        if !valid_path(path) {
            return Err(StorageError::UnexpectedEntry { name: path.clone() }.into());
        }
        if bytes.len() > crate::assets::MAX_IMAGE_BYTES {
            return Err(StorageError::EntryTooLarge { name: path.clone() }.into());
        }
    }
    let required: Vec<_> = references(template)
        .into_iter()
        .map(|p| {
            p.trim_start_matches('/')
                .trim_start_matches("./")
                .to_string()
        })
        .filter(|p| p.starts_with("packages/") || p.starts_with("capabilities/"))
        .collect();
    if files.is_empty() {
        return match required.first() {
            Some(path) => Err(PackageError::ResourceMissing { path: path.clone() }.into()),
            None => Ok(()),
        };
    }
    let catalog: Catalog =
        serde_json::from_slice(files.get(INVENTORY).ok_or_else(invalid)?).map_err(|_| invalid())?;
    if catalog.version != 1 || catalog.packages.len() > 256 || catalog.capabilities.len() > 256 {
        return Err(invalid());
    }
    let names: BTreeSet<_> = catalog.packages.iter().map(|p| p.name.as_str()).collect();
    let ids: BTreeSet<_> = catalog.capabilities.iter().map(|c| c.id.as_str()).collect();
    let roots: BTreeSet<_> = catalog.packages.iter().map(|p| p.root.as_str()).collect();
    let entries: BTreeSet<_> = catalog
        .capabilities
        .iter()
        .map(|c| c.entrypoint.as_str())
        .collect();
    if names.len() != catalog.packages.len()
        || roots.len() != catalog.packages.len()
        || ids.len() != catalog.capabilities.len()
        || entries.len() != catalog.capabilities.len()
    {
        return Err(invalid());
    }
    let exists = |path: &str| {
        if files.contains_key(path) {
            Ok(())
        } else {
            Err(CoreError::from(PackageError::ResourceMissing {
                path: path.into(),
            }))
        }
    };
    for package in &catalog.packages {
        if !token(&package.name)
            || !token(&package.version)
            || package.root != format!("packages/{}-{}", package.name, package.version)
            || !relative(&package.entrypoint)
            || package.license.trim().is_empty()
            || package.sha256.len() != 64
            || !package.sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || package
                .dependencies
                .iter()
                .any(|name| !names.contains(name.as_str()))
        {
            return Err(invalid());
        }
        exists(&format!("{}/{}", package.root, package.entrypoint))?;
        // Packages may name their license LICENSE, LICENSE.txt, etc.
        if !files.keys().any(|p| {
            p.strip_prefix(&format!("{}/", package.root))
                .is_some_and(|p| !p.contains('/') && p.to_ascii_uppercase().starts_with("LICENSE"))
        }) {
            return Err(invalid());
        }
    }
    for capability in &catalog.capabilities {
        if !token(&capability.id)
            || !valid_path(&capability.entrypoint)
            || !capability.entrypoint.starts_with("capabilities/")
            || capability.signature.trim().is_empty()
            || capability
                .packages
                .iter()
                .any(|name| !names.contains(name.as_str()))
        {
            return Err(invalid());
        }
        exists(&capability.entrypoint)?;
    }
    for path in files.keys() {
        if path != INVENTORY
            && path != "packages/PROVENANCE.md"
            && !entries.contains(path.as_str())
            && !catalog
                .packages
                .iter()
                .any(|p| path.starts_with(&format!("{}/", p.root)))
        {
            return Err(invalid());
        }
    }
    if !catalog.packages.is_empty() {
        exists("packages/PROVENANCE.md")?;
    }
    for path in required {
        exists(&path)?;
    }
    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
