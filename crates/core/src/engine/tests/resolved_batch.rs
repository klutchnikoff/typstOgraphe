use super::*;

fn params(engine: &Engine, calls: &[(&str, Value)]) -> BatchParams {
    BatchParams {
        guard: engine.target_guard(),
        steps: calls
            .iter()
            .map(|(id, params)| Call {
                id: (*id).to_owned().try_into().unwrap(),
                params: params.clone(),
            })
            .collect(),
        save: None,
    }
}
fn resolve(_: usize, engine: &Engine, call: &Call) -> Result<ResolvedBatchStep, CommandError> {
    let mut params = call.params.clone();
    if call.id.as_str() == "metadata.set" {
        params["guard"] = json!(engine.target_guard());
    }
    Ok(ResolvedBatchStep {
        command: Command::from_call(call.id.as_str(), params)?,
        selection: None,
    })
}

#[test]
fn resolutions_use_the_evolving_private_document_and_publish_one_undoable_transaction() {
    let mut engine = engine("Avant");
    let before = engine.stored_document();
    let guard = engine.target_guard();
    let p = params(
        &engine,
        &[
            ("text.insert", json!({"text":"Élodie "})),
            ("text.bold", json!({})),
            (
                "metadata.set",
                json!({"field":"letter.subject","value":"Réunion"}),
            ),
        ],
    );
    let out = engine
        .run_resolved_batch(p, |index, draft, call| {
            if index == 1 {
                assert!(draft.document().source().starts_with("Élodie"));
                let selection = draft.selection_target(Selection {
                    secondary: Caret::default(),
                    primary: Caret {
                        index: 6,
                        ..Default::default()
                    },
                })?;
                return Ok(ResolvedBatchStep {
                    command: Command::Bold(StyleParams::default()),
                    selection: Some(selection),
                });
            }
            resolve(index, draft, call)
        })
        .unwrap();
    assert_eq!(out.guard.revision, guard.revision + 1);
    assert_eq!(engine.history().undo, 1);
    assert!(engine.document().view.styles[..6].iter().all(|s| s.bold));
    assert_eq!(engine.metadata().courrier().objet, "Réunion");
    let final_state = engine.stored_document();
    engine.run(Command::Undo(Empty {})).unwrap();
    assert_eq!(engine.stored_document(), before);
    engine.run(Command::Redo(Empty {})).unwrap();
    assert_eq!(engine.stored_document(), final_state);
}

#[test]
fn late_resolution_or_final_save_failure_preserves_session_file_history_and_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let mut engine = engine("Avant");
    engine
        .run(Command::SaveDocument(SaveDocumentParams {
            guard: engine.target_guard(),
            path: Some(path.clone()),
        }))
        .unwrap();
    let before = engine.stored_document();
    let guard = engine.target_guard();
    let bytes = std::fs::read(&path).unwrap();
    let p = params(
        &engine,
        &[
            ("text.insert", json!({"text":"Temporaire "})),
            ("text.bold", json!({})),
        ],
    );
    let error = engine
        .run_resolved_batch(p, |index, draft, call| {
            if index == 1 {
                return Err(CommandError::new(ErrorCode::NotFound, "unknown_block"));
            }
            resolve(index, draft, call)
        })
        .unwrap_err();
    assert_eq!(error.params["step"], 1);
    let mut p = params(&engine, &[("text.insert", json!({"text":"Non "}))]);
    p.save = Some(BatchSave::As {
        path: path.clone(),
        overwrite: false,
    });
    assert_eq!(
        engine.run_resolved_batch(p, resolve).unwrap_err().reason(),
        "file_exists"
    );
    assert_eq!(engine.target_guard(), guard);
    assert_eq!(engine.stored_document(), before);
    assert_eq!(engine.history().undo, 0);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut other = Engine::empty();
    assert_eq!(
        other
            .run(Command::OpenDocument(OpenDocumentParams {
                guard: other.target_guard(),
                path,
                read_only: false,
                discard_changes: false
            }))
            .unwrap_err()
            .reason(),
        "file_locked"
    );
}

#[test]
fn forbidden_ids_are_preflighted_and_resolvers_cannot_substitute_side_effects_or_unbounded_payloads()
 {
    let mut engine = engine("Avant");
    let before = engine.stored_document();
    let guard = engine.target_guard();
    let p = params(
        &engine,
        &[
            ("text.insert", json!({"text":"Non"})),
            ("document.export", json!({})),
        ],
    );
    let error = engine
        .run_resolved_batch(p, |_, _, _| panic!("forbidden batch must not resolve"))
        .unwrap_err();
    assert_eq!(
        error.params["error"]["params"]["reason"],
        "batch_step_forbidden"
    );
    let p = params(&engine, &[("text.insert", json!({}))]);
    let error = engine
        .run_resolved_batch(p, |_, _, _| {
            Ok(ResolvedBatchStep {
                command: Command::CloseDocument(CloseDocumentParams {
                    guard,
                    discard_changes: true,
                }),
                selection: None,
            })
        })
        .unwrap_err();
    assert_eq!(
        error.params["error"]["params"]["reason"],
        "batch_step_forbidden"
    );
    let p = params(&engine, &[("text.insert", json!({}))]);
    let error = engine
        .run_resolved_batch(p, |_, _, _| {
            Ok(ResolvedBatchStep {
                command: Command::InsertText(InsertTextParams {
                    text: "x".repeat(MAX_BATCH_BYTES),
                    target: TextTarget::default(),
                    input_rules: InputRules::Literal,
                }),
                selection: None,
            })
        })
        .unwrap_err();
    assert_eq!(
        error.params["error"]["params"]["reason"],
        "batch_size_limit"
    );
    assert_eq!(engine.target_guard(), guard);
    assert_eq!(engine.stored_document(), before);
}
