use super::*;
use crate::engine::{Command as Core, Empty, Engine};
use serde_json::{Value, json};

pub(super) fn pair(source: &str, range: Range<usize>) -> (egui::Context, App, Engine) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source(source).unwrap();
    });
    app.document.selection = editor::cursor(range);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    let mut engine: Engine = Session::new(
        app.stored_document(),
        app.document.engine.document().clone(),
        None,
        app.document.engine.revision(),
    )
    .into();
    engine
        .set_adapter_selection(app.document.selection.into())
        .unwrap();
    (ctx, app, engine)
}

pub(super) fn equivalent(app: &App, engine: &Engine) {
    assert!(app.stored_document() == engine.stored_document());
    assert_eq!(app.document.engine.document().view, engine.document().view);
    assert_eq!(
        app.document.selection,
        CCursorRange::from(engine.selection())
    );
    assert_eq!(app.document.engine.revision(), engine.revision());
    assert_eq!(app.document.engine.undo().len(), engine.undo().len());
    assert_eq!(app.document.engine.redo().len(), engine.redo().len());
}

pub(super) fn gesture(
    ctx: &egui::Context,
    app: &mut App,
    engine: &mut Engine,
    key: egui::Key,
    modifiers: egui::Modifiers,
    id: &str,
    params: Value,
) {
    engine
        .run_call(Core::from_call(id, params).unwrap().to_call())
        .unwrap();
    frame(
        app,
        ctx,
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
    );
    equivalent(app, engine);
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
}

#[test]
fn tab_shift_tab_and_enter_match_json_commands_for_both_optional_annotations() {
    for source in [
        "#figure(image(\"images/parc.svg\"))\n\nSuite.",
        "#figure(image(\"images/parc.svg\"), caption: [Été *日本*])\n\nSuite.",
        "#quote(block: true)[Une pensée.]\n\nSuite.",
        "#quote(block: true, attribution: [Été *日本*])[Une pensée.]\n\nSuite.",
    ] {
        let annotation = Document::from_source(source)
            .unwrap()
            .view
            .annotations()
            .remove(0);
        let (ctx, mut app, mut engine) = pair(source, annotation.main_target());
        for part in ["annotation", "annotation", "body", "annotation"] {
            gesture(
                &ctx,
                &mut app,
                &mut engine,
                egui::Key::Tab,
                if part == "body" {
                    egui::Modifiers::SHIFT
                } else {
                    egui::Modifiers::NONE
                },
                "text.select",
                json!({"part":part}),
            );
        }
        assert!(engine.undo().is_empty());
        gesture(
            &ctx,
            &mut app,
            &mut engine,
            egui::Key::Enter,
            egui::Modifiers::NONE,
            "block.split",
            json!({}),
        );
        assert_eq!(engine.undo().len(), 1);
        app.apply(Command::Undo);
        engine.run(Core::Undo(Empty {})).unwrap();
        equivalent(&app, &engine);
        app.apply(Command::Redo);
        engine.run(Core::Redo(Empty {})).unwrap();
        equivalent(&app, &engine);
    }
}

#[test]
fn quote_enter_and_empty_line_exit_match_json_and_preserve_native_history() {
    let source = "#quote(block: true, attribution: [Élodie *日本*])[Une pensée.]";
    let (ctx, mut app, mut engine) = pair(source, 11..11);
    let original = engine.document().clone();
    for _ in 0..2 {
        gesture(
            &ctx,
            &mut app,
            &mut engine,
            egui::Key::Enter,
            egui::Modifiers::NONE,
            "block.split",
            json!({}),
        );
    }
    assert_eq!(engine.undo().len(), 2);
    for _ in 0..2 {
        app.apply(Command::Undo);
        engine.run(Core::Undo(Empty {})).unwrap();
        equivalent(&app, &engine);
    }
    assert_eq!(engine.document(), &original);
}

#[test]
fn enter_on_a_whole_quote_selection_matches_json_without_leaving_an_orphan_annotation() {
    let source = "#quote(block: true, attribution: [Élodie])[Une pensée.]";
    let range = Document::from_source(source).unwrap().view.annotations()[0]
        .structure
        .clone();
    let (ctx, mut app, mut engine) = pair(source, range);
    gesture(
        &ctx,
        &mut app,
        &mut engine,
        egui::Key::Enter,
        egui::Modifiers::NONE,
        "block.split",
        json!({}),
    );
    assert!(engine.document().view.annotations().is_empty());
    assert_eq!(engine.undo().len(), 1);
}
