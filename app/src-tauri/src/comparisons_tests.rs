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
fn stage(s: &mut Session, a: &AgentState, parent: &str, choices: &[SourceSide]) -> Value {
    let ready = staging::capture(s, a, "human", parent, choices)
        .unwrap()
        .prepare()
        .unwrap();
    staging::finish(s, a, "human", ready).unwrap()
}
#[test]
fn choices_publish_without_saving_or_snapshots_and_have_one_undo_step() {
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
    let (result, publication) = apply(&mut s, &mut a, "human", &r, ready).unwrap();
    assert!(publication.changed);
    assert_eq!(result["report"]["closed"], true);
    assert!(result.get("saved").is_none());
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    let saved = s.stored_project().unwrap();
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
    assert_ne!(s.clean_revision, Some(s.revision));
    let disk = std::fs::read(&path).unwrap();
    s.history("human", Direction::Undo).unwrap();
    assert_eq!(s.ensure_owner().unwrap().state(), &initial);
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_ne!(s.clean_revision, Some(s.revision));
    s.history("human", Direction::Redo).unwrap();
    assert_eq!(s.ensure_owner().unwrap().state(), &saved.current().state());
}
#[test]
fn stale_preparations_ime_and_invalid_choices_never_publish() {
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
    assert!(apply(&mut s, &mut a, "human", &r, ready.prepare().unwrap()).is_err());
    assert_eq!(s.updates, work);
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_eq!(
        read(&s, &a, "human", &r).unwrap()["report"]["closed"],
        false
    );
}

#[test]
fn composition_without_a_file_binding_never_requires_a_destination() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("original.typsto");
    let id = version(&mut s, &path);
    let disk = std::fs::read(&path).unwrap();
    s.file_lease = None;
    edit(&mut s, 0, "New ");
    let r = review(&mut s, &mut a, id);
    let ready = prepare(&mut s, &a, "human", &r, &[SourceSide::Before]).unwrap();
    apply(&mut s, &mut a, "human", &r, ready).unwrap();
    assert!(s.file_lease.is_none());
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    let current = s.stored_project().unwrap();
    assert_eq!(current.versions().len(), 1);
    assert_eq!(
        current.current().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Old one")
    );
    assert_ne!(s.clean_revision, Some(s.revision));
}

