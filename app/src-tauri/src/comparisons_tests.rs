use super::*;
use typstographe_core::{
    assets::Assets,
    content::{
        commands::{Command, TextTarget, TextZone},
        contract::{Settings, State},
        editing::Direction,
        project::{SnapshotDetails, VersionAction, VersionChange},
        shared::SharedDocument,
    },
};
fn fixture() -> (tempfile::TempDir, Session, AgentState) {
    let root = tempfile::tempdir().unwrap();
    let mut s = Session::managed(root.path().join("recovery")).unwrap();
    let state=State::new(serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Old one"}]},{"type":"paragraph","content":[{"type":"text","text":"Anchor"}]},{"type":"paragraph","content":[{"type":"text","text":"Old two"}]},{"type":"paragraph"}]})).unwrap(),Settings::default());
    let update = SharedDocument::from_state(&state).unwrap().update();
    s.replace(state.document, state.settings, Assets::default(), update);
    s.join("human");
    s.join("other");
    (root, s, AgentState::default())
}
fn edit(s: &mut Session, index: usize, text: &str) {
    let guard = s.guard();
    let pub_ = s
        .ensure_owner()
        .unwrap()
        .commands(
            "human",
            &guard,
            &[Command::Insert {
                target: TextTarget {
                    zone: TextZone::Body { path: vec![index] },
                    from: 0,
                    to: 0,
                },
                text: text.into(),
                marks: vec![],
            }],
        )
        .unwrap();
    s.record(&pub_);
}
fn version(s: &mut Session, path: &std::path::Path) -> uuid::Uuid {
    let guard = s.guard();
    let index = crate::versions::list(s, "human", &guard).unwrap();
    let destination = s.file_lease.is_none().then_some(path);
    let changed = crate::versions::change(
        s,
        "human",
        &guard,
        VersionChange {
            history: index["history"].as_str().unwrap().into(),
            action: VersionAction::Create {
                details: SnapshotDetails::default(),
            },
        },
        destination,
    )
    .unwrap();
    serde_json::from_value(changed["versions"].as_array().unwrap().last().unwrap()["id"].clone())
        .unwrap()
}
fn review(s: &mut Session, a: &mut AgentState, id: uuid::Uuid) -> String {
    let guard = s.guard();
    open(s, a, "human", &guard, Origin::Version { id, against: None }).unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
#[test]
fn choices_save_once_without_snapshots_and_have_one_undo_step() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("project.typsto");
    let id = version(&mut s, &path);
    let disk = std::fs::read(&path).unwrap();
    let versions = s.project.get().unwrap().owner.versions().to_vec();
    let project_id = s.project.get().unwrap().owner.project_id();
    edit(&mut s, 0, "New ");
    edit(&mut s, 2, "New ");
    let initial = s.ensure_owner().unwrap().state().clone();
    let r = review(&mut s, &mut a, id);
    assert!(read(&s, &a, "other", &r).is_err());
    let ready = prepare(
        &mut s,
        &a,
        "human",
        &r,
        &[SourceSide::Before, SourceSide::After],
    )
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_eq!(s.ensure_owner().unwrap().state(), &initial);
    let (result, publication) = apply(&mut s, &mut a, "human", &r, ready, None).unwrap();
    assert!(publication.changed);
    assert_eq!(result["report"]["closed"], true);
    assert_eq!(result["saved"]["revision"], s.revision);
    let saved = typstographe_core::content::project::Project::read(&path).unwrap();
    assert_eq!(saved.current().state(), *s.ensure_owner().unwrap().state());
    assert_eq!(
        saved.current().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Old one")
    );
    assert_eq!(
        saved.current().document.content[2],
        initial.document.content[2]
    );
    assert_eq!(saved.versions(), versions);
    assert_eq!(saved.project_id(), project_id);
    assert_eq!(s.clean_revision, Some(s.revision));
    let disk = std::fs::read(&path).unwrap();
    s.history("human", Direction::Undo).unwrap();
    assert_eq!(s.ensure_owner().unwrap().state(), &initial);
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_ne!(s.clean_revision, Some(s.revision));
    s.history("human", Direction::Redo).unwrap();
    assert_eq!(s.ensure_owner().unwrap().state(), &saved.current().state());
}
#[test]
fn stale_preparations_ime_invalid_choices_and_external_file_changes_never_publish() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("project.typsto");
    let id = version(&mut s, &path);
    edit(&mut s, 0, "New ");
    edit(&mut s, 2, "New ");
    let r = review(&mut s, &mut a, id);
    assert!(prepare(&mut s, &a, "human", &r, &[]).is_err());
    a.busy.insert("other".into());
    assert!(prepare(&mut s, &a, "human", &r, &[SourceSide::Before; 2]).is_err());
    a.busy.clear();
    let ready = capture(&mut s, &a, "human", &r, &[SourceSide::Before; 2]).unwrap();
    edit(&mut s, 2, "Human ");
    let work = s.updates.clone();
    let disk = std::fs::read(&path).unwrap();
    assert!(apply(&mut s, &mut a, "human", &r, ready.prepare().unwrap(), None).is_err());
    assert_eq!(s.updates, work);
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    let r = review(&mut s, &mut a, id);
    let ready = prepare(&mut s, &a, "human", &r, &[SourceSide::Before; 2]).unwrap();
    std::fs::write(&path, b"External change").unwrap();
    assert!(apply(&mut s, &mut a, "human", &r, ready, None).is_err());
    assert_eq!(s.updates, work);
    assert_eq!(std::fs::read(&path).unwrap(), b"External change");
    assert_eq!(
        read(&s, &a, "human", &r).unwrap()["report"]["closed"],
        false
    );
}
#[test]
fn failed_destination_preserves_the_choices_work_and_history_for_retry() {
    let (root, mut s, mut a) = fixture();
    let id = version(&mut s, &root.path().join("original.typsto"));
    s.file_lease = None;
    edit(&mut s, 0, "New ");
    let r = review(&mut s, &mut a, id);
    let choices = [SourceSide::Before];
    let work = s.updates.clone();
    let invalid = root.path().join("missing").join("file.typsto");
    let ready = prepare(&mut s, &a, "human", &r, &choices).unwrap();
    assert!(apply(&mut s, &mut a, "human", &r, ready, Some(&invalid)).is_err());
    assert_eq!(s.updates, work);
    assert!(s.file_lease.is_none());
    let ready = prepare(&mut s, &a, "human", &r, &choices).unwrap();
    let path = root.path().join("retry.typsto");
    apply(&mut s, &mut a, "human", &r, ready, Some(&path)).unwrap();
    let saved = typstographe_core::content::project::Project::read(&path).unwrap();
    assert_eq!(saved.versions().len(), 1);
    assert_eq!(
        saved.current().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Old one")
    );
}
#[test]
fn historical_pairs_compose_from_the_right_and_replace_the_captured_live_state() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("history.typsto");
    let first = version(&mut s, &path);
    edit(&mut s, 0, "Second ");
    edit(&mut s, 2, "Second ");
    let second = version(&mut s, &path);
    edit(&mut s, 0, "Current ");
    let guard = s.guard();
    let value = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version {
            id: second,
            against: Some(first),
        },
    )
    .unwrap();
    let r = value["id"].as_str().unwrap();
    let ready = prepare(
        &mut s,
        &a,
        "human",
        r,
        &[SourceSide::Before, SourceSide::After],
    )
    .unwrap();
    apply(&mut s, &mut a, "human", r, ready, None).unwrap();
    let saved = typstographe_core::content::project::Project::read(&path).unwrap();
    assert_eq!(saved.versions().len(), 2);
    assert_eq!(
        saved.current().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Old one")
    );
    assert_eq!(
        saved.current().document.content[2].content[0]
            .text
            .as_deref(),
        Some("Second Old two")
    );
}
#[test]
fn identical_historical_states_can_replace_a_different_current_document() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("identical.typsto");
    let first = version(&mut s, &path);
    let second = version(&mut s, &path);
    edit(&mut s, 0, "Current ");
    let guard = s.guard();
    let value = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version {
            id: first,
            against: Some(second),
        },
    )
    .unwrap();
    let r = value["id"].as_str().unwrap();
    assert!(value["report"]["groups"].as_array().unwrap().is_empty());
    let ready = prepare(&mut s, &a, "human", r, &[]).unwrap();
    apply(&mut s, &mut a, "human", r, ready, None).unwrap();
    assert_eq!(
        s.ensure_owner().unwrap().state().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Old one")
    );
}
#[test]
fn mcp_choices_stay_private_until_save_then_produce_final_receipts_without_a_snapshot() {
    let (root, mut s, mut a) = fixture();
    a.token = Some("authorized".into());
    let request = json!({"document":{"id":s.generation,"guard":s.guard()},"request_id":"composed","steps":[{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Proposal "}},{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[2]},"from":0,"to":0},"text":"Proposal "}}]});
    let result = crate::agent::dispatch(&mut s, &mut a, "proposals.submit", &request).unwrap();
    let id = result["outcome"]["id"].as_str().unwrap().to_owned();
    let guard = s.guard();
    let value = open(&mut s, &mut a, "human", &guard, Origin::Proposal { id }).unwrap();
    let r = value["id"].as_str().unwrap();
    let ready = prepare(
        &mut s,
        &a,
        "human",
        r,
        &[SourceSide::After, SourceSide::Before],
    )
    .unwrap();
    assert_eq!(a.proposal.as_ref().unwrap().status, "pending");
    assert_eq!(s.guard(), guard);
    let path = root.path().join("proposal.typsto");
    let duplicate = prepare(&mut s, &a, "human", r, &[SourceSide::After; 2]).unwrap();
    apply(&mut s, &mut a, "human", r, ready, Some(&path)).unwrap();
    assert_eq!(a.proposal.as_ref().unwrap().status, "partially_accepted");
    assert_eq!(
        a.proposal.as_ref().unwrap().decisions[1]["decision"],
        "refused"
    );
    let saved = typstographe_core::content::project::Project::read(&path).unwrap();
    assert!(saved.versions().is_empty());
    assert_eq!(
        saved.current().document.content[2].content[0]
            .text
            .as_deref(),
        Some("Old two")
    );
    let disk = std::fs::read(&path).unwrap();
    assert!(apply(&mut s, &mut a, "human", r, duplicate, None).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), disk);
}
#[test]
fn recovery_cleanup_failure_keeps_the_successful_save_and_publication() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("committed.typsto");
    let id = version(&mut s, &path);
    edit(&mut s, 0, "New ");
    let r = review(&mut s, &mut a, id);
    let recovery = s.root.join("session.json");
    std::fs::create_dir_all(&recovery).unwrap();
    let ready = prepare(&mut s, &a, "human", &r, &[SourceSide::Before]).unwrap();
    let (result, _) = apply(&mut s, &mut a, "human", &r, ready, None).unwrap();
    assert!(result["saved"]["warning"].is_string());
    assert_eq!(result["report"]["applied"], true);
    assert_eq!(s.clean_revision, Some(s.revision));
    let saved = typstographe_core::content::project::Project::read(&path).unwrap();
    assert_eq!(saved.current().state(), *s.ensure_owner().unwrap().state());
    assert_eq!(saved.versions().len(), 1);
    s.file_lease.as_ref().unwrap().check().unwrap();
}
#[test]
fn a_rejected_private_draft_blocks_before_the_durable_commit() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("pending.typsto");
    let id = version(&mut s, &path);
    edit(&mut s, 0, "New ");
    let r = review(&mut s, &mut a, id);
    let ready = prepare(&mut s, &a, "human", &r, &[SourceSide::Before]).unwrap();
    let disk = std::fs::read(&path).unwrap();
    let work = s.updates.clone();
    assert!(
        s.ensure_owner()
            .unwrap()
            .update("human", &[255], Some(99), true, None)
            .is_err()
    );
    assert!(s.ensure_owner().unwrap().draft("human").unwrap().is_some());
    assert!(apply(&mut s, &mut a, "human", &r, ready, None).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_eq!(s.updates, work);
}
