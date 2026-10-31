use super::tests::{entries, fixture, version};
use super::*;
use crate::{
    bibliography::ReferenceFile,
    document_types::{DocumentType, Metadata},
};
use std::{collections::BTreeMap, fs, io::Write};

fn insert(files: &mut BTreeMap<String, Vec<u8>>, bytes: Vec<u8>) -> ObjectId {
    let id = ObjectId::of(&bytes);
    files.insert(id.path(), bytes);
    id
}
fn rewrite(
    path: &Path,
    project: &Project,
    mut files: BTreeMap<String, Vec<u8>>,
    change: impl FnOnce(&mut schema::StateDescriptor, &mut BTreeMap<String, Vec<u8>>),
) {
    let mut descriptor = project
        .store
        .read_json::<schema::StateDescriptor>(&project.current_id)
        .unwrap();
    change(&mut descriptor, &mut files);
    let id = insert(&mut files, serde_json::to_vec(&descriptor).unwrap());
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["current"] = serde_json::to_value(id).unwrap();
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (path, bytes) in files {
        zip.start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap();
}
#[test]
fn valid_hashes_do_not_hide_mismatched_projections_sources_or_bibliography() {
    let mut archive = fixture(DocumentType::Article, "Selon cette étude ");
    archive.metadata = Metadata::demo(DocumentType::Article);
    let Metadata::Article(metadata) = &mut archive.metadata else {
        unreachable!()
    };
    metadata.references.push(ReferenceFile {
        id: "research".into(),
        name: "references.bib".into(),
        contents: "@book{example, author={Martin, Camille}, title={Une étude}, year={2020}}".into(),
    });
    archive.document.content[0].content.push(serde_json::from_value(serde_json::json!({"type":"citation","attrs":{"items":[{"file_id":"research","key":"example","supplement":"p. 2"}],"form":"normal"}})).unwrap());
    let project = Project::new(archive).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coherence.typsto");
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let original = entries(&path);
    let read = Project::read(&path).unwrap();
    assert_eq!(read.current().metadata, project.current().metadata);
    let mut world = crate::rendering::TypstWorld::default();
    world.set_dependencies(&read.current().dependencies);
    world
        .compile_with_template(
            &read.current().validate().unwrap(),
            &read.current().metadata,
            &read.current().template,
        )
        .unwrap();
    for kind in [
        "projection",
        "metadata",
        "yjs",
        "body",
        "main",
        "type",
        "contract",
        "encoding",
        "schema",
        "old-generator",
        "future-generator",
        "bibliography",
        "bibliography-changed",
        "bibliography-unclaimed",
        "duplicate-resource",
        "path",
    ] {
        rewrite(&path, &project, original.clone(), |d, files| match kind {
            "projection" => {
                d.document = insert(
                    files,
                    serde_json::to_vec(&fixture(DocumentType::Article, "Different text").document)
                        .unwrap(),
                )
            }
            "metadata" => {
                d.metadata = insert(
                    files,
                    serde_json::to_vec(&Metadata::blank(DocumentType::Article)).unwrap(),
                )
            }
            "yjs" => d.yjs.object = insert(files, vec![]),
            "body" => d.sources.body = insert(files, b"Different source".to_vec()),
            "main" => d.sources.main = insert(files, b"Different main".to_vec()),
            "type" => d.document_type = DocumentType::Courrier,
            "contract" => d.contract_version += 1,
            "encoding" => d.yjs.encoding = "yjs-v2".into(),
            "schema" => d.version += 1,
            "old-generator" => d.producer.generator = 1,
            "future-generator" => d.producer.generator = schema::SOURCE_GENERATOR_VERSION + 1,
            "bibliography" => {
                let mut resources: schema::ResourceMap =
                    project.store.read_json(&d.resources).unwrap();
                resources.retain(|p, _| !p.starts_with("bibliography/"));
                d.resources = insert(files, serde_json::to_vec(&resources).unwrap());
            }
            "bibliography-changed" | "bibliography-unclaimed" => {
                let mut resources: schema::ResourceMap =
                    project.store.read_json(&d.resources).unwrap();
                let path = if kind == "bibliography-changed" {
                    resources
                        .keys()
                        .find(|p| p.starts_with("bibliography/"))
                        .unwrap()
                        .clone()
                } else {
                    "bibliography/unclaimed.bib".into()
                };
                let id = insert(files, b"@book{unclaimed, title={Unknown}}".to_vec());
                resources.insert(path, id);
                d.resources = insert(files, serde_json::to_vec(&resources).unwrap());
            }
            "duplicate-resource" => {
                let resources: schema::ResourceMap = project.store.read_json(&d.resources).unwrap();
                let (path, id) = resources.first_key_value().unwrap();
                let pair = format!(
                    "{}:{}",
                    serde_json::to_string(path).unwrap(),
                    serde_json::to_string(id).unwrap()
                );
                d.resources = insert(files, format!("{{{pair},{pair}}}").into_bytes());
            }
            "path" => {
                let mut resources: schema::ResourceMap =
                    project.store.read_json(&d.resources).unwrap();
                resources.insert("images/../escape.svg".into(), d.sources.body.clone());
                d.resources = insert(files, serde_json::to_vec(&resources).unwrap());
            }
            _ => unreachable!(),
        });
        assert!(Project::read(&path).is_err(), "{kind}");
    }
}
#[test]
fn failed_candidates_and_label_edits_do_not_rewrite_immutable_states() {
    let project = version(
        &Project::new(fixture(DocumentType::Courrier, "Confirmed")).unwrap(),
        "v1",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("candidate.typsto");
    let mut lease = DocumentLease::acquire(&path).unwrap();
    project.save(&mut lease).unwrap();
    let original = fs::read(&path).unwrap();
    let mut invalid = project.current().clone();
    invalid.document = fixture(DocumentType::Courrier, "Changed JSON, unchanged Yjs").document;
    assert!(project.with_current(invalid).is_err());
    assert!(
        project
            .with_current(fixture(DocumentType::Courrier, "Missing checkpoint"))
            .is_err()
    );
    assert_eq!(original, fs::read(&path).unwrap());
    lease.check().unwrap();
    let record = &project.versions()[0];
    let renamed = project
        .prepare_version_details(
            record.id,
            SnapshotDetails {
                name: "Submission".into(),
                tags: vec!["ready".into()],
                description: Some("Reviewed draft".into()),
                creator: Some("Camille".into()),
            },
        )
        .unwrap();
    assert_eq!(renamed.versions()[0].state, record.state);
    assert_eq!(renamed.versions()[0].id, record.id);
    assert_eq!(renamed.versions()[0].created_at, record.created_at);
    assert_eq!(renamed.versions()[0].reason, record.reason);
    assert!(
        project
            .prepare_version_details(
                record.id,
                SnapshotDetails {
                    name: " ".into(),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert!(
        project
            .prepare_version_details(Uuid::new_v4(), Default::default())
            .is_err()
    );
    assert!(project.prepare_review_reference(Uuid::nil()).is_err());
    renamed.save(&mut lease).unwrap();
    let read = Project::read(&path).unwrap();
    assert_eq!(read.versions(), renamed.versions());
}
