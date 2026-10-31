use super::tests::{confirmed, fixture, image_fixture, version};
use super::*;
use crate::document_types::DocumentType;

#[test]
fn portable_backup_keeps_identity_versions_and_resources_after_the_original_disappears() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("original.typsto");
    let first = version(&Project::new(image_fixture("historical")).unwrap(), "v1");
    let review = Uuid::new_v4();
    let first = first.prepare_review_reference(review).unwrap();
    let project = version(
        &first
            .with_current(confirmed(fixture(DocumentType::Courrier, "Current")))
            .unwrap(),
        "v2",
    );
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let opened = Project::read(&path).unwrap();
    let backup = Backup::capture(&opened).unwrap();
    let json = serde_json::to_vec(&backup).unwrap();
    drop(opened);
    std::fs::remove_file(&path).unwrap();
    let restored = serde_json::from_slice::<Backup>(&json)
        .unwrap()
        .restore()
        .unwrap();
    assert_eq!(restored.project_id(), project.project_id());
    assert_eq!(restored.versions(), project.versions());
    assert_eq!(restored.review_references(), project.review_references());
    assert!(restored.current().assets.is_empty());
    assert!(
        restored
            .review_reference(review)
            .unwrap()
            .assets
            .contains_key("images/historical.svg")
    );
    assert!(
        restored
            .version(restored.versions()[0].id)
            .unwrap()
            .assets
            .contains_key("images/historical.svg")
    );
    assert_eq!(restored.current().state(), project.current().state());
    assert_eq!(
        restored
            .version(restored.versions()[0].id)
            .unwrap()
            .document,
        first.current().document
    );
    assert_eq!(
        restored.current().dependencies,
        project.current().dependencies
    );
    assert!(Project::from_bytes(b"not a zip".to_vec()).is_err());
    assert!(
        serde_json::from_str::<Backup>("\"not-base64\"")
            .unwrap()
            .restore()
            .is_err()
    );
}