#[test]
fn historical_composition_is_private_until_reviewed_against_the_current_document() {
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
    assert_eq!(value["report"]["context"], "historical");
    let work = s.guard();
    let disk = std::fs::read(&path).unwrap();
    assert!(apply(&mut s, &mut a, "human", r, ready).is_err());
    let next = stage(&mut s, &a, r, &[SourceSide::Before, SourceSide::After]);
    assert_eq!(s.guard(), work);
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_eq!(next["report"]["context"], "composition");
    assert_eq!(
        next["before"]["state"]["document"]["content"][0]["content"][0]["text"],
        "Current Second Old one"
    );
    assert_eq!(
        next["after"]["state"]["document"]["content"][0]["content"][0]["text"],
        "Old one"
    );
    let child = next["id"].as_str().unwrap();
    let choices = vec![SourceSide::After; next["report"]["groups"].as_array().unwrap().len()];
    let ready = prepare(&mut s, &a, "human", child, &choices).unwrap();
    apply(&mut s, &mut a, "human", child, ready).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    let saved = s.stored_project().unwrap();
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
    let current = s.guard();
    let next = stage(&mut s, &a, r, &[]);
    assert_eq!(s.guard(), current);
    assert_eq!(next["report"]["groups"].as_array().unwrap().len(), 1);
    let child = next["id"].as_str().unwrap();
    let ready = prepare(&mut s, &a, "human", child, &[SourceSide::After]).unwrap();
    apply(&mut s, &mut a, "human", child, ready).unwrap();
    assert_eq!(
        s.ensure_owner().unwrap().state().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Old one")
    );
}
#[test]
fn mcp_choices_stay_private_until_applied_then_produce_receipts_without_saving_or_snapshot() {
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
    apply(&mut s, &mut a, "human", r, ready).unwrap();
    assert_eq!(a.proposal.as_ref().unwrap().status, "partially_accepted");
    assert_eq!(
        a.proposal.as_ref().unwrap().decisions[1]["decision"],
        "refused"
    );
    assert!(!path.exists());
    assert!(s.file_lease.is_none());
    let saved = s.stored_project().unwrap();
    assert!(saved.versions().is_empty());
    assert_eq!(
        saved.current().document.content[2].content[0]
            .text
            .as_deref(),
        Some("Old two")
    );
    let revision = s.revision;
    assert!(apply(&mut s, &mut a, "human", r, duplicate).is_err());
    assert_eq!(s.revision, revision);
    assert!(!path.exists());
}
#[test]
fn composition_preserves_recovery_and_survives_a_new_checkpoint() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("project.typsto");
    let id = version(&mut s, &path);
    let disk = std::fs::read(&path).unwrap();
    edit(&mut s, 0, "New ");
    edit(&mut s, 2, "New ");
    let snapshot = crate::session::Snapshot {
        revision: s.revision,
        document: s.ensure_owner().unwrap().state().document.clone(),
        settings: Some(s.settings.clone()),
    };
    let update = s.ensure_owner().unwrap().update_bytes();
    let recovery = s.save(&snapshot, update).unwrap();
    let checkpoint = std::fs::read(&recovery).unwrap();
    let r = review(&mut s, &mut a, id);
    let ready = prepare(
        &mut s,
        &a,
        "human",
        &r,
        &[SourceSide::Before, SourceSide::After],
    )
    .unwrap();
    apply(&mut s, &mut a, "human", &r, ready).unwrap();
    assert_eq!(std::fs::read(&recovery).unwrap(), checkpoint);
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_ne!(s.clean_revision, Some(s.revision));
    s.file_lease.as_ref().unwrap().check().unwrap();
    let update = s.ensure_owner().unwrap().update_bytes();
    let snapshot = crate::session::Snapshot {
        revision: s.revision,
        document: s.ensure_owner().unwrap().state().document.clone(),
        settings: Some(s.settings.clone()),
    };
    let checkpoint = s.checkpoint(&snapshot, update.clone()).unwrap();
    s.save(&snapshot, update).unwrap();
    let stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&recovery).unwrap()).unwrap();
    assert_eq!(
        stored["checkpoint"]["document"],
        serde_json::to_value(&snapshot.document).unwrap()
    );
    let mut restored = Session::from_checkpoint(root.path().join("restored"), checkpoint).unwrap();
    assert_eq!(
        restored.ensure_owner().unwrap().state(),
        s.ensure_owner().unwrap().state()
    );
    assert_eq!(restored.stored_project().unwrap().versions().len(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), disk);
}

#[test]
fn a_rejected_private_draft_blocks_before_publication() {
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
    let failure = apply(&mut s, &mut a, "human", &r, ready).unwrap_err();
    assert!(!failure.starts_with('{') && !failure.contains("pending_draft"));
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_eq!(s.updates, work);
}

#[test]
fn an_external_file_change_does_not_block_an_edit_or_overwrite_the_file() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("external.typsto");
    let id = version(&mut s, &path);
    edit(&mut s, 0, "New ");
    let r = review(&mut s, &mut a, id);
    let ready = prepare(&mut s, &a, "human", &r, &[SourceSide::Before]).unwrap();
    std::fs::write(&path, b"External change").unwrap();
    apply(&mut s, &mut a, "human", &r, ready).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"External change");
    assert!(s.file_lease.as_ref().unwrap().check().is_err());
    assert_ne!(s.clean_revision, Some(s.revision));
    let current = s.stored_project().unwrap();
    let mut lease = s.file_lease.as_ref().unwrap().clone();
    assert!(current.save(&mut lease).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"External change");
}
#[test]
fn keeping_current_content_does_not_dirty_a_clean_document_or_add_undo() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("clean.typsto");
    let id = version(&mut s, &path);
    edit(&mut s, 0, "New ");
    version(&mut s, &path);
    let disk = std::fs::read(&path).unwrap();
    let guard = s.guard();
    let undo = s.ensure_owner().unwrap().snapshot("human").unwrap();
    let r = review(&mut s, &mut a, id);
    let ready = prepare(&mut s, &a, "human", &r, &[SourceSide::After]).unwrap();
    let (_, publication) = apply(&mut s, &mut a, "human", &r, ready).unwrap();
    assert!(!publication.changed);
    assert_eq!(s.guard(), guard);
    assert_eq!(s.clean_revision, Some(s.revision));
    assert_eq!(
        s.ensure_owner()
            .unwrap()
            .snapshot("human")
            .unwrap()
            .undo_steps,
        undo.undo_steps
    );
    assert_eq!(std::fs::read(&path).unwrap(), disk);
}

