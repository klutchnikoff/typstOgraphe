use super::super::{
    bands::{BandContent, EditorAction},
    contextual::ContextualAction,
};
use super::*;

fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn invoke() -> egui::Event {
    key(egui::Key::Enter, egui::Modifiers::CTRL)
}
fn setup(source: &str, needle: &str) -> (egui::Context, App) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source(source).unwrap()
    });
    let text = &app.document.engine.document().view.text;
    let at = text[..text.find(needle).unwrap()].chars().count();
    app.document.selection = editor::cursor(at..at);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    (ctx, app)
}
fn settle(app: &mut App, ctx: &egui::Context) {
    for _ in 0..4 {
        frame(app, ctx, vec![]);
    }
}

#[test]
fn a_spelling_issue_takes_priority_over_the_surrounding_widget_without_editing_it() {
    let (ctx, mut app) = setup("- Un courrrier\n- Une phrase.", "courrrier");
    let before = app.stored_document();
    let guard = app.document.engine.target_guard();
    assert!(matches!(
        app.contextual_action(),
        Some(ContextualAction::Correct(_))
    ));
    assert!(
        app.ui_command_label(Command::ContextualAction)
            .contains("courrrier")
    );
    frame(&mut app, &ctx, vec![invoke()]);
    settle(&mut app, &ctx);
    assert!(matches!(
        app.bands.open.as_ref().unwrap().content,
        BandContent::Spelling { .. }
    ));
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.engine.target_guard(), guard);
    assert_eq!(app.document.engine.history().undo, 0);
}

#[test]
fn contextual_capture_keeps_the_suggestions_request_across_the_first_frame() {
    let (ctx, mut app) = demo_app();
    app.capture_widgets(&ctx, "context-spelling");
    let before = app.stored_document();
    app.spelling.checker.wait();
    settle(&mut app, &ctx);
    assert!(app.spelling.checker.ready);
    assert!(
        app.spelling
            .checker
            .suggestions
            .as_ref()
            .and_then(|(_, s)| s.as_ref())
            .is_some_and(|s| !s.is_empty())
    );
    assert!(matches!(
        app.bands.open.as_ref().unwrap().content,
        BandContent::Spelling { .. }
    ));
    assert!(!app.bands.open.as_ref().unwrap().focus_pending);
    assert_eq!(app.stored_document(), before);
}

#[test]
fn selection_uses_its_active_end_and_resolves_a_fully_selected_issue() {
    let (ctx, mut app) = setup("Un courrrier.\n\n- Bonjour", "Bonjour");
    app.document.selection = editor::cursor(4..app.document.engine.document().view.len());
    assert!(
        app.contextual_action().is_none(),
        "the active end is outside the misspelled word"
    );
    app.document.selection = editor::cursor(3..12);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    assert!(matches!(
        app.contextual_action(),
        Some(ContextualAction::Correct(_))
    ));
}

#[test]
fn lists_and_image_captions_open_their_settings_and_repeat_only_refocuses_the_draft() {
    for (source, needle) in [
        ("- Bonjour\n- Merci", "Merci"),
        (
            "#figure(image(\"images/parc.svg\"), caption: [Le parc])",
            "parc",
        ),
    ] {
        let (ctx, mut app) = setup(source, needle);
        app.preferences.spelling.enabled = false;
        let before = app.stored_document();
        let guard = app.document.engine.target_guard();
        frame(&mut app, &ctx, vec![invoke()]);
        settle(&mut app, &ctx);
        let target = app.bands.open.as_ref().unwrap().target;
        match &mut app.bands.open.as_mut().unwrap().content {
            BandContent::List { kind } => *kind = BlockKind::Numbered,
            BandContent::Image { alt, .. } => *alt = "Brouillon privé".into(),
            _ => panic!("widget settings"),
        }
        frame(&mut app, &ctx, vec![invoke()]);
        settle(&mut app, &ctx);
        let band = app.bands.open.as_ref().unwrap();
        assert_eq!(band.target, target);
        match &band.content {
            BandContent::List { kind } => assert_eq!(*kind, BlockKind::Numbered),
            BandContent::Image { alt, .. } => {
                assert_eq!(alt, "Brouillon privé");
                assert!(ctx.memory(|m| m.has_focus(egui::Id::new("editor-band-input"))))
            }
            _ => unreachable!(),
        }
        assert_eq!(app.stored_document(), before);
        assert_eq!(app.document.engine.target_guard(), guard);
        assert_eq!(app.document.engine.history().undo, 0);
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(app.bands.open.is_none());
        assert_eq!(app.stored_document(), before);
    }
}

