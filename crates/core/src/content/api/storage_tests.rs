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
fn cli_mcp_sessions_use_application_libraries_for_reopened_projects_and_exports() {
    let seed = new("courrier");
    let mut archive = seed.archive().unwrap();
    archive.metadata =
        crate::document_types::Metadata::demo(crate::document_types::DocumentType::Courrier);
    let application_libraries = archive.dependencies.clone();
    let mut dependencies = (*archive.dependencies).clone();
    dependencies.insert(
        "capabilities/dates.typ".into(),
        b"#panic(\"Old transport copy must not run\")"
            .as_slice()
            .into(),
    );
    archive.dependencies = std::sync::Arc::new(dependencies);
    archive.yjs.clear();
    let stored = Project::new(archive).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transport.typsto");
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
    assert!(session.page_png(1).is_ok());
    session.save().unwrap();
    assert_eq!(
        session.archive().unwrap().dependencies,
        application_libraries
    );
    assert!(session.page_png(1).is_ok());
    let output = dir.path().join("export");
    session
        .export(&output, crate::protocol::ExportFormat::Typst, false)
        .unwrap();
    assert_eq!(
        std::fs::read(output.join("capabilities/dates.typ")).unwrap(),
        application_libraries["capabilities/dates.typ"].as_ref()
    );
}