#[test]
fn staged_compositions_reject_stale_guards_other_windows_and_obsolete_candidates() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("staged.typsto");
    let first = version(&mut s, &path);
    edit(&mut s, 0, "Second ");
    let second = version(&mut s, &path);
    edit(&mut s, 2, "Current ");
    let guard = s.guard();
    let parent = open(
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
    let id = parent["id"].as_str().unwrap();
    assert!(staging::capture(&mut s, &a, "other", id, &[SourceSide::After]).is_err());
    let child = stage(&mut s, &a, id, &[SourceSide::After]);
    let child_id = child["id"].as_str().unwrap().to_string();
    let choices = vec![SourceSide::After; child["report"]["groups"].as_array().unwrap().len()];
    let obsolete = prepare(&mut s, &a, "human", &child_id, &choices).unwrap();
    let next = stage(&mut s, &a, id, &[SourceSide::Before]);
    assert!(read(&s, &a, "human", &child_id).is_err());
    assert!(apply(&mut s, &mut a, "human", &child_id, obsolete).is_err());
    let pending = staging::capture(&mut s, &a, "human", id, &[SourceSide::After])
        .unwrap()
        .prepare()
        .unwrap();
    edit(&mut s, 2, "Later ");
    let work = s.guard();
    assert!(staging::finish(&mut s, &a, "human", pending).is_err());
    let child_id = next["id"].as_str().unwrap();
    assert_eq!(
        read(&s, &a, "human", child_id).unwrap()["report"]["stale"],
        true
    );
    assert!(prepare(&mut s, &a, "human", child_id, &[SourceSide::After]).is_err());
    assert_eq!(s.guard(), work);
    close(&mut s, "human", id).unwrap();
    assert!(read(&s, &a, "human", child_id).is_err());
    assert!(staging::capture(&mut s, &a, "human", id, &[SourceSide::After]).is_err());
}

#[test]
fn refusing_every_mcp_change_closes_the_proposal_without_editing_or_saving() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("unchanged.typsto");
    version(&mut s, &path);
    let disk = std::fs::read(&path).unwrap();
    a.token = Some("authorized".into());
    let request = json!({"document":{"id":s.generation,"guard":s.guard()},"request_id":"refuse-all","steps":[{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Proposal "}}]});
    let result = crate::agent::dispatch(&mut s, &mut a, "proposals.submit", &request).unwrap();
    let id = result["outcome"]["id"].as_str().unwrap().to_owned();
    let guard = s.guard();
    let state = s.stored_project().unwrap();
    let context = s.ensure_owner().unwrap().snapshot("human").unwrap();
    let value = open(&mut s, &mut a, "human", &guard, Origin::Proposal { id }).unwrap();
    let r = value["id"].as_str().unwrap();
    let ready = prepare(&mut s, &a, "human", r, &[SourceSide::Before]).unwrap();
    let (value, publication) = apply(&mut s, &mut a, "human", r, ready).unwrap();
    assert_eq!(a.proposal.as_ref().unwrap().status, "rejected");
    assert_eq!(value["report"]["closed"], true);
    assert!(publication.update.is_empty());
    assert_eq!(s.guard(), guard);
    assert_eq!(
        s.stored_project().unwrap().current().state(),
        state.current().state()
    );
    assert_eq!(s.stored_project().unwrap().versions(), state.versions());
    assert_eq!(
        s.ensure_owner().unwrap().snapshot("human").unwrap(),
        context
    );
    assert_eq!(std::fs::read(path).unwrap(), disk);
}
