use super::{Error, schema::*, store::Store};
use crate::{
    assets::{Assets, MAX_DOCUMENT_BYTES, MAX_IMAGE_BYTES},
    content::archive::Archive,
    errors::{CoreError, StorageError},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn inconsistent(field: &str) -> Error {
    crate::content::contract::Error::InconsistentState {
        field: field.into(),
    }
    .into()
}
pub(super) fn prepare(mut archive: Archive, initialize: bool) -> Result<Archive, Error> {
    archive.validate_embedded()?;
    if archive.yjs.is_empty() {
        if !initialize {
            return Err(inconsistent("yjs"));
        }
        archive.yjs = crate::content::shared::verify_or_initialize(&archive.state(), &[])?;
    }
    archive.assets = crate::content::retained_assets(
        &archive.document,
        &archive.metadata,
        &archive.template,
        &archive.assets,
        &archive.dependencies,
    )
    .map_err(|reason| crate::content::contract::Error::InvalidDocument { reason })?;
    Ok(archive)
}
pub(super) fn encode(store: &mut Store, archive: &Archive) -> Result<ObjectId, Error> {
    let body = archive.validate_embedded()?;
    let references = crate::bibliography::resources(&archive.metadata);
    let mut resources = ResourceMap::new();
    for (path, bytes) in archive
        .assets
        .iter()
        .chain(archive.dependencies.iter())
        .chain(references.iter())
    {
        if resources
            .insert(path.clone(), store.insert(bytes.clone())?)
            .is_some()
        {
            return Err(StorageError::DuplicateEntry { name: path.clone() }.into());
        }
    }
    let descriptor = StateDescriptor {
        version: 1,
        contract_version: crate::content::contract::VERSION,
        document_type: archive.metadata.document_type(),
        document: store.insert_json(&archive.document)?,
        metadata: store.insert_json(&archive.metadata)?,
        yjs: Checkpoint {
            encoding: "yjs-v1".into(),
            object: store.insert(archive.yjs.clone())?,
        },
        model: Model {
            id: archive.model_id.clone(),
            source: store.insert(archive.template.as_bytes())?,
        },
        sources: Sources {
            main: store.insert(archive.metadata.main_source().into_bytes())?,
            body: store.insert(body.into_bytes())?,
        },
        resources: store.insert_json(&resources)?,
        producer: Producer {
            application: env!("CARGO_PKG_VERSION").into(),
            generator: 1,
            typst: typst::utils::version().raw().into(),
        },
    };
    let bytes = super::store::serialize(&descriptor)?;
    check_budget(store, &descriptor, &resources, bytes.len() as u64)?;
    store.insert(bytes)
}
fn descriptor(store: &Store, id: &ObjectId) -> Result<(StateDescriptor, ResourceMap), Error> {
    let descriptor_bytes = store.size(id)?;
    if descriptor_bytes > MAX_INDEX_BYTES {
        return Err(StorageError::EntryTooLarge {
            name: "descriptor".into(),
        }
        .into());
    }
    let descriptor: StateDescriptor = store.read_json(id)?;
    descriptor.validate()?;
    if store.size(&descriptor.resources)? > MAX_INDEX_BYTES {
        return Err(StorageError::EntryTooLarge {
            name: "resources".into(),
        }
        .into());
    }
    let resources = store
        .read_json::<ResourceManifest>(&descriptor.resources)?
        .0;
    for path in resources.keys() {
        if !crate::assets::valid_path(path)
            && !crate::bibliography::valid_path(path)
            && !crate::packages::valid_embedded_path(path)
        {
            return Err(StorageError::UnexpectedEntry { name: path.clone() }.into());
        }
    }
    check_budget(store, &descriptor, &resources, descriptor_bytes)?;
    Ok((descriptor, resources))
}
fn check_budget(
    store: &Store,
    descriptor: &StateDescriptor,
    resources: &ResourceMap,
    descriptor_bytes: u64,
) -> Result<(), Error> {
    if store.size(&descriptor.yjs.object)? > crate::content::shared::MAX_STATE_BYTES as u64 {
        return Err(StorageError::EntryTooLarge {
            name: "state.yjs".into(),
        }
        .into());
    }
    let mut bytes = descriptor_bytes;
    // Logical size counts each file even when several paths share the same object.
    for id in descriptor.refs().into_iter().chain(resources.values()) {
        bytes = bytes
            .checked_add(store.size(id)?)
            .ok_or(StorageError::LetterTooLarge)?;
    }
    if bytes > MAX_DOCUMENT_BYTES {
        return Err(StorageError::LetterTooLarge.into());
    }
    if resources.len() > MAX_OBJECTS || store.size(&descriptor.resources)? > MAX_INDEX_BYTES {
        return Err(StorageError::FileCount.into());
    }
    for (path, id) in resources {
        if crate::assets::valid_path(path) && store.size(id)? > MAX_IMAGE_BYTES as u64 {
            return Err(StorageError::EntryTooLarge { name: path.clone() }.into());
        }
    }
    Ok(())
}
fn text(store: &Store, id: &ObjectId) -> Result<String, Error> {
    String::from_utf8(store.read(id)?.to_vec()).map_err(|e| CoreError::external(e).into())
}
pub(super) fn decode(store: &Store, id: &ObjectId) -> Result<Archive, Error> {
    let (descriptor, resources) = descriptor(store, id)?;
    let mut assets = Assets::default();
    let mut dependencies = BTreeMap::new();
    let mut bibliography = BTreeMap::new();
    for (path, id) in resources {
        let bytes = store.read(&id)?;
        if crate::assets::valid_path(&path) {
            assets.insert(path, bytes);
        } else if crate::bibliography::valid_path(&path) {
            bibliography.insert(path, bytes);
        } else {
            dependencies.insert(path, bytes);
        }
    }
    let archive = Archive {
        document: store.read_json(&descriptor.document)?,
        metadata: store.read_json(&descriptor.metadata)?,
        template: text(store, &descriptor.model.source)?,
        model_id: descriptor.model.id,
        assets,
        dependencies: Arc::new(dependencies),
        yjs: store.read(&descriptor.yjs.object)?.to_vec(),
    };
    if archive.yjs.is_empty() {
        return Err(inconsistent("yjs"));
    }
    if archive.metadata.document_type() != descriptor.document_type {
        return Err(inconsistent("document_type"));
    }
    if crate::bibliography::resources(&archive.metadata) != bibliography {
        return Err(inconsistent("bibliography"));
    }
    let body = archive.validate_embedded()?;
    if body != text(store, &descriptor.sources.body)? {
        return Err(inconsistent("source"));
    }
    if archive.metadata.main_source() != text(store, &descriptor.sources.main)? {
        return Err(StorageError::MainMismatch.into());
    }
    Ok(archive)
}
/// Traverse only the known graph schema. Unreachable objects are never copied.
pub(super) fn reachable<'a>(
    store: &Store,
    roots: impl Iterator<Item = &'a ObjectId>,
) -> Result<BTreeSet<ObjectId>, Error> {
    let mut reachable = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for root in roots {
        if !visited.insert(root.clone()) {
            continue;
        }
        let (descriptor, resources) = descriptor(store, root)?;
        reachable.insert(root.clone());
        reachable.extend(descriptor.refs().into_iter().cloned());
        reachable.extend(resources.into_values());
        if reachable.len() > MAX_OBJECTS {
            return Err(StorageError::FileCount.into());
        }
    }
    Ok(reachable)
}
