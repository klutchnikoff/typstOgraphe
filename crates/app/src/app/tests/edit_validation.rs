use super::*;

fn editing(source: &str, range: Range<usize>) -> (egui::Context, App) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app.document.engine, |data| {
        data.document = Document::from_source(source).unwrap();
    });
    app.document
        .engine
        .update_adapter_cursor(editor::to_core(app.document.selection))
        .unwrap();
    app.document.selection = editor::cursor(range);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    (ctx, app)
}
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn native_replacements_cut_and_backspace_refuse_role_boundaries_and_restore_selection() {
    for (source, range, event) in [
        ("/ Mot: Définition", 2..6, egui::Event::Text("X".into())),
        (
            "#quote(block: true, attribution: [Auteur])[Texte]",
            4..7,
            egui::Event::Paste("X".into()),
        ),
        ("/ Mot: Définition", 2..6, egui::Event::Cut),
        ("/ Mot: Définition", 4..4, key(egui::Key::Backspace)),
        (
            "#quote(block: true, attribution: [Auteur])[Texte]",
            6..6,
            key(egui::Key::Backspace),
        ),
    ] {
        let (ctx, mut app) = editing(source, range.clone());
        let stored = app.stored_document();
        let revision = app.document.engine.revision();
        frame(&mut app, &ctx, vec![event]);
        assert_eq!(app.stored_document(), stored, "{source}");
        assert_eq!(app.document.engine.revision(), revision);
        assert_eq!(app.document.selection.as_sorted_char_range(), range);
        assert!(!app.document.engine.can_undo());
        assert_eq!(
            app.dialogs.message.as_deref(),
            Some(crate::tr!("edit-protected-boundary").as_str())
        );
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.document.selection.as_sorted_char_range(), range);
    }
}

#[test]
fn rejected_paste_keeps_original_caption_and_a_following_valid_edit_works() {
    let source = "#figure(image(\"images/parc.svg\"), caption: [Cap])";
    let (ctx, mut app) = editing(source, 1..4);
    let before = app.stored_document();
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("Deux\nlignes".into())],
    );
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.selection.as_sorted_char_range(), 1..4);
    assert!(!app.document.engine.can_undo());
    frame(&mut app, &ctx, vec![egui::Event::Text("Été".into())]);
    assert_eq!(app.document.engine.document().view.text, "\u{fffc}Été");
    assert_eq!(app.document.engine.history().undo, 1);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
}

#[test]
fn native_select_all_delete_clears_all_structures_and_undo_restores_them() {
    let source = "/ Mot: Définition\n\n#quote(block: true, attribution: [Auteur])[Texte]\n\n#figure(image(\"images/parc.svg\"), caption: [Cap])";
    let len = Document::from_source(source).unwrap().view.len();
    let (ctx, mut app) = editing(source, 0..len);
    let original = app.stored_document();
    frame(&mut app, &ctx, vec![key(egui::Key::Backspace)]);
    assert_eq!(app.document.engine.document().source(), "");
    assert_eq!(
        app.document.engine.document().view.blocks,
        [BlockKind::Paragraph]
    );
    assert_eq!(app.document.engine.history().undo, 1);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), original);
    assert_eq!(app.document.selection.as_sorted_char_range(), 0..len);
}

#[test]
fn native_and_explicit_unicode_replacements_share_source_styles_normalization_and_history() {
    for (source, range, replacement) in [
        ("Un *été*.", 3..6, "Noël"),
        ("= *Titre*", 0..5, "Autre"),
        ("/ Mot: Définition", 4..14, "Élodie"),
        (
            "#quote(block: true, attribution: [Auteur])[Texte]",
            0..5,
            "Une\nidée",
        ),
        (
            "#figure(image(\"images/parc.svg\"), caption: [Cap])",
            1..4,
            "Légende",
        ),
        ("Alpha", 5..5, "\n\n\nBeta"),
    ] {
        let (ctx, mut app) = editing(source, range.clone());
        let stored = app.stored_document();
        let mut explicit: crate::engine::Engine =
            crate::engine::Engine::from_stored(stored.clone(), None).unwrap();
        let draft = explicit.begin_native_edit(editor::to_core(app.document.selection));
        let mut view = draft.view().clone();
        view.typing_style = app.document.engine.document().view.typing_style.clone();
        explicit
            .commit_native_edit(
                draft,
                view,
                editor::to_core(app.document.selection),
                crate::engine::NativeInput::Typing,
            )
            .unwrap();
        let target = explicit
            .selection_target(editor::to_core(editor::cursor(range.clone())))
            .unwrap();
        let command = crate::engine::Command::from_call(
            "text.insert",
            serde_json::json!({
                "target": crate::engine::TextTarget::from(target), "text": replacement,
            }),
        )
        .unwrap();
        explicit.run_call(command.to_call()).unwrap();
        let explicit_selection = explicit.selection();
        frame(&mut app, &ctx, vec![egui::Event::Text(replacement.into())]);
        assert_eq!(
            app.document.engine.document().source(),
            explicit.document().source(),
            "{source}"
        );
        assert!(
            app.stored_document() == explicit.stored_document(),
            "{source}"
        );
        assert_eq!(
            app.document.engine.document().view,
            explicit.document().view
        );
        assert_eq!(
            app.document.selection.as_sorted_char_range(),
            explicit_selection.range()
        );
        assert_eq!(app.document.engine.revision(), explicit.revision());
        assert_eq!(app.document.engine.history().undo, explicit.history().undo);
        app.apply(Command::Undo);
        explicit
            .run(crate::engine::Command::Undo(crate::engine::Empty {}))
            .unwrap();
        assert_eq!(app.stored_document(), stored);
        assert_eq!(explicit.stored_document(), stored);
    }
}
