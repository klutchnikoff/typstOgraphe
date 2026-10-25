use super::super::bands::{BandContent, EditorAction};
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
fn setup(source: &str, selected: &str) -> (egui::Context, App) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source(source).unwrap()
    });
    let text = &app.document.engine.document().view.text;
    let at = text[..text.find(selected).unwrap()].chars().count();
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
fn open_widget(app: &mut App, ctx: &egui::Context, remove: bool) {
    let widget = editor::widgets(&app.document.engine.document().view).remove(0);
    app.dispatch_editor_action(
        if remove {
            EditorAction::Remove(widget)
        } else {
            EditorAction::Settings(widget)
        },
        ctx,
    );
    settle(app, ctx);
}

#[test]
fn ctrl_backspace_removes_the_entire_widget_from_body_and_annotation_in_one_undo() {
    for (source, selected) in [
        ("Avant.\n\n- Un\n- Deux\n\nAprès.", "Deux"),
        ("Avant.\n\n+ Un\n+ Deux\n\nAprès.", "Deux"),
        (
            "Avant.\n\n/ Un: Définition\n/ Deux: Explication\n\nAprès.",
            "Explication",
        ),
        (
            "Avant.\n\n#quote(block: true, attribution: [Auteur])[Citation.]\n\nAprès.",
            "Auteur",
        ),
        (
            "Avant.\n\n#figure(image(\"images/parc.svg\"), caption: [Légende])\n\nAprès.",
            "Légende",
        ),
        (
            "Avant.\n\n#postscript[Une précision.]\n\nAprès.",
            "précision",
        ),
        (
            "Avant.\n\n#attachment[Formulaire]\n#attachment[Programme]\n\nAprès.",
            "Programme",
        ),
    ] {
        let (ctx, mut app) = setup(source, selected);
        let before = app.stored_document();
        assert!(app.command_enabled(Command::RemoveWidget), "{source}");
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Backspace, egui::Modifiers::CTRL)],
        );
        assert!(
            editor::widgets(&app.document.engine.document().view).is_empty(),
            "{source}"
        );
        assert_eq!(app.document.engine.history().undo, 1, "{source}");
        assert!(app.document.engine.document().view.text.contains("Avant."));
        assert!(app.document.engine.document().view.text.contains("Après."));
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), before, "{source}");
    }
}

#[test]
fn widget_removal_binding_is_customizable_and_preserves_native_word_deletion_elsewhere() {
    let (ctx, mut app) = setup("Premier deuxième", "deuxième");
    let end = app.document.engine.document().view.len();
    app.document.selection = editor::cursor(end..end);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    assert!(!app.command_enabled(Command::RemoveWidget));
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Backspace, egui::Modifiers::CTRL)],
    );
    assert_eq!(app.document.engine.document().view.text, "Premier ");

    let (ctx, mut app) = setup("- Un\n- Deux", "Deux");
    let overrides = serde_json::from_value(serde_json::json!({
        "edit.remove_widget": [{"key": "F6"}]
    }))
    .unwrap();
    app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &overrides).unwrap();
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::F6, egui::Modifiers::NONE)],
    );
    assert!(editor::widgets(&app.document.engine.document().view).is_empty());
    assert_eq!(app.document.engine.history().undo, 1);
}

#[test]
fn return_applies_the_last_native_parameter_input_without_inserting_a_document_newline() {
    let (ctx, mut app) = setup("Une phrase.", "Une");
    app.document.selection = editor::cursor(0..3);
    app.open_link();
    settle(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Text("https://example.org".into()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    assert!(app.bands.open.is_none());
    assert_eq!(app.document.engine.document().view.text, "Une phrase.");
    assert!(
        app.document.engine.document().view.styles[..3]
            .iter()
            .all(|s| s.link.as_deref() == Some("https://example.org"))
    );
    assert_eq!(app.document.engine.history().undo, 1);

    let (ctx, mut app) = setup(
        "#figure(image(\"images/parc.svg\"), caption: [Légende])",
        "Légende",
    );
    open_widget(&mut app, &ctx, false);
    let before = app.stored_document();
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Text("Vue du parc".into()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    assert!(app.bands.open.is_none());
    assert_eq!(
        app.document.engine.document().view.images[&1].alt,
        "Vue du parc"
    );
    assert!(app.document.engine.document().view.text.contains("Légende"));
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
}

#[test]
fn return_validates_list_parameters_and_deletion_while_escape_discards_either() {
    let (ctx, mut app) = setup("Avant.\n\n- Un\n- Deux\n\nAprès.", "Deux");
    let before = app.stored_document();
    open_widget(&mut app, &ctx, false);
    if let BandContent::List { kind } = &mut app.bands.open.as_mut().unwrap().content {
        *kind = BlockKind::Numbered;
    }
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.bands.open.is_none());
    assert_eq!(
        app.document
            .engine
            .document()
            .view
            .blocks
            .iter()
            .filter(|k| **k == BlockKind::Numbered)
            .count(),
        2
    );
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
    for remove in [false, true] {
        open_widget(&mut app, &ctx, remove);
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(app.bands.open.is_none());
        assert_eq!(app.stored_document(), before);
    }
    open_widget(&mut app, &ctx, true);
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.bands.open.is_none());
    assert!(editor::widgets(&app.document.engine.document().view).is_empty());
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
}

#[test]
fn return_activates_the_focused_cancel_button_instead_of_the_default_action() {
    let (ctx, mut app) = setup("- Un\n- Deux", "Deux");
    let before = app.stored_document();
    open_widget(&mut app, &ctx, true);
    let output = search_draw(&mut app, &ctx, vec![]);
    let cancel = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == crate::tr!("cancel") => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap();
    let mut focused = false;
    for _ in 0..30 {
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
        );
        if ctx
            .memory(|m| m.focused())
            .and_then(|id| ctx.read_response(id))
            .is_some_and(|r| r.rect.contains(cancel))
        {
            focused = true;
            break;
        }
    }
    assert!(focused, "the cancel button is reachable using Tab");
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.bands.open.is_none());
    assert_eq!(app.stored_document(), before);
}

#[test]
fn spelling_return_applies_the_first_suggestion_and_ime_return_never_applies_a_draft() {
    let (ctx, mut app) = setup("Un courrrier.", "courrrier");
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.apply(Command::SpellingSuggestions);
    app.spelling.checker.wait();
    settle(&mut app, &ctx);
    assert!(app.bands.open.is_some());
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.bands.open.is_none());
    assert_eq!(app.document.engine.document().view.text, "Un courrier.");
    assert_eq!(app.document.engine.history().undo, 1);

    let (ctx, mut app) = setup("- Un\n- Deux", "Deux");
    open_widget(&mut app, &ctx, false);
    let before = app.stored_document();
    app.show_band(
        app.document
            .engine
            .selection_target(editor::to_core(app.document.selection))
            .unwrap(),
        BandContent::Link { url: String::new() },
    );
    settle(&mut app, &ctx);
    for events in [
        vec![egui::Event::Ime(egui::ImeEvent::Enabled)],
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("にほん".into()))],
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    ] {
        frame(&mut app, &ctx, events);
        assert!(app.bands.open.is_some());
        assert_eq!(app.stored_document(), before);
    }
}
