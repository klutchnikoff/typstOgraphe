use super::*;
use crate::session::Session;
use typstographe_core::{
    assets::Assets,
    content::{
        contract::{Settings, State},
        project::{SnapshotDetails, VersionAction},
        shared::SharedDocument,
    },
};
fn fixture(root: &Path) -> Session {
    let state = State::new(serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Confirmed"}]}]})).unwrap(), Settings::default());
    let mut session = Session::managed(root.into()).unwrap();
    let update = SharedDocument::from_state(&state).unwrap().update();
    session.replace(
        state.document,
        state.settings.clone(),
        Assets::default(),
        update,
    );
    session.join("main");
    session
}
fn capture(history: &Value, name: &str) -> VersionChange {
    VersionChange {
        history: history["history"].as_str().unwrap().into(),
        action: VersionAction::Create {
            details: SnapshotDetails {
                name: name.into(),
                ..Default::default()
            },
        },
    }
}
#[test]
fn native_version_decisions_preserve_document_revision_private_fields_and_history() {
    let root = tempfile::tempdir().unwrap();
    let mut session = fixture(&root.path().join("recovery"));
    let guard = session.guard();
    let history = list(&mut session, "main", &guard).unwrap();
    let state = session.ensure_owner().unwrap().state().clone();
    let before = session.ensure_owner().unwrap().snapshot("main").unwrap();
    let path = root.path().join("versions.typsto");
    let first = change(
        &mut session,
        "main",
        &guard,
        capture(&history, "v1"),
        Some(&path),
    )
    .unwrap();
    let id = serde_json::from_value(first["versions"][0]["id"].clone()).unwrap();
    assert_eq!(session.guard(), guard);
    assert_eq!(session.ensure_owner().unwrap().state(), &state);
    assert_eq!(
        serde_json::to_value(session.ensure_owner().unwrap().snapshot("main").unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(
        read(&mut session, "main", &guard, id).unwrap()["state"],
        json!(state)
    );
    let epoch = session
        .open_private_recovery("main", &guard.document)
        .unwrap();
    let update = session.ensure_owner().unwrap().update_bytes();
    session
        .conserve_private(
            "main",
            crate::recovery::drafts::Input {
                generation: guard.document.clone(),
                epoch,
                sequence: 1,
                update,
                document: json!(state.document),
                settings: json!(state.settings),
                fields: vec![json!({"private":"Uncommitted"})],
                forms: vec![],
                resources: vec![],
                rejected: false,
                recovered: vec![],
                retain: false,
            },
        )
        .unwrap();
    let private = crate::recovery::drafts::path(&session.root, &guard.document, "main").unwrap();
    let bytes = std::fs::read(&private).unwrap();
    let second = change(&mut session, "main", &guard, capture(&first, "v2"), None).unwrap();
    assert_eq!(second["versions"].as_array().unwrap().len(), 2);
    assert_eq!(std::fs::read(&private).unwrap(), bytes);
    assert!(change(&mut session, "main", &guard, capture(&first, "stale"), None).is_err());
    let mut stale = guard.clone();
    stale.revision += 1;
    assert!(list(&mut session, "main", &stale).is_err());
    assert!(list(&mut session, "foreign", &guard).is_err());
    std::fs::write(&path, b"external edit").unwrap();
    assert!(
        change(
            &mut session,
            "main",
            &guard,
            capture(&second, "refused"),
            None
        )
        .is_err()
    );
    assert_eq!(
        list(&mut session, "main", &guard).unwrap()["history"],
        second["history"]
    );
    assert_eq!(std::fs::read(path).unwrap(), b"external edit");
}
#[test]
fn an_archive_commit_keeps_its_version_even_when_recovery_cleanup_fails() {
    let root = tempfile::tempdir().unwrap();
    let mut session = fixture(&root.path().join("recovery"));
    let guard = session.guard();
    let index = list(&mut session, "main", &guard).unwrap();
    let recovery = session.root.join("session.json");
    std::fs::create_dir_all(&recovery).unwrap();
    let path = root.path().join("committed.typsto");
    let response = change(
        &mut session,
        "main",
        &guard,
        capture(&index, "Saved despite cleanup"),
        Some(&path),
    )
    .unwrap();
    assert!(response["warning"].is_string());
    let saved = typstographe_core::content::project::Project::read(&path).unwrap();
    assert_eq!(saved.versions().len(), 1);
    assert_eq!(
        session.project.get().unwrap().owner.versions(),
        saved.versions()
    );
    session.file_lease.as_ref().unwrap().check().unwrap();
}
