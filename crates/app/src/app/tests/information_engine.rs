use super::annotations_engine::{equivalent, pair};
use super::*;
use crate::engine::{Command as Core, Engine};
use serde_json::{Value, json};

fn run(engine: &mut Engine, id: &str, params: Value) {
    engine
        .run_call(Core::from_call(id, params).unwrap().to_call())
        .unwrap();
}
fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        modifiers,
        physical_key: None,
        pressed: true,
        repeat: false,
    }
}

#[test]
fn native_metadata_typing_matches_json_grouping_and_keeps_field_focus_body_and_preferences() {
    let (ctx, mut app, mut engine) = pair("Un *été* à _Noël_.", 3..6);
    app.dialogs.information_open = true;
    app.dialogs.information_group = InformationGroup::Fields("information");
    let preferences = app.preferences.clone();
    let selection = app.document.selection;
    let subject = app.document.engine.metadata().letter().objet.clone();
    recovery_click(&mut app, &ctx, &subject);
    for (events, value) in [
        (
            vec![
                key(egui::Key::A, egui::Modifiers::COMMAND),
                egui::Event::Text("A".into()),
            ],
            "A",
        ),
        (vec![egui::Event::Text("B".into())], "AB"),
    ] {
        let guard = engine.target_guard();
        run(
            &mut engine,
            "metadata.set",
            json!({"guard":guard,"field":"letter.subject","value":value,"input":"typing"}),
        );
        let _ = search_draw(&mut app, &ctx, events);
        equivalent(&app, &engine);
        let id = app
            .document
            .history_field
            .expect("champ de métadonnées au focus");
        assert!(ctx.memory(|m| m.has_focus(id)));
        assert_ne!(id, egui::Id::new(editor::EDITOR_ID));
    }
    assert_eq!(engine.history().undo, 1);
    assert_eq!(app.document.selection, selection);
    assert_eq!(app.preferences, preferences);
    for (ui, id) in [(Command::Undo, "edit.undo"), (Command::Redo, "edit.redo")] {
        app.apply(ui);
        run(&mut engine, id, json!({}));
        equivalent(&app, &engine);
        let _ = search_draw(&mut app, &ctx, vec![]);
        equivalent(&app, &engine);
    }
}

#[test]
fn today_button_publishes_the_same_explicit_fixed_date_as_json_without_changing_text() {
    let (ctx, mut app, mut engine) = pair("Bonjour.", 2..5);
    app.dialogs.information_open = true;
    app.dialogs.information_group = InformationGroup::Fields("information");
    recovery_click(&mut app, &ctx, &crate::tr!("date-today"));
    let fixed = app.document.engine.metadata().letter().date.clone();
    let guard = engine.target_guard();
    run(
        &mut engine,
        "metadata.set",
        json!({"guard":guard,"field":"letter.date","value":fixed}),
    );
    equivalent(&app, &engine);
    assert_eq!(engine.document().source(), "Bonjour.");
    for (ui, id) in [(Command::Undo, "edit.undo"), (Command::Redo, "edit.redo")] {
        app.apply(ui);
        run(&mut engine, id, json!({}));
        equivalent(&app, &engine);
    }
}

#[test]
fn custom_marker_drafts_remain_local_until_valid_then_match_guarded_json_and_undo() {
    let (ctx, mut app, _) = pair("- Bonjour.", 0..0);
    let template = format!(
        "{}\n#let editor-options = (custom-bullet-marker: true,)",
        app.document.engine.template()
    );
    app.install_model(template).unwrap();
    app.document.engine = Engine::from_stored(app.stored_document(), None).unwrap();
    app.reset_view_sync();
    let mut engine: Engine =
        crate::engine::Engine::from_stored(app.stored_document(), None).unwrap();
    engine
        .set_adapter_selection(editor::to_core(app.document.selection))
        .unwrap();
    app.dialogs.information_open = true;
    app.dialogs.information_group = InformationGroup::General;
    recovery_click(&mut app, &ctx, &crate::tr!("bullet-marker-automatic"));
    recovery_click(&mut app, &ctx, &crate::tr!("bullet-marker-custom"));
    let guard = engine.target_guard();
    run(
        &mut engine,
        "document.set_option",
        json!({"guard":guard,"option":{"key":"bullet_marker","value":"◆"}}),
    );
    equivalent(&app, &engine);
    let output = search_draw(&mut app, &ctx, vec![]);
    let point = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "◆" => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .next_back()
        .expect("champ du caractère personnalisé");
    for pressed in [true, false] {
        let _ = search_draw(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let _ = search_draw(
        &mut app,
        &ctx,
        vec![
            key(egui::Key::A, egui::Modifiers::COMMAND),
            egui::Event::Text("ab".into()),
        ],
    );
    let _ = search_draw(&mut app, &ctx, vec![]);
    equivalent(&app, &engine);
    assert_eq!(app.dialogs.marker_draft.as_ref().unwrap().1, "ab");
    let _ = search_draw(
        &mut app,
        &ctx,
        vec![
            key(egui::Key::A, egui::Modifiers::COMMAND),
            egui::Event::Text("▪".into()),
        ],
    );
    let guard = engine.target_guard();
    run(
        &mut engine,
        "document.set_option",
        json!({"guard":guard,"option":{"key":"bullet_marker","value":"▪"}}),
    );
    equivalent(&app, &engine);
    app.apply(Command::Undo);
    run(&mut engine, "edit.undo", json!({}));
    equivalent(&app, &engine);
}
