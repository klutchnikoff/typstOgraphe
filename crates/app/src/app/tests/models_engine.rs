use super::annotations_engine::{equivalent, pair};
use super::*;
use crate::engine::{Command as Core, Empty, Engine, ModelInput, Query};
use serde_json::{Value, json};

const CUSTOM: &str = "#let editor-options = (custom-bullet-marker: true,)\n#let courrier(..args) = args.pos().last()";
fn run(engine: &mut Engine, id: &str, params: Value) {
    engine
        .run_call(Core::from_call(id, params).unwrap().to_call())
        .unwrap();
}
fn presentation(engine: &Engine) -> ModelPresentation {
    serde_json::from_value(
        engine
            .query_call(Query::ModelPresentation(Empty {}).to_call())
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn source_install_preset_and_undo_redo_match_json_and_refresh_model_policy() {
    let (_ctx, mut app, mut engine) = pair("Un *été* et une _idée_.", 3..6);
    let preferences = app.preferences.clone();
    app.install_model(CUSTOM.into()).unwrap();
    let guard = engine.target_guard();
    run(
        &mut engine,
        "document.set_model",
        json!({"guard":guard,"model":{"kind":"source","source":CUSTOM}}),
    );
    equivalent(&app, &engine);
    assert_eq!(app.document.presentation, presentation(&engine));
    for option in [
        crate::engine::DocumentOption::Paper(PaperFormat::A3),
        crate::engine::DocumentOption::Landscape(true),
        crate::engine::DocumentOption::BulletMarker(Some("◆".into())),
    ] {
        app.set_document_option(option.clone());
        let guard = engine.target_guard();
        run(
            &mut engine,
            "document.set_option",
            json!({"guard":guard,"option":option}),
        );
    }
    let original_selection = app.document.selection;
    app.apply(Command::Model(Model::Personal));
    let guard = engine.target_guard();
    run(
        &mut engine,
        "courrier.model.personal",
        json!({"guard":guard}),
    );
    equivalent(&app, &engine);
    assert_eq!(app.document.presentation, presentation(&engine));
    assert_eq!(app.document.selection, original_selection);
    assert_eq!(app.preferences, preferences);
    for (ui, id) in [(Command::Undo, "edit.undo"), (Command::Redo, "edit.redo")] {
        app.apply(ui);
        run(&mut engine, id, json!({}));
        equivalent(&app, &engine);
        assert_eq!(app.document.presentation, presentation(&engine));
    }
    let history = engine.history().undo;
    app.apply(Command::Model(Model::Personal));
    let guard = engine.target_guard();
    run(
        &mut engine,
        "courrier.model.personal",
        json!({"guard":guard}),
    );
    equivalent(&app, &engine);
    assert_eq!(engine.history().undo, history);
}

#[test]
fn file_import_uses_the_same_command_as_json_and_a_stale_dialog_never_reads_its_file() {
    let (_ctx, mut app, mut engine) = pair("Bonjour.", 0..0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("modèle.typ");
    std::fs::write(&path, CUSTOM).unwrap();
    let app_guard = app.document.engine.target_guard();
    let engine_guard = engine.target_guard();
    app.install_model_input(app_guard, ModelInput::Path { path: path.clone() })
        .unwrap();
    run(
        &mut engine,
        "document.set_model",
        json!({"guard":engine_guard,"model":{"kind":"path","path":path}}),
    );
    equivalent(&app, &engine);
    assert_eq!(app.document.presentation, presentation(&engine));
    std::fs::remove_file(&path).unwrap();
    let captured = app.document.engine.target_guard();
    let stale = engine.target_guard();
    let core = Core::from_call("text.insert", json!({"text":"Une suite. "})).unwrap();
    app.execute_document_command(core.clone()).unwrap();
    engine.run_call(core.to_call()).unwrap();
    let before = app.stored_document();
    assert!(
        app.install_model_input(captured, ModelInput::Path { path: path.clone() })
            .is_err()
    );
    let error = engine
        .run_call(
            Core::from_call(
                "document.set_model",
                json!({"guard":stale,"model":{"kind":"path","path":path}}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap_err();
    assert_eq!(error.reason(), "stale_revision");
    assert!(app.stored_document() == before);
    equivalent(&app, &engine);
}

#[test]
fn invalid_import_keeps_presentation_and_published_state_then_a_valid_import_recovers() {
    let (_ctx, mut app, mut engine) = pair("Bonjour.", 0..3);
    let before = app.stored_document();
    let policy = app.document.presentation.clone();
    let source = "#let compte-rendu(body) = body";
    let guard = engine.target_guard();
    let error = engine
        .run_call(
            Core::from_call(
                "document.set_model",
                json!({"guard":guard,"model":{"kind":"source","source":source}}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap_err();
    assert_eq!(
        app.install_model(source.into()).unwrap_err(),
        App::command_error_text(&error)
    );
    assert!(app.stored_document() == before);
    assert_eq!(app.document.presentation, policy);
    equivalent(&app, &engine);
    app.install_model(CUSTOM.into()).unwrap();
    let guard = engine.target_guard();
    run(
        &mut engine,
        "document.set_model",
        json!({"guard":guard,"model":{"kind":"source","source":CUSTOM}}),
    );
    equivalent(&app, &engine);
    for kind in DocumentType::ALL {
        for model in kind.models() {
            assert!(
                Core::from_call(model.model.id(), json!({"guard":engine.target_guard()})).is_ok()
            );
        }
    }
}
