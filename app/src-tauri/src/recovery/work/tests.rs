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

#[test]
fn a_second_incident_preserves_the_latest_resumed_form_without_duplicating_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = setup(dir.path());
    conserve(&mut first, "main", "Original private");
    drop(first);
    let mut resumed = setup(dir.path());
    let entry = resumed.work_list().unwrap().remove(0);
    resumed
        .work_restore("main", &entry.id, resumed.revision)
        .unwrap();
    let source = resumed.join("main").recovered_items[0].clone();
    let generation = resumed.generation.clone();
    let epoch = resumed.open_private_recovery("main", &generation).unwrap();
    let cp = resumed.recovery_checkpoint().unwrap();
    let mut form = source.payload.clone();
    form["sourceId"] = json!(source.id);
    form["data"]["name"] = json!("Latest private input");
    resumed
        .conserve_private(
            "main",
            drafts::Input {
                generation,
                epoch,
                sequence: 1,
                update: cp.update.clone(),
                document: json!(cp.document),
                settings: json!(cp.settings),
                resources: cp.resources,
                forms: vec![form],
                fields: vec![],
                rejected: false,
                recovered: vec![source],
                retain: false,
            },
        )
        .unwrap();
    drop(resumed);
    let mut next = setup(dir.path());
    let entry = next.work_list().unwrap().remove(0);
    assert_eq!(entry.pending, 1);
    next.work_restore("main", &entry.id, next.revision).unwrap();
    let restored = next.join("main").recovered_items;
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].payload["data"]["name"], "Latest private input");
}
#[test]
fn rejected_json_exports_open_as_isolated_input_without_publishing_their_invalid_body() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old-export.json");
    fs::write(&path,serde_json::to_vec(&json!({"format":"typstographe-rejected-draft-v1","content":{"document":{"type":"unknown","text":"Private rejected paragraph"}},"update":[255]})).unwrap()).unwrap();
    let record = read_work(&path).unwrap();
    let mut s = setup(dir.path());
    s.restore_work_record("main", record).unwrap();
    assert!(
        !serde_json::to_string(s.ensure_owner().unwrap().state())
            .unwrap()
            .contains("Private rejected paragraph")
    );
    assert!(
        serde_json::to_string(&s.join("main").recovered_items)
            .unwrap()
            .contains("Private rejected paragraph")
    );
    assert!(path.exists());
}

#[test]
fn removing_one_copy_never_deletes_a_summary_for_a_different_document() {
    let dir = tempfile::tempdir().unwrap();
    let mut old = setup(dir.path());
    conserve(&mut old, "main", "First private form");
    let mut other = old.recovery_checkpoint().unwrap();
    other.document.content[0].content[0].text = Some("Another document".into());
    other.update =
        SharedDocument::from_state(&State::new(other.document.clone(), other.settings.clone()))
            .unwrap()
            .update();
    old.recovery
        .as_ref()
        .unwrap()
        .write(other, "Other project".into())
        .unwrap();
    drop(old);
    let current = setup(dir.path());
    let entry = current.work_list().unwrap().remove(0);
    current.work_delete(&entry.id).unwrap();
    let remaining = current.work_list().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].name, "Other project");
}

#[test]
fn cancelling_the_last_form_of_an_already_saved_document_leaves_no_false_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = setup(dir.path());
    s.mark_clean().unwrap();
    conserve(&mut s, "main", "Temporary private input");
    let path = drafts::path(&s.root, &s.generation, "main").unwrap();
    let mut input = drafts::read(&path).unwrap().input;
    input.sequence += 1;
    input.forms.clear();
    input.retain = false;
    s.conserve_private("main", input).unwrap();
    assert!(!path.exists());
    assert!(!s.root.join("session.json").exists());
    drop(s);
    assert!(setup(dir.path()).work_list().unwrap().is_empty());
}

#[test]
fn a_pending_field_keeps_its_original_base_when_a_later_checkpoint_contains_peer_edits() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = setup(dir.path());
    conserve(&mut s, "main", "Private form");
    let mut record = drafts::read(&drafts::path(&s.root, &s.generation, "main").unwrap()).unwrap();
    record.input.fields = vec![json!({
        "target":{"kind":"metadata","field":"letter.subject"},
        "value":"Private changed subject", "base":"Subject before concurrent editing"
    })];
    let field = items(&record)
        .into_iter()
        .find(|i| i.kind == "field")
        .unwrap();
    assert_eq!(field.payload["base"], "Subject before concurrent editing");
}

#[test]
fn a_transition_also_waits_for_registered_windows_with_suspended_synchronization() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = setup(dir.path());
    s.join("suspended");
    s.connection("suspended", &s.generation.clone(), false)
        .unwrap();
    assert!(
        !s.synchronization()
            .windows
            .contains(&"suspended".to_string())
    );
    let transition = s.begin_transition("main", &s.generation.clone()).unwrap();
    assert!(transition.windows.contains("suspended"));
    assert!(!transition.ready());
}

#[test]
fn a_closed_views_private_copy_remains_available_while_other_views_stay_open() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = setup(dir.path());
    s.join("closed");
    conserve(&mut s, "closed", "Private closed view");
    assert!(s.work_list().unwrap().is_empty());
    s.leave("closed");
    let entries = s.work_list().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].window, "closed");
    assert!(!s.work_list().unwrap().iter().any(|e| e.window == "main"));
}
