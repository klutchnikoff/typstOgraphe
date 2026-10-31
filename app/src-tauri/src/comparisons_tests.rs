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
#[test]
fn local_version_reprises_preserve_the_file_and_snapshots_and_have_independent_undo() {
    let (root, mut s, mut a) = fixture();
    let path = root.path().join("project.typsto");
    let id = version(&mut s, &path);
    let disk = std::fs::read(&path).unwrap();
    edit(&mut s, 0, "New ");
    edit(&mut s, 2, "New ");
    let current = s.ensure_owner().unwrap().state().clone();
    let guard = s.guard();
    let view = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    let review = view["id"].as_str().unwrap();
    assert_eq!(view["report"]["groups"].as_array().unwrap().len(), 2);
    assert!(read(&s, &a, "other", review).is_err());
    for g in [1, 0] {
        let ready = prepare(&mut s, &a, "human", review, &[g]).unwrap();
        decide(&mut s, &mut a, "human", review, &[g], Some(ready)).unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), disk);
    assert_eq!(
        s.ensure_owner().unwrap().state().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Old one")
    );
    let frozen = s.comparisons[review].plan.after().state();
    assert_eq!(frozen, current);
    s.history("human", Direction::Undo).unwrap();
    assert!(
        read(&s, &a, "human", review).unwrap()["report"]["stale"]
            .as_bool()
            .unwrap()
    );
    assert!(prepare(&mut s, &a, "human", review, &[0]).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), disk);
}
#[test]
fn external_edits_revocation_ime_and_late_refusal_never_publish_a_prepared_candidate() {
    let (root, mut s, mut a) = fixture();
    let id = version(&mut s, &root.path().join("p.typsto"));
    edit(&mut s, 0, "New ");
    let guard = s.guard();
    let view = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    let review = view["id"].as_str().unwrap().to_owned();
    a.busy.insert("other".into());
    assert!(prepare(&mut s, &a, "human", &review, &[0]).is_err());
    a.busy.clear();
    let ready = prepare(&mut s, &a, "human", &review, &[0]).unwrap();
    let checkpoint = s.updates.clone();
    decide(&mut s, &mut a, "human", &review, &[0], None).unwrap();
    assert!(decide(&mut s, &mut a, "human", &review, &[0], Some(ready)).is_err());
    assert_eq!(s.updates, checkpoint);
    let guard = s.guard();
    let view = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    let review = view["id"].as_str().unwrap().to_owned();
    let ready = prepare(&mut s, &a, "human", &review, &[0]).unwrap();
    edit(&mut s, 2, "Human ");
    let checkpoint = s.updates.clone();
    assert!(decide(&mut s, &mut a, "human", &review, &[0], Some(ready)).is_err());
    assert_eq!(s.updates, checkpoint);
}
#[test]
fn mcp_uses_the_same_comparison_for_partial_decisions_without_replaying_the_original_batch() {
    let (_root, mut s, mut a) = fixture();
    a.token = Some("authorized".into());
    let request = json!({"document":{"id":s.generation,"guard":s.guard()},"request_id":"local","summary":"Test","steps":[{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"New "}},{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[2]},"from":0,"to":0},"text":"New "}}]});
    let pending = crate::agent::dispatch(&mut s, &mut a, "proposals.submit", &request).unwrap();
    let id = pending["outcome"]["id"].as_str().unwrap().to_owned();
    let guard = s.guard();
    let view = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Proposal { id: id.clone() },
    )
    .unwrap();
    assert_eq!(view["report"]["closed"], false);
    let review = view["id"].as_str().unwrap().to_owned();
    assert!(crate::agent::commit_accepted(&mut s, &mut a, &id, "human").is_err());
    let ready = prepare(&mut s, &a, "human", &review, &[0]).unwrap();
    decide(&mut s, &mut a, "human", &review, &[0], Some(ready)).unwrap();
    assert!(prepare(&mut s, &a, "human", &review, &[1]).is_ok());
    let retry =
        crate::agent::dispatch(&mut s, &mut a, "proposals.get", &json!({"proposal":id})).unwrap();
    assert_eq!(retry["outcome"]["status"], "pending");
    decide(&mut s, &mut a, "human", &review, &[1], None).unwrap();
    assert_eq!(a.proposal.as_ref().unwrap().status, "partially_accepted");
    assert_eq!(
        s.ensure_owner().unwrap().state().document.content[2].content[0]
            .text
            .as_deref(),
        Some("Old two")
    );
    let view = read(&s, &a, "other", &review).unwrap();
    assert_eq!(view["report"]["groups"][0]["decision"], "applied");
    assert_eq!(view["report"]["groups"][1]["decision"], "refused");
    a.token = None;
    assert!(prepare(&mut s, &a, "human", &review, &[1]).is_err());
}

