use super::*;

#[test]
fn links_accept_soft_lines_and_reject_paragraph_or_list_boundaries() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx, None);
    for (source, selection, allowed) in [
        ("Été\nNoël", 0..8, true),
        ("Été\n\nNoël", 0..9, false),
        ("- Un\n- Deux", 0..7, false),
    ] {
        fixture(&mut app.document.engine, |session| {
            session.document = Document::from_source(source).unwrap();
        });
        app.document.selection = editor::cursor(selection);
        app.dialogs.link_open = false;
        app.apply(Command::Link);
        assert_eq!(app.dialogs.link_open, allowed, "{source}");
    }
}

#[test]
fn native_enter_distinguishes_soft_newlines_and_paragraphs_with_undo_and_reopening() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source("Alpha").unwrap();
    });
    app.document.selection = editor::cursor(5..5);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert_eq!(app.document.engine.document().source(), "Alpha\n");
    frame(&mut app, &ctx, vec![egui::Event::Text("Beta".into())]);
    assert_eq!(app.document.engine.document().source(), "Alpha\nBeta");
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert_eq!(app.document.engine.document().view.text, "Alpha\nBeta\n\n");
    frame(&mut app, &ctx, vec![egui::Event::Text("Gamma".into())]);
    let paragraphs = app.document.engine.document().clone();
    app.document.selection = editor::cursor(12..12);
    app.document.force_selection = true;
    fixture(&mut app.document.engine, |session| {
        session.typing_group = false;
    });
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![key(egui::Key::Backspace)]);
    assert_eq!(
        app.document.engine.document().source(),
        "Alpha\nBeta\nGamma"
    );
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document(), &paragraphs);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paragraphes.typsto");
    assert!(app.save_path(path.clone()));
    assert!(app.open_path(path));
    assert_eq!(app.document.engine.document().view, paragraphs.view);
    assert_eq!(app.document.engine.document().source(), paragraphs.source());
    assert!(!app.document.engine.document().source().contains('¶'));
}

#[test]
fn native_egui_replaces_a_unicode_selection_without_source_markers() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source("Élodie écrit.\n\nDeuxième paragraphe.").unwrap();
    });
    app.document.selection = editor::cursor(0..6);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![egui::Event::Text("Noël".into())]);
    assert!(
        app.document
            .engine
            .document()
            .view
            .text
            .starts_with("Noël écrit."),
        "{}",
        app.document.engine.document().view.text
    );
    assert_eq!(
        app.document.engine.document().view.len(),
        app.document.engine.document().view.text.chars().count()
    );
    assert!(Document::from_source(app.document.engine.document().source()).is_ok());
    app.apply(Command::Undo);
    assert!(
        app.document
            .engine
            .document()
            .view
            .text
            .starts_with("Élodie écrit.")
    );
}

#[test]
fn native_keyboard_shortcut_formats_multiple_paragraphs() {
    let (ctx, mut app) = demo_app();
    app.document.selection = editor::cursor(0..180);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::I,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    assert!(
        app.document
            .engine
            .document()
            .view
            .mark_state(0..180, Mark::Italic)
            .0
    );
    assert!(!app.document.engine.document().view.text.contains("#emph"));
    assert_eq!(app.document.selection.as_sorted_char_range(), 0..180);
}

#[test]
fn text_shortcuts_can_be_undone_to_the_complete_literal_input() {
    for (source, closing, expected, kind) in [
        ("\\=", " ", "= ", Some(BlockKind::Heading(1))),
        ("\\/", " ", "/ ", Some(BlockKind::Description)),
        ("\\>", " ", "> ", Some(BlockKind::Quote)),
        ("\\*Bonjour", "*", "*Bonjour*", None),
    ] {
        let (ctx, mut app) = demo_app();
        fixture(&mut app.document.engine, |session| {
            session.document = Document::from_source(source).unwrap();
        });
        let end = app.document.engine.document().view.len();
        app.document.selection = editor::cursor(end..end);
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![egui::Event::Text(closing.into())]);
        if let Some(kind) = kind {
            assert_eq!(app.document.engine.document().view.blocks[0], kind);
        } else {
            assert!(
                app.document
                    .engine
                    .document()
                    .view
                    .mark_state(0..7, Mark::Bold)
                    .0
            );
        }
        app.apply(Command::Undo);
        assert_eq!(app.document.engine.document().view.text, expected);
    }
}