#[test]
fn no_context_does_nothing_including_no_newline_or_global_preferences() {
    for source in [
        "Bonjour.",
        "#quote(block: true)[Bonjour.]",
        "#postscript[Bonjour.]",
        "#attachment[Bonjour.]",
    ] {
        let (ctx, mut app) = setup(source, "Bonjour");
        app.preferences.spelling.enabled = false;
        let before = app.stored_document();
        assert!(!app.command_enabled(Command::ContextualAction));
        frame(&mut app, &ctx, vec![invoke()]);
        assert!(app.bands.open.is_none());
        assert!(!app.dialogs.preferences_open);
        assert_eq!(app.stored_document(), before);
        assert_eq!(app.document.engine.history().undo, 0);
    }
}

#[test]
fn read_only_documents_offer_no_contextual_mutation() {
    for source in ["- Bonjour", "- Un courrrier"] {
        let (ctx, mut app) = setup(
            source,
            if source.contains("courrrier") {
                "courrrier"
            } else {
                "Bonjour"
            },
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("readonly.typsto");
        crate::storage::save(&path, &app.stored_document()).unwrap();
        assert!(app.open_path_with_access(path, true));
        let at = if source.contains("courrrier") { 4 } else { 1 };
        app.document.selection = editor::cursor(at..at);
        app.document.force_selection = true;
        settle(&mut app, &ctx);
        let before = app.stored_document();
        assert!(!app.command_enabled(Command::ContextualAction));
        frame(&mut app, &ctx, vec![invoke()]);
        assert!(app.bands.open.is_none());
        assert_eq!(app.stored_document(), before);
        assert_eq!(app.document.engine.history().undo, 0);
    }
}

#[test]
fn an_open_confirmation_is_refocused_without_confirming_or_removing_anything() {
    let (ctx, mut app) = setup("- Bonjour\n- Merci", "Merci");
    let widget = editor::widgets(&app.document.engine.document().view).remove(0);
    app.dispatch_editor_action(EditorAction::Remove(widget), &ctx);
    settle(&mut app, &ctx);
    let before = app.stored_document();
    for _ in 0..2 {
        frame(&mut app, &ctx, vec![invoke()]);
        settle(&mut app, &ctx);
        assert!(matches!(
            app.bands.open.as_ref().unwrap().content,
            BandContent::Remove { .. }
        ));
        assert_eq!(app.stored_document(), before);
        assert_eq!(app.document.engine.history().undo, 0);
    }
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.bands.open.is_none());
    assert!(editor::widgets(&app.document.engine.document().view).is_empty());
    assert_eq!(app.document.engine.history().undo, 1);
}

#[test]
fn palette_and_custom_binding_use_the_same_resolution_and_show_the_specific_action() {
    let (ctx, mut app) = setup("- Bonjour\n- Merci", "Merci");
    app.preferences.spelling.enabled = false;
    app.apply(Command::Palette);
    app.dialogs.palette_query = "contextuelle".into();
    let _ = search_draw(&mut app, &ctx, vec![]);
    let output = search_draw(&mut app, &ctx, vec![]);
    assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == crate::tr!("widget-list-settings"))));
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(matches!(
        app.bands.open.as_ref().unwrap().content,
        BandContent::List { .. }
    ));
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    let overrides =
        serde_json::from_value(serde_json::json!({"edit.contextual_action": [{"key": "F6"}]}))
            .unwrap();
    app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &overrides).unwrap();
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::F6, egui::Modifiers::NONE)],
    );
    assert!(matches!(
        app.bands.open.as_ref().unwrap().content,
        BandContent::List { .. }
    ));
    assert_eq!(app.document.engine.history().undo, 0);
}

#[test]
fn body_ime_parameter_ime_modal_and_search_keep_their_keyboard_context() {
    let (ctx, mut app) = setup("- Bonjour", "Bonjour");
    app.preferences.spelling.enabled = false;
    let before = app.stored_document();
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Enabled)],
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("にほん".into()))],
    );
    frame(&mut app, &ctx, vec![invoke()]);
    assert!(app.bands.open.is_none());
    assert_eq!(app.stored_document(), before);
    assert!(app.document.ime.draft.is_some());
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    app.apply(Command::Find);
    settle(&mut app, &ctx);
    frame(&mut app, &ctx, vec![invoke()]);
    assert!(app.bands.open.is_none());
    app.close_search();
    app.dialogs.preferences_open = true;
    frame(&mut app, &ctx, vec![invoke()]);
    assert!(app.bands.open.is_none());
    app.dialogs.preferences_open = false;
    app.document.selection = editor::cursor(0..7);
    app.document.force_selection = true;
    settle(&mut app, &ctx);
    app.open_link();
    settle(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Enabled)],
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("にほん".into()))],
    );
    frame(&mut app, &ctx, vec![invoke()]);
    assert!(matches!(
        app.bands.open.as_ref().unwrap().content,
        BandContent::Link { .. }
    ));
    assert!(!app.bands.open.as_ref().unwrap().focus_pending);
    assert_eq!(app.stored_document(), before);
}
