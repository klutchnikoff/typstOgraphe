use super::*;
use crate::engine::{Empty, Engine, Query};
use serde_json::json;

#[test]
fn migrated_ui_presentations_use_the_canonical_typed_identifiers() {
    for kind in DocumentType::ALL {
        for presentation in Registry::new(kind).definitions() {
            if let Some(command) = presentation.command.document_command() {
                assert_eq!(presentation.id, command.id());
                assert!(
                    crate::engine::Command::from_call(presentation.id, command.to_call().params)
                        .is_ok()
                );
            }
        }
    }
}

#[test]
fn formatting_blocks_and_letter_actions_match_json_calls_with_selection_and_history() {
    for (trigger, id, source, range) in [
        (Command::Bold, "text.bold", "Un été.", 3..6),
        (Command::Italic, "text.italic", "Un été.", 3..6),
        (Command::Clear, "text.clear", "Un *été*.", 3..6),
        (
            Command::RemoveLink,
            "text.remove_link",
            "#link(\"https://example.org\")[Élodie]",
            0..6,
        ),
        (
            Command::Block(BlockKind::Paragraph),
            "block.paragraph",
            "= Titre",
            0..5,
        ),
        (
            Command::Block(BlockKind::Heading(1)),
            "block.heading_1",
            "Un titre",
            0..8,
        ),
        (
            Command::Block(BlockKind::Heading(2)),
            "block.heading_2",
            "Un titre",
            0..8,
        ),
        (
            Command::Block(BlockKind::Heading(3)),
            "block.heading_3",
            "Un titre",
            0..8,
        ),
        (
            Command::Block(BlockKind::Bullet),
            "block.bullet",
            "Premier\n\nSecond",
            0..15,
        ),
        (
            Command::Block(BlockKind::Numbered),
            "block.numbered",
            "Premier\n\nSecond",
            0..15,
        ),
        (
            Command::Block(BlockKind::Description),
            "block.description",
            "Un terme",
            0..8,
        ),
        (
            Command::Block(BlockKind::Quote),
            "block.quote",
            "Une idée",
            0..8,
        ),
        (
            Command::InsertPostscript,
            "letter.insert_postscript",
            "Bonjour",
            0..0,
        ),
        (
            Command::InsertAttachment,
            "letter.insert_attachment",
            "Bonjour",
            0..0,
        ),
    ] {
        let (ctx, mut app) = demo_app();
        fixture(&mut app.document.engine, |session| {
            session.document = Document::from_source(source).unwrap();
        });
        app.document.selection = editor::cursor(range);
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let before = app.stored_document();
        let mut engine: Engine = Session::new(
            before.clone(),
            app.document.engine.document().clone(),
            None,
            app.document.engine.revision(),
        )
        .into();
        engine
            .set_adapter_selection(app.document.selection.into())
            .unwrap();
        engine
            .run(crate::engine::Command::from_call(id, json!({})).unwrap())
            .unwrap();
        app.apply(trigger);
        assert_eq!(
            app.document.engine.document().source(),
            engine.document().source(),
            "{id}"
        );
        assert!(app.stored_document() == engine.stored_document(), "{id}");
        assert_eq!(
            app.document.engine.document().view,
            engine.document().view,
            "{id}"
        );
        assert_eq!(
            app.document.selection,
            CCursorRange::from(engine.selection()),
            "{id}"
        );
        assert_eq!(
            app.document.engine.undo().len(),
            engine.undo().len(),
            "{id}"
        );
        app.apply(Command::Undo);
        engine.run(crate::engine::Command::Undo(Empty {})).unwrap();
        assert!(
            app.stored_document() == before && engine.stored_document() == before,
            "{id}"
        );
        assert_eq!(
            app.document.selection,
            CCursorRange::from(engine.selection()),
            "{id}"
        );
        app.apply(Command::Redo);
        engine.run(crate::engine::Command::Redo(Empty {})).unwrap();
        assert!(app.stored_document() == engine.stored_document(), "{id}");
    }
}

#[test]
fn native_typing_selection_and_preedit_keep_engine_queries_in_sync_with_the_published_document() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source("Texte").unwrap();
    });
    app.document.selection = editor::cursor(0..5);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    assert_eq!(
        app.document.engine.selection(),
        app.document.selection.into()
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("にほん".into()))],
    );
    let published = app
        .document
        .engine
        .query(Query::Text(crate::engine::TextQueryParams {
            target: crate::engine::TextTarget::All {},
        }))
        .unwrap();
    assert_eq!(published["text"], "Texte");
    assert_eq!(
        app.document.engine.selection(),
        app.document.selection.into()
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Commit("日本".into()))],
    );
    let published = app
        .document
        .engine
        .query(Query::Text(crate::engine::TextQueryParams {
            target: crate::engine::TextTarget::All {},
        }))
        .unwrap();
    assert_eq!(published["text"], "日本");
    assert_eq!(app.document.engine.selection().range(), 2..2);
    let inspect = app.document.engine.query(Query::Inspect(Empty {})).unwrap();
    assert_eq!(
        inspect["selection"],
        serde_json::to_value(
            app.document
                .engine
                .selection_target(app.document.selection.into())
                .unwrap()
        )
        .unwrap()
    );
}

#[test]
fn link_dialog_uses_the_captured_selection_and_refuses_a_stale_revision() {
    for stale in [false, true] {
        let (ctx, mut app) = demo_app();
        fixture(&mut app.document.engine, |session| {
            session.document = Document::from_source("Avant après").unwrap();
        });
        app.document.selection = editor::cursor(0..5);
        app.apply(Command::Link);
        app.dialogs.link_url = "https://example.org".into();
        app.document.selection = editor::cursor(6..11);
        if stale {
            fixture(&mut app.document.engine, |session| {
                session.metadata.courrier_mut().objet = "Autre objet".into();
            });
            app.changed();
        }
        recovery_click(&mut app, &ctx, &crate::tr!("apply"));
        if stale {
            assert!(app.dialogs.link_open);
            assert_eq!(
                app.dialogs.link_error.as_deref(),
                Some(crate::tr!("edit-stale-draft").as_str())
            );
            assert!(
                app.document
                    .engine
                    .document()
                    .view
                    .styles
                    .iter()
                    .all(|style| style.link.is_none())
            );
        } else {
            assert!(!app.dialogs.link_open);
            assert!(
                app.document.engine.document().view.styles[..5]
                    .iter()
                    .all(|style| style.link.as_deref() == Some("https://example.org"))
            );
            assert!(
                app.document.engine.document().view.styles[6..]
                    .iter()
                    .all(|style| style.link.is_none())
            );
        }
    }
}