#[test]
fn historical_pairs_are_read_only_even_if_the_current_document_happens_to_match() {
    let (root, mut s, mut a) = fixture();
    let first = version(&mut s, &root.path().join("p.typsto"));
    edit(&mut s, 0, "New ");
    let second = version(&mut s, &root.path().join("p.typsto"));
    let guard = s.guard();
    let view = open(
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
    let review = view["id"].as_str().unwrap();
    assert_eq!(view["report"]["read_only"], true);
    assert!(prepare(&mut s, &a, "human", review, &[0]).is_err());
    assert!(decide(&mut s, &mut a, "human", review, &[0], None).is_err());
}
#[test]
fn historical_packages_never_become_decisions_or_replace_application_libraries() {
    let (_root, mut s, mut a) = fixture();
    let mut archived = s.stored_project().unwrap().current().clone();
    let libraries = typstographe_core::packages::for_template(&archived.template);
    let mut old = (*libraries).clone();
    old.insert(
        "capabilities/dates.typ".into(),
        b"#panic(\"Archived libraries must not run inside the application\")"
            .as_slice()
            .into(),
    );
    archived.dependencies = std::sync::Arc::new(old);
    let project = typstographe_core::content::project::Project::new(archived.clone())
        .unwrap()
        .prepare_version(
            SnapshotDetails::default(),
            typstographe_core::content::project::SnapshotReason::Manual,
        )
        .unwrap();
    let id = project.versions()[0].id;
    s.project = std::sync::OnceLock::from(crate::project::Context::new(project, true));
    s.owner = None;
    let guard = s.guard();
    let unchanged = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    assert!(unchanged["report"]["groups"].as_array().unwrap().is_empty());
    assert!(unchanged["report"]["files"].as_array().unwrap().is_empty());
    crate::versions::preview(archived.clone()).unwrap();
    edit(&mut s, 0, "New ");
    edit(&mut s, 2, "New ");
    let guard = s.guard();
    let view = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    assert_eq!(view["report"]["groups"].as_array().unwrap().len(), 2);
    let review = view["id"].as_str().unwrap();
    let ready = prepare(&mut s, &a, "human", review, &[0]).unwrap();
    assert_eq!(ready.archive().dependencies, libraries);
    decide(&mut s, &mut a, "human", review, &[0], Some(ready)).unwrap();
    for direction in [Direction::Undo, Direction::Redo] {
        s.history("human", direction).unwrap();
        assert_eq!(s.dependencies_for(&s.settings), libraries);
        assert_eq!(
            s.stored_project().unwrap().current().dependencies,
            libraries
        );
    }
    let current = s.stored_project().unwrap().current().clone();
    assert_eq!(current.document.content[0], archived.document.content[0]);
    let backup = s
        .project_recovery(current.state(), current.yjs.clone())
        .unwrap();
    let recovered = backup.archive.restore().unwrap();
    assert_eq!(recovered.current().dependencies, libraries);
    assert_eq!(
        recovered.version(id).unwrap().dependencies,
        archived.dependencies
    );
}

#[test]
fn frozen_mcp_states_remain_consultable_after_a_human_edit_before_the_first_opening() {
    let (_root, mut s, mut a) = fixture();
    a.token = Some("authorized".into());
    let request = json!({"document":{"id":s.generation,"guard":s.guard()},"request_id":"stale-view","steps":[{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Proposal "}}]});
    let result = crate::agent::dispatch(&mut s, &mut a, "proposals.submit", &request).unwrap();
    let id = result["outcome"]["id"].as_str().unwrap().to_owned();
    edit(&mut s, 0, "Human ");
    let confirmed = s.ensure_owner().unwrap().state().clone();
    let guard = s.guard();
    let view = open(&mut s, &mut a, "human", &guard, Origin::Proposal { id }).unwrap();
    assert_eq!(view["report"]["stale"], true);
    assert_eq!(
        view["before"]["state"]["document"]["content"][0]["content"][0]["text"],
        "Old one"
    );
    assert_eq!(
        view["after"]["state"]["document"]["content"][0]["content"][0]["text"],
        "Proposal Old one"
    );
    let review = view["id"].as_str().unwrap();
    assert!(prepare(&mut s, &a, "human", review, &[0]).is_err());
    assert_eq!(s.ensure_owner().unwrap().state(), &confirmed);
}
#[test]
fn a_delayed_close_cannot_remove_a_more_recent_frozen_view() {
    let (root, mut s, mut a) = fixture();
    let id = version(&mut s, &root.path().join("p.typsto"));
    edit(&mut s, 0, "New ");
    let guard = s.guard();
    let one = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    let two = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    close(&mut s, "human", one["id"].as_str().unwrap()).unwrap();
    assert!(read(&s, &a, "human", two["id"].as_str().unwrap()).is_ok());
}

#[test]
fn detached_preparation_cannot_publish_over_an_edit_made_after_checkpoint_capture() {
    let (root, mut s, mut a) = fixture();
    let id = version(&mut s, &root.path().join("p.typsto"));
    edit(&mut s, 0, "New ");
    let guard = s.guard();
    let view = open(
        &mut s,
        &mut a,
        "human",
        &guard,
        Origin::Version { id, against: None },
    )
    .unwrap();
    let id = view["id"].as_str().unwrap().to_owned();
    let checkpoint = capture(&mut s, &a, "human", &id, &[0]).unwrap();
    edit(&mut s, 2, "Human ");
    let current = s.updates.clone();
    let ready = checkpoint.prepare().unwrap();
    assert!(decide(&mut s, &mut a, "human", &id, &[0], Some(ready)).is_err());
    assert_eq!(s.updates, current);
}
