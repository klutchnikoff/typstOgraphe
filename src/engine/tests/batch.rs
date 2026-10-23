use super::*;
use crate::document_types::MetadataField;

fn step(id: &str, params: Value) -> Call {
    Call {
        id: id.to_owned().try_into().unwrap(),
        params,
    }
}
fn params(e: &Engine, steps: Vec<Call>) -> BatchParams {
    BatchParams {
        guard: e.target_guard(),
        steps,
        save: None,
    }
}
fn batch(e: &mut Engine, steps: Vec<Call>) -> Outcome {
    let p = params(e, steps);
    e.run_call(Command::Batch(p).to_call()).unwrap()
}
fn image() -> ImageInput {
    ImageInput::from(&crate::assets::ImageData {
        extension: "svg".into(),
        data: include_bytes!("../../../tests/fixtures/signature.svg")
            .as_slice()
            .into(),
    })
}
#[test]
fn typed_and_json_steps_publish_one_revision_and_one_complete_undo() {
    let mut e = engine("Avant");
    e.run(Command::InsertText(InsertTextParams {
        target: TextTarget::default(),
        text: "Un ".into(),
        input_rules: InputRules::Literal,
    }))
    .unwrap();
    let before = e.stored_document();
    let selection = e.selection();
    let guard = e.target_guard();
    let old_undo = e.undo.len();
    let mut next = guard;
    next.revision += 1;
    let out = batch(
        &mut e,
        vec![
            step(
                "metadata.set",
                json!({"guard":guard,"field":"letter.subject","value":"Réunion"}),
            ),
            step(
                "document.set_option",
                json!({"guard":next,"option":{"key":"language","value":"en"}}),
            ),
            step("text.insert", json!({"text":"Été, Noël et 💡. "})),
            step("text.bold", json!({"target":{"kind":"all"},"value":"on"})),
        ],
    );
    assert!(out.changed);
    assert_eq!(out.guard.revision, guard.revision + 1);
    assert_eq!(out.guard.document_id, guard.document_id);
    assert_eq!(e.undo.len(), old_undo + 1);
    assert!(e.redo.is_empty());
    assert_eq!(e.metadata.courrier().objet, "Réunion");
    assert!(e.document.view.styles.iter().all(|s| s.bold));
    assert!(e.document.view.text.contains("Été, Noël et 💡."));
    assert_eq!(out.value.as_ref().unwrap()["steps"], 4);
    assert!(out.value.as_ref().unwrap()["saved"].is_null());
    assert!(out.effects.contains(&Effect::Render));
    e.resolve_target(out.selection.unwrap()).unwrap();
    let final_state = e.stored_document();
    let final_selection = e.selection();
    e.run(Command::Undo(Empty {})).unwrap();
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), selection);
    e.run(Command::Redo(Empty {})).unwrap();
    assert!(e.stored_document() == final_state);
    assert_eq!(e.selection(), final_selection);
}
#[test]
fn a_late_failure_keeps_body_resources_selection_history_saved_state_and_disk_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("current.typsto");
    let mut e = engine("Avant");
    crate::storage::save(&path, &e.stored_document()).unwrap();
    e.run(Command::InsertText(InsertTextParams {
        target: TextTarget::default(),
        text: "Temporaire".into(),
        input_rules: InputRules::Literal,
    }))
    .unwrap();
    e.run(Command::Undo(Empty {})).unwrap();
    let stored = e.stored_document();
    let selection = e.selection();
    let guard = e.target_guard();
    let saved = e.saved.clone();
    let changed_at = e.changed_at;
    let disk = std::fs::read(&path).unwrap();
    let (undo, redo) = (e.undo.len(), e.redo.len());
    let mut p = params(
        &e,
        vec![
            step("text.insert", json!({"text":"À perdre "})),
            step(
                "insert.image",
                json!({"image":image(),"caption":"Image temporaire"}),
            ),
            step(
                "text.link",
                json!({"target":{"kind":"all"},"url":"javascript:invalid"}),
            ),
        ],
    );
    p.save = Some(BatchSave::As {
        path: path.clone(),
        overwrite: true,
    });
    let error = e.run(Command::Batch(p)).unwrap_err();
    assert_eq!(error.reason(), "batch_step");
    assert_eq!(error.params["step"], 2);
    assert_eq!(error.params["command"], "text.link");
    assert!(e.stored_document() == stored);
    assert_eq!(e.selection(), selection);
    assert_eq!(e.target_guard(), guard);
    assert!(e.saved == saved);
    assert_eq!(e.changed_at, changed_at);
    assert_eq!((e.undo.len(), e.redo.len()), (undo, redo));
    assert_eq!(std::fs::read(path).unwrap(), disk);
}
#[test]
fn all_external_effects_history_queries_and_nested_batches_are_rejected_before_imports() {
    let mut e = engine("Avant");
    let guard = e.target_guard();
    let forbidden = vec![
        step("document.save", json!({"guard":guard})),
        step(
            "document.save_as",
            json!({"guard":guard,"path":"must-not-exist.typsto"}),
        ),
        step(
            "document.export",
            json!({"guard":guard,"directory":"must-not-exist"}),
        ),
        step(
            "document.new",
            json!({"guard":guard,"type":"courrier","model":"courrier.model.personal"}),
        ),
        step(
            "document.open",
            json!({"guard":guard,"path":"missing.typsto"}),
        ),
        step(
            "document.close",
            json!({"guard":guard,"discard_changes":true}),
        ),
        step("recovery.restore", json!({"guard":guard,"reference":1})),
        step("recovery.delete", json!({"reference":1})),
        step("edit.undo", json!({})),
        step("edit.redo", json!({})),
        step(
            "document.batch",
            json!({"guard":guard,"steps":[{"id":"text.bold","params":{}}]}),
        ),
        step("metadata.get", json!({})),
        step("spelling.learn", json!({})),
    ];
    let stored = e.stored_document();
    for command in forbidden {
        let id = command.id.as_str().to_string();
        let p = params(
            &e,
            vec![
                step(
                    "insert.image",
                    json!({"image":{"kind":"path","path":"absent.svg"}}),
                ),
                command,
            ],
        );
        let error = e.run(Command::Batch(p)).unwrap_err();
        assert_eq!(error.params["step"], 1, "{id}: {error}");
        assert_eq!(error.params["command"], id);
        assert!(e.stored_document() == stored);
        assert_eq!(e.target_guard(), guard);
    }
}
#[test]
fn limits_guards_and_parameter_errors_are_checked_without_mutating_the_session() {
    let mut e = engine("Avant");
    let before = e.stored_document();
    let guard = e.target_guard();
    for (steps, reason) in [
        (vec![], "batch_step_limit"),
        (
            vec![step("text.bold", json!({})); super::super::batch::MAX_BATCH_STEPS + 1],
            "batch_step_limit",
        ),
        (
            vec![step(
                "text.insert",
                json!({"text":"x".repeat(super::super::batch::MAX_BATCH_BYTES)}),
            )],
            "batch_size_limit",
        ),
    ] {
        let p = params(&e, steps);
        assert_eq!(e.run(Command::Batch(p)).unwrap_err().reason(), reason);
        assert_eq!(e.target_guard(), guard);
        assert!(e.stored_document() == before);
    }
    assert!(
        Command::from_call(
            "document.batch",
            json!({"guard":guard,"steps":[],"typo":true})
        )
        .is_err()
    );
    let p = params(
        &e,
        vec![step(
            "metadata.set",
            json!({"field":"letter.subject","value":"Titre"}),
        )],
    );
    let err = e.run(Command::Batch(p)).unwrap_err();
    assert_eq!(err.params["step"], 0);
    assert_eq!(err.params["error"]["code"], "invalid_params");
    let mut p = params(&e, vec![step("text.insert", json!({"text":"Bonjour"}))]);
    p.guard.revision += 1;
    assert_eq!(
        e.run(Command::Batch(p)).unwrap_err().reason(),
        "stale_revision"
    );
    let mut p = params(&e, vec![step("text.bold", json!({}))]);
    p.guard = engine("Autre").target_guard();
    assert_eq!(
        e.run(Command::Batch(p)).unwrap_err().reason(),
        "foreign_document"
    );
    let mut empty = Engine::empty();
    let p = params(&empty, vec![step("text.bold", json!({}))]);
    assert_eq!(
        empty.run(Command::Batch(p)).unwrap_err().reason(),
        "no_document"
    );
}
#[test]
fn explicit_targets_are_not_reanchored_after_a_step_changes_the_document() {
    let mut e = engine("Été avant");
    let target = explicit(&e, 0..3);
    let before = e.stored_document();
    let guard = e.target_guard();
    let p = params(
        &e,
        vec![
            step("text.insert", json!({"text":"Texte ajouté "})),
            step("text.bold", json!({"target":target})),
        ],
    );
    let error = e.run(Command::Batch(p)).unwrap_err();
    assert_eq!(error.params["step"], 1);
    assert_eq!(error.params["error"]["params"]["reason"], "stale_revision");
    assert!(e.stored_document() == before);
    assert_eq!(e.target_guard(), guard);
    // Les cibles symboliques se résolvent sur le brouillon courant, comme hors lot.
    batch(
        &mut e,
        vec![
            step("text.insert", json!({"text":"Texte ajouté "})),
            step("text.italic", json!({"target":{"kind":"all"},"value":"on"})),
        ],
    );
    assert!(e.document.view.styles.iter().all(|s| s.italic));
    assert_eq!(e.undo.len(), 1);
}
#[test]
fn net_noop_selection_and_typing_style_batches_preserve_redo_and_do_not_mark_dirty() {
    let mut e = engine("Avant");
    batch(
        &mut e,
        vec![step("text.insert", json!({"text":"À annuler "}))],
    );
    e.run(Command::Undo(Empty {})).unwrap();
    let guard = e.target_guard();
    let before = e.stored_document();
    let out = batch(
        &mut e,
        vec![
            step("text.bold", json!({"target":{"kind":"all"},"value":"on"})),
            step("text.bold", json!({"target":{"kind":"all"},"value":"off"})),
            step("text.select", json!({"target":{"kind":"all"}})),
        ],
    );
    assert!(!out.changed);
    assert_eq!(out.guard, guard);
    assert!(!e.is_dirty());
    assert!(e.stored_document() == before);
    assert_eq!(e.selection().range(), 0..5);
    assert_eq!(e.redo.len(), 1);
    assert!(e.undo.is_empty());
    assert!(out.effects.is_empty());
    batch(
        &mut e,
        vec![
            step("text.select", json!({})),
            step("text.bold", json!({"value":"off"})),
        ],
    );
    e.run(Command::Redo(Empty {})).unwrap();
    assert!(e.document.view.text.starts_with("À annuler"));
    let mut empty = engine("");
    let out = batch(&mut empty, vec![step("text.bold", json!({"value":"on"}))]);
    assert!(!out.changed && empty.document.view.typing_style.bold);
    assert!(empty.undo.is_empty());
}
#[test]
fn a_single_final_atomic_save_updates_the_clean_marker_and_undo_redo_remains_one_step() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let mut e = engine("Avant");
    e.run(Command::SaveDocument(SaveDocumentParams {
        guard: e.target_guard(),
        path: Some(path.clone()),
    }))
    .unwrap();
    let before = e.stored_document();
    let guard = e.target_guard();
    let mut p = params(
        &e,
        vec![
            step("text.insert", json!({"text":"Après "})),
            step("text.italic", json!({"target":{"kind":"all"},"value":"on"})),
        ],
    );
    p.save = Some(BatchSave::Current {});
    let out = e.run(Command::Batch(p)).unwrap();
    assert!(out.changed && out.effects.contains(&Effect::Saved));
    assert_eq!(e.revision, guard.revision + 1);
    assert_eq!(e.undo.len(), 1);
    assert!(!e.is_dirty());
    assert_eq!(out.value.as_ref().unwrap()["saved"], path.to_str().unwrap());
    let final_state = e.stored_document();
    assert!(crate::storage::read(&path).unwrap() == final_state);
    e.run(Command::Undo(Empty {})).unwrap();
    assert!(e.stored_document() == before);
    assert!(e.is_dirty());
    assert!(crate::storage::read(&path).unwrap() == final_state);
    e.run(Command::Redo(Empty {})).unwrap();
    assert!(e.stored_document() == final_state);
    assert!(!e.is_dirty());
}
#[test]
fn final_save_failures_do_not_publish_even_selection_and_leave_existing_files_intact() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("existing.typsto");
    let mut e = engine("Avant");
    crate::storage::save(&existing, &e.stored_document()).unwrap();
    let disk = std::fs::read(&existing).unwrap();
    let before = e.stored_document();
    let guard = e.target_guard();
    let selection = e.selection();
    for save in [
        BatchSave::Current {},
        BatchSave::As {
            path: existing.clone(),
            overwrite: false,
        },
        BatchSave::As {
            path: dir.path().join("missing/letter.typsto"),
            overwrite: true,
        },
    ] {
        let mut p = params(
            &e,
            vec![
                step("text.select", json!({"target":{"kind":"all"}})),
                step("text.insert", json!({"text":"Remplacement"})),
            ],
        );
        p.save = Some(save);
        assert!(e.run(Command::Batch(p)).is_err());
        assert!(e.stored_document() == before);
        assert_eq!(e.target_guard(), guard);
        assert_eq!(e.selection(), selection);
        assert!(e.undo.is_empty());
        assert!(e.file_path.is_none());
        assert_eq!(std::fs::read(&existing).unwrap(), disk);
    }
}
#[test]
fn imports_metadata_signature_and_model_form_one_autonomous_undoable_candidate() {
    let dir = tempfile::tempdir().unwrap();
    let external = dir.path().join("image.svg");
    std::fs::write(
        &external,
        include_bytes!("../../../tests/fixtures/signature.svg"),
    )
    .unwrap();
    let mut e = engine("Avant");
    let before = e.stored_document();
    let guard = e.target_guard();
    let mut second = guard;
    second.revision += 1;
    let mut third = guard;
    third.revision += 2;
    let template = e.template.clone() + "\n// Batch model\n";
    let mut p = params(
        &e,
        vec![
            step(
                "insert.image",
                json!({"image":{"kind":"path","path":external},"alt":"Signature graphique","caption":"Légende"}),
            ),
            step(
                "document.set_signature",
                json!({"guard":second,"image":image()}),
            ),
            step(
                "document.set_model",
                json!({"guard":third,"model":{"kind":"source","source":template}}),
            ),
        ],
    );
    let output = dir.path().join("copy.typsto");
    p.save = Some(BatchSave::As {
        path: output.clone(),
        overwrite: false,
    });
    let out = e.run_call(Command::Batch(p).to_call()).unwrap();
    assert!(out.value.as_ref().unwrap()["presentation"].is_object());
    assert_eq!(e.revision, guard.revision + 1);
    assert_eq!(e.undo.len(), 1);
    let final_state = e.stored_document();
    assert_eq!(final_state.assets.len(), 2);
    std::fs::remove_file(external).unwrap();
    assert!(crate::storage::read(&output).unwrap() == final_state);
    let mut world = crate::rendering::TypstWorld::default();
    world.set_dependencies(&final_state.dependencies);
    world.set_assets(&final_state.assets);
    world
        .compile_with_template(
            &final_state.source,
            &final_state.metadata,
            &final_state.template,
        )
        .unwrap();
    e.run(Command::Undo(Empty {})).unwrap();
    assert!(e.stored_document() == before);
    e.run(Command::Redo(Empty {})).unwrap();
    assert!(e.stored_document() == final_state);
}
#[test]
fn batch_copies_personal_spelling_configuration_without_modifying_it_or_global_services() {
    let mut e = engine("courrrier");
    let mut words = crate::spelling::WordLists::new();
    words
        .entry("fr".into())
        .or_default()
        .insert("courrrier".into());
    e.configure_spelling(&words);
    let guard = e.target_guard();
    let error = e
        .run(Command::Batch(params(
            &e,
            vec![step("spelling.next", json!({}))],
        )))
        .unwrap_err();
    assert_eq!(
        error.params["error"]["params"]["reason"],
        "spelling_no_issues"
    );
    assert_eq!(e.target_guard(), guard);
    assert!(
        e.query(Query::SpellingIssues(Empty {})).unwrap()["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let p = params(
        &e,
        vec![step(
            "metadata.set",
            json!({"guard":guard,"field":MetadataField::LetterSubject,"value":"Courrier relu"}),
        )],
    );
    e.run(Command::Batch(p)).unwrap();
    assert_eq!(e.metadata.courrier().objet, "Courrier relu");
    assert!(
        e.query(Query::SpellingIssues(Empty {})).unwrap()["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        e.query(Query::RecoveryList(Empty {})).unwrap()["configured"],
        false
    );
}
