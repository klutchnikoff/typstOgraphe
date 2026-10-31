use super::*;
use serde_json::json;

fn session(root: &Path) -> Session {
    let mut s = Session::fresh(root.into());
    let state = State::new(serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Confirmed"}]},{"type":"paragraph"}]})).unwrap(), Default::default());
    let shared = SharedDocument::from_state(&state).unwrap();
    s.replace(
        state.document,
        state.settings,
        Default::default(),
        shared.update(),
    );
    s.join("main");
    s.join("peer");
    s
}
fn input(s: &mut Session, window: &str) -> Input {
    let generation = s.generation.clone();
    let epoch = s.open_private_recovery(window, &generation).unwrap();
    let owner = s.ensure_owner().unwrap();
    Input {
        generation,
        epoch,
        sequence: 1,
        update: owner.update_bytes(),
        document: serde_json::to_value(&owner.state().document).unwrap(),
        settings: serde_json::to_value(&owner.state().settings).unwrap(),
        fields: vec![],
        forms: vec![],
        resources: vec![],
        rejected: false,
        recovered: vec![],
        retain: false,
    }
}

#[test]
fn unconfirmed_forms_and_fields_survive_a_new_process_without_entering_the_owner() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path());
    let confirmed = s.ensure_owner().unwrap().state().clone();
    let mut request = input(&mut s, "main");
    request
        .fields
        .push(json!({"target":{"kind":"metadata","field":"letter.subject"},"value":"Pending"}));
    request.forms.push(json!({"id":"author","context":{"kind":"record","collection":"authors"},"data":{"name":"Draft author é😀"}}));
    let generation = s.generation.clone();
    s.conserve_private("main", request).unwrap();
    let file = path(dir.path(), &generation, "main").unwrap();
    assert_eq!(s.ensure_owner().unwrap().state(), &confirmed);
    s.mark_clean().unwrap();
    drop(s);
    let record = read(&file).unwrap();
    assert_eq!(record.input.fields[0]["value"], "Pending");
    assert_eq!(record.input.forms[0]["data"]["name"], "Draft author é😀");
    let restored = Session::from_checkpoint(dir.path().join("reopened"), record.confirmed).unwrap();
    assert_eq!(restored.fixture.unwrap(), confirmed.document);
}

#[test]
fn revisions_views_and_windows_cannot_overwrite_or_clean_each_others_private_input() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path());
    let mut main = input(&mut s, "main");
    main.forms.push(json!({"name":"First"}));
    s.conserve_private("main", main.clone()).unwrap();
    let mut peer = input(&mut s, "peer");
    peer.forms.push(json!({"name":"Peer"}));
    s.conserve_private("peer", peer).unwrap();
    let file = path(dir.path(), &s.generation, "main").unwrap();
    main.sequence = 3;
    main.forms[0] = json!({"name":"Latest"});
    s.conserve_private("main", main.clone()).unwrap();
    main.sequence = 2;
    main.forms[0] = json!({"name":"Obsolete"});
    assert!(!s.conserve_private("main", main.clone()).unwrap());
    assert_eq!(read(&file).unwrap().input.forms[0]["name"], "Latest");
    let generation = s.generation.clone();
    s.open_private_recovery("main", &generation).unwrap();
    assert!(s.conserve_private("main", main.clone()).is_err());
    assert!(s.conserve_private("stranger", main.clone()).is_err());
    s.mark_clean().unwrap();
    assert_eq!(
        read(&path(dir.path(), &generation, "peer").unwrap())
            .unwrap()
            .input
            .forms[0]["name"],
        "Peer"
    );
    s.reset_document_generation();
    assert!(s.conserve_private("main", main).is_err());
    assert!(
        fs::read_dir(file.parent().unwrap())
            .unwrap()
            .flatten()
            .any(|entry| read(&entry.path())
                .is_ok_and(|r| r.window == "main" && r.input.forms[0]["name"] == "Latest"))
    );
}

#[test]
fn rejected_body_and_private_images_are_kept_but_invalid_replacements_preserve_the_previous_copy() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path());
    let mut request = input(&mut s, "main");
    request.document =
        json!({"type":"doc","content":[{"type":"figure","attrs":{"src":"missing.svg"}}]});
    request.rejected = true;
    request.settings["metadata"]["fields"]["options"]["paper"] = json!("invalid-paper");
    use base64::Engine;
    request.resources.push(Resource {
        path: "images/chosen.svg".into(),
        data: base64::engine::general_purpose::STANDARD
            .encode(include_bytes!("../../../../../fixtures/signature.svg")),
    });
    s.conserve_private("main", request.clone()).unwrap();
    let file = path(dir.path(), &s.generation, "main").unwrap();
    let before = fs::read(&file).unwrap();
    assert!(s.assets.is_empty());
    let recovered = read(&file).unwrap();
    assert_eq!(recovered.input.document, request.document);
    assert_eq!(recovered.input.settings, request.settings);
    assert_eq!(recovered.input.resources[0].data, request.resources[0].data);
    request.sequence += 1;
    request.update = vec![255];
    assert!(s.conserve_private("main", request.clone()).is_err());
    assert_eq!(fs::read(&file).unwrap(), before);
    request.update = recovered.input.update;
    request.resources[0].path = "../escape.svg".into();
    assert!(s.conserve_private("main", request.clone()).is_err());
    assert_eq!(fs::read(&file).unwrap(), before);
    request.resources.clear();
    request.sequence += 1;
    s.conserve_private("main", request).unwrap();
}

