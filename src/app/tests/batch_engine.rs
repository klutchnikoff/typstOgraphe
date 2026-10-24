use super::*;
use crate::engine::{self, BatchParams, BatchSave, Call, Command as Core};
use serde_json::json;

fn step(id: &str, params: serde_json::Value) -> Call {
    Call {
        id: id.to_owned().try_into().unwrap(),
        params,
    }
}
fn params(app: &App, steps: Vec<Call>) -> BatchParams {
    BatchParams {
        guard: app.document.engine.target_guard(),
        steps,
        save: None,
    }
}

#[test]
fn batch_adapter_matches_json_execution_and_restores_the_whole_transaction_with_undo() {
    let (_ctx, mut app) = demo_app();
    app.document.selection = editor::cursor(0..5);
    app.sync_published_selection();
    let stored = app.stored_document();
    let selection = app.document.selection;
    let mut core: engine::Engine =
        crate::engine::Engine::from_stored(stored.clone(), None).unwrap();
    core.set_adapter_selection(editor::to_core(selection))
        .unwrap();
    let steps = |guard| {
        vec![
            step(
                "metadata.set",
                json!({"guard":guard,"field":"letter.subject","value":"Lettre relue"}),
            ),
            step("text.italic", json!({"target":{"kind":"all"},"value":"on"})),
        ]
    };
    let p = params(&app, steps(app.document.engine.target_guard()));
    let app_outcome = app.execute_document_command(Core::Batch(p)).unwrap();
    let core_outcome = core
        .run_call(
            Core::Batch(BatchParams {
                guard: core.target_guard(),
                steps: steps(core.target_guard()),
                save: None,
            })
            .to_call(),
        )
        .unwrap();
    assert!(app.stored_document() == core.stored_document());
    assert_eq!(app_outcome.changed, core_outcome.changed);
    assert_eq!(app_outcome.value, core_outcome.value);
    assert_eq!(app.document.engine.history().undo, 1);
    let core_selection: CCursorRange = editor::to_editor(core.selection());
    assert_eq!(app.document.selection, core_selection);
    let final_state = app.stored_document();
    app.apply(Command::Undo);
    assert!(app.stored_document() == stored);
    assert_eq!(app.document.selection, selection);
    app.apply(Command::Redo);
    assert!(app.stored_document() == final_state);
}
#[test]
fn saved_batch_clears_recovery_only_on_success_and_keeps_preview_synchronization() {
    let (ctx, mut app) = demo_app();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("recovery");
    seed_recovery(&root, &app.stored_document(), None);
    app.start_recovery(root.clone(), ctx);
    app.recover_document(0);
    assert!(app.document.engine.is_recovered());
    app.recovery.worker.as_mut().unwrap().make_due();
    app.checkpoint_recovery();
    app.recovery.worker.as_mut().unwrap().wait();
    let prefs = app.preferences.clone();
    let guard = app.document.engine.target_guard();
    let before = app.stored_document();
    let steps = vec![step("text.insert", json!({"text":"Ajout au courrier. "}))];
    let mut p = params(&app, steps.clone());
    p.save = Some(BatchSave::As {
        path: dir.path().join("missing/file.typsto"),
        overwrite: false,
    });
    assert!(app.execute_document_command(Core::Batch(p)).is_err());
    assert!(app.stored_document() == before);
    assert_eq!(app.document.engine.target_guard(), guard);
    assert!(app.document.engine.is_recovered());
    assert!(
        std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.path().join("document.recovery").exists())
    );
    let output = dir.path().join("copy.typsto");
    let mut p = params(&app, steps);
    p.save = Some(BatchSave::As {
        path: output.clone(),
        overwrite: false,
    });
    app.preview.forward_pending = false;
    app.document.force_selection = false;
    let outcome = app.execute_document_command(Core::Batch(p)).unwrap();
    app.recovery.worker.as_mut().unwrap().wait();
    assert!(outcome.effects.contains(&engine::Effect::Saved));
    assert!(!app.document.engine.is_recovered() && !app.is_dirty());
    assert!(app.preview.forward_pending && app.document.force_selection);
    assert_eq!(app.preferences, prefs);
    assert!(storage::read(&output).unwrap() == app.stored_document());
    assert_eq!(app.document.engine.history().undo, 1);
    drop(app);
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
}
#[test]
fn model_batch_refreshes_editor_policy_immediately_and_metadata_only_batches_keep_focus() {
    let (_ctx, mut app) = demo_app();
    let original = app.stored_document();
    let selected = app.document.selection;
    let source = "#let editor-options = (custom-bullet-marker: true,)\n#let courrier(..args) = args.pos().last()";
    let guard = app.document.engine.target_guard();
    let p = params(
        &app,
        vec![step(
            "document.set_model",
            json!({"guard":guard,"model":{"kind":"source","source":source}}),
        )],
    );
    app.document.force_selection = false;
    app.execute_document_command(Core::Batch(p)).unwrap();
    assert!(app.document.presentation.custom_bullet_marker);
    assert!(!app.document.force_selection);
    assert_eq!(app.document.selection, selected);
    app.apply(Command::Undo);
    assert!(app.stored_document() == original);
    assert!(!app.document.presentation.custom_bullet_marker);
    let guard = app.document.engine.target_guard();
    let p = params(
        &app,
        vec![step(
            "metadata.set",
            json!({"guard":guard,"field":"letter.subject","value":"Objet modifié"}),
        )],
    );
    app.document.force_selection = false;
    app.execute_document_command(Core::Batch(p)).unwrap();
    assert!(!app.document.force_selection);
    assert_eq!(app.document.selection, selected);
    assert_eq!(
        app.document.engine.metadata().letter().objet,
        "Objet modifié"
    );
}
#[test]
fn batch_errors_are_localized_and_do_not_change_editor_content_selection_or_focus() {
    let (_ctx, mut app) = demo_app();
    let before = app.stored_document();
    let selection = app.document.selection;
    let guard = app.document.engine.target_guard();
    let p = params(
        &app,
        vec![
            step("text.insert", json!({"text":"Brouillon temporaire"})),
            step("text.link", json!({"url":"invalid"})),
        ],
    );
    app.document.force_selection = false;
    assert!(!app.run_document_command(Core::Batch(p)));
    assert!(app.stored_document() == before);
    assert_eq!(app.document.selection, selection);
    assert_eq!(app.document.engine.target_guard(), guard);
    assert!(!app.document.force_selection);
    assert!(!app.document.engine.can_undo());
    assert!(
        app.dialogs
            .message
            .as_ref()
            .unwrap()
            .replace(['\u{2068}', '\u{2069}'], "")
            .starts_with("Étape 2 :")
    );
}
