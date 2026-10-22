use super::*;

fn editing(source: &str, range: Range<usize>) -> (egui::Context, App) {
    let (ctx, mut app) = demo_app();
    app.document.session.document = Document::from_source(source).unwrap();
    app.document.session.saved = app.stored_document();
    app.document.session.undo.clear();
    app.document.session.redo.clear();
    app.document.session.typing_group = false;
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
        let revision = app.document.session.revision;
        frame(&mut app, &ctx, vec![event]);
        assert_eq!(app.stored_document(), stored, "{source}");
        assert_eq!(app.document.session.revision, revision);
        assert_eq!(app.document.selection.as_sorted_char_range(), range);
        assert!(app.document.session.undo.is_empty());
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
    assert!(app.document.session.undo.is_empty());
    frame(&mut app, &ctx, vec![egui::Event::Text("Été".into())]);
    assert_eq!(app.document.session.document.view.text, "\u{fffc}Été");
    assert_eq!(app.document.session.undo.len(), 1);
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
    assert_eq!(app.document.session.document.source(), "");
    assert_eq!(
        app.document.session.document.view.blocks,
        [BlockKind::Paragraph]
    );
    assert_eq!(app.document.session.undo.len(), 1);
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
        let document = Document::from_source(source).unwrap();
        let mut explicit = Session::new(
            stored.clone(),
            document,
            None,
            app.document.session.revision,
        );
        explicit.document.view.typing_style =
            app.document.session.document.view.typing_style.clone();
        let target = explicit
            .selection_target(editor::cursor(range.clone()).into())
            .unwrap();
        let explicit_selection = explicit.replace_text(target, replacement).unwrap();
        frame(&mut app, &ctx, vec![egui::Event::Text(replacement.into())]);
        assert_eq!(
            app.document.session.document.source(),
            explicit.document.source(),
            "{source}"
        );
        assert!(
            app.stored_document() == explicit.stored_document(),
            "{source}"
        );
        assert_eq!(app.document.session.document.view, explicit.document.view);
        assert_eq!(
            app.document.selection.as_sorted_char_range(),
            explicit_selection.range()
        );
        assert_eq!(app.document.session.revision, explicit.revision);
        assert_eq!(app.document.session.undo.len(), explicit.undo.len());
        app.apply(Command::Undo);
        explicit.history(true, explicit_selection).unwrap();
        assert_eq!(app.stored_document(), stored);
        assert_eq!(explicit.stored_document(), stored);
    }
}