#[test]
fn an_oversized_write_keeps_the_previous_copy_and_a_retry_with_the_same_sequence_is_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path());
    let mut request = input(&mut s, "main");
    request.forms.push(json!({"text":"Keep me"}));
    s.conserve_private("main", request.clone()).unwrap();
    let file = path(dir.path(), &s.generation, "main").unwrap();
    let before = fs::read(&file).unwrap();
    request.sequence += 1;
    request.forms = vec![json!({"text":"x".repeat(MAX_DOCUMENT_BYTES as usize)})];
    assert!(s.conserve_private("main", request.clone()).is_err());
    assert_eq!(fs::read(&file).unwrap(), before);
    request.forms = vec![json!({"text":"Retried"})];
    s.conserve_private("main", request).unwrap();
    assert_eq!(read(&file).unwrap().input.forms[0]["text"], "Retried");
}

#[test]
fn a_clean_confirmed_copy_is_removed_without_deleting_private_or_unreadable_siblings() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path());
    let clean = input(&mut s, "main");
    s.conserve_private("main", clean).unwrap();
    let mut private = input(&mut s, "peer");
    private.forms.push(json!({"text":"Keep"}));
    s.conserve_private("peer", private).unwrap();
    let corrupt = directory(dir.path(), &s.generation)
        .unwrap()
        .join("corrupt.json");
    fs::write(&corrupt, b"bad").unwrap();
    s.mark_clean().unwrap();
    assert!(!path(dir.path(), &s.generation, "main").unwrap().exists());
    assert!(path(dir.path(), &s.generation, "peer").unwrap().exists());
    assert!(corrupt.exists());
}

#[cfg(unix)]
#[test]
fn private_recovery_paths_cannot_follow_a_symlink() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("drafts")).unwrap();
    assert!(path(root.path(), &uuid::Uuid::new_v4().to_string(), "main").is_err());
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}

#[test]
fn process_exit_without_cleanup_preserves_private_input_and_a_recoverable_confirmed_base() {
    const CHILD: &str = "TYPSTOGRAPHE_DRAFT_RECOVERY_CHILD";
    if let Some(base) = std::env::var_os(CHILD) {
        let base = PathBuf::from(base);
        let managed = Session::managed(base.clone()).unwrap();
        let mut s = session(&managed.root);
        s.recovery = managed.recovery;
        let mut request = input(&mut s, "main");
        request
            .forms
            .push(json!({"name":"Author left open before interruption"}));
        s.conserve_private("main", request).unwrap();
        fs::write(
            base.join("record-path"),
            path(&s.root, &s.generation, "main")
                .unwrap()
                .to_string_lossy()
                .as_bytes(),
        )
        .unwrap();
        // No destructors, window-close handler, or recovery cleanup runs.
        std::process::exit(73);
    }
    let base = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "recovery::drafts::tests::process_exit_without_cleanup_preserves_private_input_and_a_recoverable_confirmed_base"])
        .env(CHILD, base.path()).status().unwrap();
    assert_eq!(status.code(), Some(73));
    let file = fs::read_to_string(base.path().join("record-path")).unwrap();
    assert_eq!(
        read(Path::new(&file)).unwrap().input.forms[0]["name"],
        "Author left open before interruption"
    );
    let next = crate::recovery::Store::new(base.path().into()).unwrap();
    let entries = next.list().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].error.is_none());
}

#[test]
fn a_storage_failure_keeps_the_previous_copy_and_can_be_retried() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path());
    let mut request = input(&mut s, "main");
    request.forms.push(json!({"text":"Previous private work"}));
    s.conserve_private("main", request.clone()).unwrap();
    let file = path(dir.path(), &s.generation, "main").unwrap();
    let previous = fs::read(&file).unwrap();
    let backup = file.with_extension("backup");
    fs::rename(&file, &backup).unwrap();
    fs::create_dir(&file).unwrap(); // Deterministic IO failure on all supported systems.
    request.sequence += 1;
    request.forms = vec![json!({"text":"Retried private work"})];
    assert!(s.conserve_private("main", request.clone()).is_err());
    assert_eq!(fs::read(&backup).unwrap(), previous);
    fs::remove_dir(&file).unwrap();
    fs::rename(&backup, &file).unwrap();
    s.conserve_private("main", request).unwrap();
    assert_eq!(
        read(&file).unwrap().input.forms[0]["text"],
        "Retried private work"
    );
}

#[test]
fn a_reloaded_view_does_not_consume_unrestored_private_input() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("new-session");
    let mut s = session(&root);
    let mut old = input(&mut s, "main");
    old.forms.push(json!({"text":"Unrestored form"}));
    s.conserve_private("main", old.clone()).unwrap();
    let current_file = path(&root, &s.generation, "main").unwrap();
    let mut current = input(&mut s, "main");
    assert_ne!(current.epoch, old.epoch);
    assert!(!current_file.exists());
    current.forms.push(json!({"text":"New view input"}));
    s.conserve_private("main", current).unwrap();
    assert!(s.conserve_private("main", old).is_err());
    s.mark_clean().unwrap();
    let records: Vec<_> = fs::read_dir(current_file.parent().unwrap())
        .unwrap()
        .flatten()
        .filter_map(|entry| read(&entry.path()).ok())
        .collect();
    assert_eq!(records.len(), 2);
    assert!(
        records
            .iter()
            .any(|r| r.input.forms[0]["text"] == "Unrestored form")
    );
    assert!(
        records
            .iter()
            .any(|r| r.input.forms[0]["text"] == "New view input")
    );
}
