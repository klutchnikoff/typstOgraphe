use super::*;
use crate::{
    assets::Assets,
    document_types::{DocumentType, Metadata},
};
use std::{collections::BTreeMap, fs, io::Read, sync::Arc};

pub(super) fn fixture(kind: DocumentType, text: &str) -> Archive {
    let metadata = Metadata::blank(kind);
    let template = kind.default_model();
    let model_id = kind.models().next().unwrap().model.id().into();
    Archive {
        document: serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})).unwrap(),
        metadata, dependencies: crate::packages::for_template(&template), template, model_id,
        assets: Assets::default(), yjs: vec![],
    }
}
pub(super) fn version(project: &Project, name: &str) -> Project {
    project
        .prepare_version(
            SnapshotDetails {
                name: name.into(),
                ..Default::default()
            },
            SnapshotReason::Manual,
        )
        .unwrap()
}
pub(super) fn confirmed(mut archive: Archive) -> Archive {
    archive.yjs = crate::content::shared::verify_or_initialize(&archive.state(), &[]).unwrap();
    archive
}
pub(super) fn entries(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut entry = zip.by_index(i).unwrap();
            let name = entry.name().unwrap().into_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (name, bytes)
        })
        .collect()
}
fn compile(archive: &Archive) {
    let mut world = crate::rendering::TypstWorld::default();
    world.set_assets(&archive.assets);
    world.set_dependencies(&archive.dependencies);
    world
        .compile_with_template(
            &archive.validate_embedded().unwrap(),
            &archive.metadata,
            &archive.template,
        )
        .unwrap();
}
#[test]
fn roundtrip_all_available_types_preserves_identity_history_and_shared_state() {
    let dir = tempfile::tempdir().unwrap();
    for kind in DocumentType::ALL.into_iter().filter(|k| k.can_create()) {
        let path = dir.path().join(format!("{}.typsto", kind.file_stem()));
        let project = version(
            &Project::new(fixture(kind, "Texte é 日本語 😀")).unwrap(),
            "v1",
        );
        let mut lease = DocumentLease::acquire(&path).unwrap();
        project.save(&mut lease).unwrap();
        let read = Project::read(&path).unwrap();
        assert_eq!(read.project_id(), project.project_id());
        assert_eq!(read.versions(), project.versions());
        assert_eq!(read.current().document, project.current().document);
        assert_eq!(read.current().metadata, project.current().metadata);
        assert_eq!(read.current().yjs, project.current().yjs);
        assert_eq!(read.current().template, project.current().template);
        assert_eq!(read.current().dependencies, project.current().dependencies);
        compile(read.current());
        compile(&read.version(read.versions()[0].id).unwrap());
        let before = entries(&path);
        let again = version(&read, "same content, different identity");
        again.save(&mut lease).unwrap();
        let after = entries(&path);
        let objects = |files: BTreeMap<String, Vec<u8>>| {
            files
                .into_iter()
                .filter(|(p, _)| p.starts_with("objects/"))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(objects(before), objects(after));
        assert_eq!(again.versions()[0].state, again.versions()[1].state);
        assert_ne!(again.versions()[0].id, again.versions()[1].id);
        let copy = again.independent_copy();
        assert_ne!(copy.project_id(), again.project_id());
        assert_eq!(copy.versions(), again.versions());
        copy.save(
            &mut DocumentLease::acquire(
                &dir.path().join(format!("{}-copy.typsto", kind.file_stem())),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
pub(super) fn image_fixture(name: &str) -> Archive {
    let mut archive = fixture(DocumentType::Courrier, "Image et légende");
    let path = format!("images/{name}.svg");
    archive.assets.insert(path.clone(), format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><title>{name}</title><rect width="10" height="10"/></svg>"#).into_bytes().into());
    archive.document.content.push(serde_json::from_value(serde_json::json!({"type":"figure","attrs":{"src":path,"alt":"Un carré"},"content":[{"type":"text","text":"Image test"}]})).unwrap());
    archive
}
#[test]
fn history_and_review_roots_keep_images_and_models_while_gc_removes_only_unreachable_objects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("images.typsto");
    let review_id = Uuid::new_v4();
    let first = version(&Project::new(image_fixture("old")).unwrap(), "original")
        .prepare_review_reference(review_id)
        .unwrap();
    let old = first.versions()[0].id;
    let mut next = image_fixture("new");
    next.template.push_str("\n// new pinned model");
    let second = version(&first.with_current(confirmed(next)).unwrap(), "replacement");
    let mut lease = DocumentLease::acquire(&path).unwrap();
    second.save(&mut lease).unwrap();
    let read = Project::read(&path).unwrap();
    assert!(
        read.version(old)
            .unwrap()
            .assets
            .contains_key("images/old.svg")
    );
    let removed = read
        .prepare_remove_version(old)
        .unwrap()
        .with_current(confirmed(fixture(DocumentType::Courrier, "No image")))
        .unwrap();
    let resumed = removed.prepare_review_reference(review_id).unwrap();
    assert_eq!(resumed.review_references(), first.review_references());
    resumed.save(&mut lease).unwrap();
    let read = Project::read(&path).unwrap();
    assert!(
        read.review_reference(review_id)
            .unwrap()
            .assets
            .contains_key("images/old.svg")
    );
    compile(&read.review_reference(review_id).unwrap());
    let second_id = read.versions()[0].id;
    let historical_view = read.clone();
    let pruned = read.prepare_remove_version(second_id).unwrap();
    pruned.save(&mut lease).unwrap();
    let files = entries(&path);
    assert!(files.values().any(|b| b.windows(5).any(|w| w == b">old<")));
    assert!(!files.values().any(|b| b.windows(5).any(|w| w == b">new<")));
    assert!(
        pruned
            .prepare_remove_version(first.review_references()[0].id)
            .is_err()
    );
    // Old in-memory views still own the original file handle across replacement.
    compile(&historical_view.version(second_id).unwrap());
}
#[test]
fn ordinary_saves_preserve_all_versions_and_full_quota_does_not_consume_review_slots() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quota.typsto");
    let mut project = Project::new(fixture(DocumentType::Article, "Initial")).unwrap();
    for n in 0..MAX_SNAPSHOTS {
        project = version(&project, &format!("v{n}"));
    }
    assert!(matches!(
        project.prepare_version(Default::default(), SnapshotReason::Manual),
        Err(Error::Core(CoreError::StorageError(
            StorageError::SnapshotLimit { .. }
        )))
    ));
    let project = project
        .prepare_review_reference(Uuid::new_v4())
        .unwrap()
        .with_current(confirmed(fixture(DocumentType::Article, "Modified")))
        .unwrap();
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let read = Project::read(&path).unwrap();
    assert_eq!(read.versions().len(), MAX_SNAPSHOTS);
    assert_eq!(read.review_references().len(), 1);
    assert_eq!(read.versions()[0].state, read.versions()[99].state);
}
#[test]
fn superseded_packages_compile_from_their_embedded_inventory() {
    let mut archive = fixture(DocumentType::Courrier, "Historical library");
    let mut files = archive.dependencies.as_ref().clone();
    let mut inventory: serde_json::Value =
        serde_json::from_slice(&files["dependencies.json"]).unwrap();
    let package = serde_json::json!({"name":"retired","version":"0.0.1","root":"packages/retired-0.0.1","entrypoint":"main.typ","license":"MIT","archive":"https://example.org/old.tar.gz","sha256":"0".repeat(64),"dependencies":[]});
    inventory["packages"].as_array_mut().unwrap().push(package);
    files.insert(
        "dependencies.json".into(),
        serde_json::to_vec(&inventory).unwrap().into(),
    );
    files.insert(
        "packages/retired-0.0.1/main.typ".into(),
        "#let old = [Ancienne bibliothèque]".as_bytes().into(),
    );
    files.insert(
        "packages/retired-0.0.1/LICENSE".into(),
        b"MIT".as_slice().into(),
    );
    archive
        .template
        .push_str("\n#import \"packages/retired-0.0.1/main.typ\": old\n");
    archive.dependencies = Arc::new(files);
    assert!(archive.validate().is_err());
    let project = Project::new(archive).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pinned.typsto");
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let read = Project::read(&path).unwrap();
    assert_eq!(read.current().dependencies, project.current().dependencies);
    compile(read.current());
}