#[test]
fn description_editing_uses_tab_and_colon_without_exposing_typst() {
    for separator in [
        egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::Text(" ".into()),
    ] {
        let (ctx, mut app) = demo_app();
        fixture(&mut app.document.engine, |session| {
            session.document = Document::from_source("Échéance").unwrap();
        });
        app.document.selection = editor::cursor(8..8);
        app.apply(Command::Block(BlockKind::Description));
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        if matches!(separator, egui::Event::Text(_)) {
            frame(&mut app, &ctx, vec![egui::Event::Text(":".into())]);
        }
        frame(&mut app, &ctx, vec![separator]);
        frame(&mut app, &ctx, vec![egui::Event::Text("Vendredi".into())]);
        assert_eq!(
            app.document.engine.document().view.text,
            "Échéance\tVendredi"
        );
        assert_eq!(
            app.document.engine.document().view.blocks,
            [BlockKind::Description]
        );
        let restored = Document::from_source(app.document.engine.document().source()).unwrap();
        assert_eq!(restored.view.text, app.document.engine.document().view.text);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
        );
        assert_eq!(app.document.selection.primary.index, 0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(app.document.selection.primary.index, 9);
        let end = app.document.engine.document().view.len();
        app.document.selection = editor::cursor(end..end);
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(
            app.document.engine.document().view.blocks,
            [BlockKind::Description; 2]
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(
            app.document.engine.document().view.blocks,
            [BlockKind::Description, BlockKind::Paragraph]
        );
    }
}

#[test]
fn quote_keyboard_edits_body_and_attribution_and_exits_without_losing_source() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source("Bonjour").unwrap();
    });
    app.document.selection = editor::cursor(7..7);
    app.apply(Command::Block(BlockKind::Quote));
    frame(&mut app, &ctx, vec![]);
    assert_eq!(
        app.document.engine.document().view.blocks,
        [BlockKind::Quote, BlockKind::QuoteAttribution]
    );
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
    frame(&mut app, &ctx, vec![egui::Event::Text("Élodie".into())]);
    assert_eq!(app.document.engine.document().view.text, "Bonjour\nÉlodie");
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        }],
    );
    assert_eq!(app.document.selection.primary.index, 0);
    app.document.selection = editor::cursor(7..7);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("Deuxième pensée.".into())],
    );
    assert_eq!(
        app.document.engine.document().view.text,
        "Bonjour\nDeuxième pensée.\nÉlodie"
    );
    assert_eq!(
        app.document.engine.document().view.blocks,
        [
            BlockKind::Quote,
            BlockKind::Quote,
            BlockKind::QuoteAttribution
        ]
    );
    let source = app.document.engine.document().source().to_owned();
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert_eq!(
        app.document.engine.document().view.blocks,
        [
            BlockKind::Quote,
            BlockKind::Quote,
            BlockKind::QuoteAttribution,
            BlockKind::Paragraph
        ]
    );
    frame(&mut app, &ctx, vec![egui::Event::Text("Suite.".into())]);
    let restored = Document::from_source(app.document.engine.document().source()).unwrap();
    assert_eq!(restored.view, app.document.engine.document().view);
    app.apply(Command::Undo);
    app.apply(Command::Undo);
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document().source(), source);
}

