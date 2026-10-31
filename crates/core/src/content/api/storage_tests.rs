use super::tests::{call, new, target};
use super::*;
use crate::{
    DocumentLease,
    content::project::{Project, SnapshotDetails, SnapshotReason},
};

#[test]
fn cli_mcp_sessions_keep_history_on_save_and_isolate_copies_and_failed_batches() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("original.typsto");
    let seed = new("article");
    let storage = seed
        .stored_project()
        .unwrap()
        .prepare_version(
            SnapshotDetails {
                name: "v1".into(),
                ..Default::default()
            },
            SnapshotReason::Manual,
        )
        .unwrap();
    storage
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let mut session = Session::new();
    session
        .execute("document.open", json!({"path":path}))
        .unwrap();
    session
        .execute(
            "text.insert",
            json!({"target":target(&[0],0,0),"text":"Edited"}),
        )
        .unwrap();
    session.save().unwrap();
    let saved = Project::read(&path).unwrap();
    assert_eq!(saved.project_id(), storage.project_id());
    assert_eq!(saved.versions(), storage.versions());
    let before = std::fs::read(&path).unwrap();
    assert!(
        session
            .batch(
                &[
                    call(
                        "text.insert",
                        json!({"target":target(&[0],0,0),"text":"Rejected"})
                    ),
                    call("text.delete", json!({"target":target(&[99],0,1)}))
                ],
                Some(session::Save::Current)
            )
            .is_err()
    );
    assert_eq!(before, std::fs::read(&path).unwrap());
    let copy_path = dir.path().join("copy.typsto");
    let mut fork = session.fork().unwrap();
    fork.save_as(&copy_path).unwrap();
    let copy = Project::read(&copy_path).unwrap();
    assert_ne!(copy.project_id(), storage.project_id());
    assert_eq!(copy.versions(), storage.versions());
    assert_eq!(
        session.stored_project().unwrap().project_id(),
        storage.project_id()
    );
}

#[test]
fn pinned_dependencies_remain_owned_by_the_common_session_after_catalogue_changes() {
    let seed = new("courrier");
    let mut archive = seed.archive().unwrap();
    archive.metadata =
        crate::document_types::Metadata::demo(crate::document_types::DocumentType::Courrier);
    archive.template = archive
        .template
        .replace("capabilities/dates.typ", "capabilities/pinned-dates.typ");
    let mut dependencies = (*archive.dependencies).clone();
    let dates = dependencies.remove("capabilities/dates.typ").unwrap();
    dependencies.insert("capabilities/pinned-dates.typ".into(), dates);
    let mut inventory: Value = serde_json::from_slice(&dependencies["dependencies.json"]).unwrap();
    for capability in inventory["capabilities"].as_array_mut().unwrap() {
        if capability["id"] == "dates" {
            capability["entrypoint"] = "capabilities/pinned-dates.typ".into();
        }
    }
    dependencies.insert(
        "dependencies.json".into(),
        serde_json::to_vec(&inventory).unwrap().into(),
    );
    archive.dependencies = std::sync::Arc::new(dependencies);
    archive.yjs.clear();
    let stored = Project::new(archive).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pinned.typsto");
    stored
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let mut session = Session::new();
    session
        .execute("document.open", json!({"path":path}))
        .unwrap();
    session
        .execute(
            "text.insert",
            json!({"target":target(&[0],0,0),"text":"Local edit"}),
        )
        .unwrap();
    session.save().unwrap();
    assert_eq!(
        session.archive().unwrap().dependencies,
        stored.current().dependencies
    );
    assert!(session.page_png(1).is_ok());
    let output = dir.path().join("export");
    session
        .export(&output, crate::protocol::ExportFormat::Typst, false)
        .unwrap();
    assert_eq!(
        std::fs::read(output.join("capabilities/pinned-dates.typ")).unwrap(),
        stored.current().dependencies["capabilities/pinned-dates.typ"].as_ref()
    );
}
