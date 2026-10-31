use super::*;
fn setup(root: &Path) -> Session {
    let mut s = Session::managed(root.into()).unwrap();
    let state=State::new(serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Confirmed work"}]},{"type":"paragraph"}]})).unwrap(),Default::default());
    let shared = SharedDocument::from_state(&state).unwrap();
    s.replace(
        state.document,
        state.settings,
        Default::default(),
        shared.update(),
    );
    s.join("main");
    s
}
fn conserve(s: &mut Session, window: &str, form: &str) {
    let generation = s.generation.clone();
    let epoch = s.open_private_recovery(window, &generation).unwrap();
    let owner = s.ensure_owner().unwrap();
    let record = drafts::Input {
        generation,
        epoch,
        sequence: 1,
        update: owner.update_bytes(),
        document: json!(owner.state().document),
        settings: json!(owner.state().settings),
        fields: vec![],
        forms: vec![
            json!({"id":"form","form":"author","context":{"kind":"record","collection":"authors"},"data":{"name":form}}),
        ],
        resources: vec![],
        rejected: false,
        recovered: vec![],
        retain: true,
    };
    s.conserve_private(window, record).unwrap();
}
#[test]
fn isolated_resume_is_durable_before_consumption_and_does_not_share_forms() {
    let dir = tempfile::tempdir().unwrap();
    let mut old = setup(dir.path());
    conserve(&mut old, "main", "Private author");
    let previous = old.root.clone();
    drop(old);
    let mut current = setup(dir.path());
    let entries = current.work_list().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].pending, 1);
    current
        .work_restore("main", &entries[0].id, current.revision)
        .unwrap();
    assert!(current.file_lease.is_none());
    assert_eq!(current.join("main").recovered_items.len(), 1);
    assert!(current.join("peer").recovered_items.is_empty());
    let cp = current.ensure_owner().unwrap().state();
    assert!(
        !serde_json::to_string(cp)
            .unwrap()
            .contains("Private author")
    );
    let file = drafts::path(&current.root, &current.generation, "main").unwrap();
    assert_eq!(drafts::read(&file).unwrap().input.recovered.len(), 1);
    assert!(current.recovered_name.is_some());
    assert!(!has_envelopes(&previous));
}
#[test]
fn failed_resume_keeps_the_source_and_current_work() {
    let dir = tempfile::tempdir().unwrap();
    let mut old = setup(dir.path());
    conserve(&mut old, "main", "Keep private");
    drop(old);
    let mut current = setup(dir.path());
    let entry = current.work_list().unwrap().remove(0);
    let before = current.ensure_owner().unwrap().state().clone();
    let generation = current.generation.clone();
    fs::write(current.root.join("drafts"), b"not a directory").unwrap();
    assert!(
        current
            .work_restore("main", &entry.id, current.revision)
            .is_err()
    );
    assert_eq!(current.generation, generation);
    assert_eq!(current.ensure_owner().unwrap().state(), &before);
    assert!(current.claim_work(&entry.id).unwrap().1.exists());
}
#[test]
fn a_transition_needs_durable_acknowledgements_from_all_views() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = setup(dir.path());
    s.join("peer");
    let generation = s.generation.clone();
    let tx = s.begin_transition("main", &generation).unwrap();
    assert!(s.lifecycle(s.revision).is_err());
    assert!(
        s.acknowledge_transition(
            "main",
            super::super::transition::Acknowledgement {
                id: tx.id.clone(),
                error: None,
                abandon: false
            }
        )
        .is_err()
    );
    for window in ["main", "peer"] {
        conserve(&mut s, window, window);
        s.acknowledge_transition(
            window,
            super::super::transition::Acknowledgement {
                id: tx.id.clone(),
                error: None,
                abandon: false,
            },
        )
        .unwrap();
    }
    assert!(s.lifecycle(s.revision).is_ok());
    s.revision += 1;
    assert!(s.lifecycle(s.revision).is_err());
}
#[test]
fn abandonment_removes_only_the_current_windows_copy_and_retains_the_peer() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = setup(dir.path());
    s.join("peer");
    conserve(&mut s, "main", "Local");
    conserve(&mut s, "peer", "Other");
    let generation = s.generation.clone();
    let peer = drafts::path(&s.root, &generation, "peer").unwrap();
    let bytes = fs::read(&peer).unwrap();
    s.abandon_window_work("main", &generation).unwrap();
    assert_eq!(fs::read(peer).unwrap(), bytes);
    assert!(!drafts::path(&s.root, &generation, "main").unwrap().exists());
    assert!(s.work_delete("../../elsewhere").is_err());
}