#[test]
fn converting_multiple_paragraphs_to_quote_preserves_selection_and_can_be_undone() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx, None);
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source("Un.\n\nDeux.\n\nTrois.").unwrap();
    });
    let original = app.document.engine.document().clone();
    app.document.selection = CCursorRange::two(CCursor::new(3), CCursor::new(0));
    app.apply(Command::Block(BlockKind::Quote));
    assert_eq!(app.document.selection.primary.index, 0);
    assert_eq!(app.document.selection.secondary.index, 3);
    assert_eq!(
        app.document.engine.document().view.text,
        "Un.\n\n\nDeux.\n\nTrois."
    );
    assert_eq!(
        app.document.engine.document().view.blocks,
        [
            BlockKind::Quote,
            BlockKind::QuoteAttribution,
            BlockKind::Paragraph,
            BlockKind::Paragraph,
            BlockKind::Paragraph,
            BlockKind::Paragraph
        ]
    );
    assert_eq!(
        Document::from_source(app.document.engine.document().source())
            .unwrap()
            .view,
        app.document.engine.document().view
    );
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document(), &original);
    app.document.selection = editor::cursor(0..8);
    app.apply(Command::Block(BlockKind::Quote));
    assert_eq!(
        app.document.engine.document().view.quote_groups()[0].body,
        0..3
    );
    app.document.selection = editor::cursor(0..app.document.engine.document().view.len());
    app.apply(Command::Block(BlockKind::Paragraph));
    assert!(
        !app.document
            .engine
            .document()
            .view
            .blocks
            .iter()
            .any(|kind| matches!(kind, BlockKind::Quote | BlockKind::QuoteAttribution))
    );
}

#[test]
fn description_tab_with_a_selection_and_enter_in_text_create_an_empty_label() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source("/ Échéance: Vendredi").unwrap();
    });
    app.document.selection = editor::cursor(2..5);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
    assert_eq!(app.document.selection.as_sorted_char_range(), 9..9);
    assert_eq!(
        app.document.engine.document().view.text,
        "Échéance\tVendredi"
    );
    app.document.selection = editor::cursor(12..12);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    let before = app.document.engine.document().source().to_owned();
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert_eq!(
        app.document.engine.document().view.text,
        "Échéance\tVen\n\tdredi"
    );
    assert_eq!(app.document.selection.primary.index, 13);
    assert_eq!(
        app.document.engine.document().view.blocks,
        [BlockKind::Description; 2]
    );
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document().source(), before);
    app.apply(Command::Redo);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![egui::Event::Text("Contact".into())]);
    assert_eq!(
        app.document.engine.document().view.text,
        "Échéance\tVen\nContact\tdredi"
    );
    let restored = Document::from_source(app.document.engine.document().source()).unwrap();
    assert_eq!(restored.view, app.document.engine.document().view);
}

#[test]
fn letter_notes_insert_type_scope_editing_exit_and_undo_use_the_shared_session() {
    let (ctx, mut app) = demo_app();
    let body = app.document.engine.document().view.text.clone();
    app.apply(Command::InsertPostscript);
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("Merci encore.".into())],
    );
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("#postscript[Merci encore.]")
    );
    app.apply(Command::InsertAttachment);
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("Formulaire signé".into())],
    );
    let key = |key| egui::Event::Key {
        key,
        pressed: true,
        repeat: false,
        physical_key: None,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("Rapport annuel".into())],
    );
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("#attachment[Formulaire signé]")
    );
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("#attachment[Rapport annuel]")
    );
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    let line = app
        .document
        .engine
        .document()
        .view
        .line_at(app.range().start);
    assert_eq!(
        app.document.engine.document().view.blocks[line],
        BlockKind::Paragraph
    );
    app.apply(Command::Undo);
    assert_eq!(
        app.document.engine.document().view.blocks[line],
        BlockKind::Attachment
    );
    app.apply(Command::Redo);
    assert_eq!(
        app.document.engine.document().view.blocks[line],
        BlockKind::Paragraph
    );
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("courrier.typsto");
    assert!(app.save_path(archive.clone()));
    let stored = crate::storage::read(&archive).unwrap();
    assert_eq!(stored.source, app.document.engine.document().source());
    assert!(app.document.engine.document().view.text.starts_with(&body));
    assert!(
        stored
            .dependencies
            .contains_key("capabilities/correspondence.typ")
    );
    let registry = crate::commands::Registry::new(DocumentType::CompteRendu);
    assert!(!registry.contains(Command::InsertPostscript));
    assert!(!registry.contains(Command::InsertAttachment));
}
