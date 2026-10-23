use super::*;

#[test]
fn search_bar_enter_shift_enter_and_replacement_match_json_with_focus_preserved() {
    use crate::engine::{Command as Core, Query, SearchParams};
    use serde_json::json;
    let (ctx, mut app, mut engine) =
        super::annotations_engine::pair("Un *été* et un _ÉTÉ_, été.", 0..0);
    app.apply(Command::FindReplace);
    app.search.engine.query = "été".into();
    app.search.engine.replacement = "Noël".into();
    let _ = search_draw(&mut app, &ctx, vec![]);
    for (id, modifiers) in [
        ("edit.find_next", egui::Modifiers::NONE),
        ("edit.find_next", egui::Modifiers::NONE),
        ("edit.find_previous", egui::Modifiers::SHIFT),
    ] {
        engine
            .run_call(
                Core::from_call(id, json!({"query":"été"}))
                    .unwrap()
                    .to_call(),
            )
            .unwrap();
        let _ = search_draw(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
        );
        super::annotations_engine::equivalent(&app, &engine);
        assert_eq!(app.search_focus(&ctx), Some(egui::Id::new(SEARCH_ID)));
    }
    let found = engine
        .query_call(
            Query::Find(SearchParams {
                query: "été".into(),
                case_sensitive: false,
            })
            .to_call(),
        )
        .unwrap();
    let occurrence = found["matches"][0].clone();
    engine
        .run_call(
            Core::from_call(
                "edit.replace_current",
                json!({"query":"été","occurrence":occurrence,"replacement":"Noël"}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(REPLACE_ID)));
    let _ = search_draw(
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
    super::annotations_engine::equivalent(&app, &engine);
    assert_eq!(app.search_focus(&ctx), Some(egui::Id::new(REPLACE_ID)));
    engine
        .run_call(
            Core::from_call(
                "edit.replace_all",
                json!({"guard":engine.target_guard(),"query":"été","replacement":"Noël"}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap();
    app.apply(Command::ReplaceAll);
    super::annotations_engine::equivalent(&app, &engine);
    for (ui, id) in [(Command::Undo, "edit.undo"), (Command::Redo, "edit.redo")] {
        app.apply(ui);
        engine
            .run_call(Core::from_call(id, json!({})).unwrap().to_call())
            .unwrap();
        super::annotations_engine::equivalent(&app, &engine);
    }
}

#[test]
fn search_cache_is_revision_and_session_bound_and_stale_clicks_are_refused() {
    let (_ctx, mut app) = demo_app();
    app.apply(Command::Find);
    app.search.engine.query = "Madame".into();
    app.refresh_search();
    assert!(!app.search.occurrences.is_empty());
    let old_guard = app.search.guard.unwrap();
    app.document
        .engine
        .run(crate::engine::Command::InsertText(
            crate::engine::InsertTextParams {
                target: Default::default(),
                text: "X".into(),
                input_rules: Default::default(),
            },
        ))
        .unwrap();
    let before = app.stored_document();
    let selection = app.document.selection;
    app.select_search_result(0);
    assert!(app.stored_document() == before);
    assert_eq!(app.document.selection, selection);
    assert_eq!(
        app.dialogs.message.as_deref(),
        Some(crate::tr!("edit-stale-draft").as_str())
    );
    app.refresh_search();
    assert_ne!(app.search.guard.unwrap(), old_guard);
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "Madame, bonjour Madame.".into();
    app.install_document(
        stored.clone(),
        Document::from_source(&stored.source).unwrap(),
        None,
    )
    .unwrap();
    app.search.engine.query = "Madame".into();
    app.refresh_search();
    assert_eq!(app.search.occurrences.len(), 2);
    assert_ne!(app.search.guard.unwrap().document_id, old_guard.document_id);
    app.select_search_result(1);
    assert_eq!(app.range(), 16..22);
}

#[test]
fn incremental_search_keeps_one_selection_and_clicking_another_result_updates_preview_highlights() {
    let (ctx, mut app) = demo_app();
    let mut letter = StoredDocument::blank(DocumentType::Courrier);
    letter.source = "Un *été*, puis un ÉTÉ et encore un _été_.".into();
    let document = Document::from_source(&letter.source).unwrap();
    app.install_document(letter, document, None).unwrap();
    let before = app.stored_document();
    let mut world = crate::rendering::TypstWorld::default();
    let compiled = world
        .compile_with_template(
            app.document.engine.document().source(),
            app.document.engine.metadata(),
            app.document.engine.template(),
        )
        .unwrap();
    app.preview.render = Some(
        app.document
            .engine
            .install_render(
                app.document.engine.target_guard(),
                crate::navigation::RenderSync::new(
                    app.document.engine.revision(),
                    world,
                    compiled,
                    app.document.engine.document().source_map.clone(),
                ),
            )
            .unwrap(),
    );
    app.preview.revision = Some(app.document.engine.revision());
    let _ = search_draw(&mut app, &ctx, vec![]);
    let key = |key, modifiers| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let _ = search_draw(
        &mut app,
        &ctx,
        vec![key(egui::Key::F, egui::Modifiers::COMMAND)],
    );
    assert!(app.search.open && app.search_focus(&ctx).is_some());
    let caret = app.document.selection;
    let _ = search_draw(&mut app, &ctx, vec![egui::Event::Text("été".into())]);
    assert_eq!(app.search.engine.matches.len(), 3);
    assert_eq!(
        app.document.selection, caret,
        "la saisie de recherche ne déplace pas le curseur du document"
    );
    assert_eq!(app.stored_document(), before);
    assert!(!app.is_dirty());
    let _ = search_draw(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert_eq!(app.range(), app.search.engine.matches[0]);
    assert!(
        app.search_focus(&ctx).is_some(),
        "Entrée doit permettre de continuer la saisie de recherche"
    );
    let mut previous = None;
    for index in [1, 2] {
        let output = search_draw(&mut app, &ctx, vec![]);
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.text == app.document.engine.document().view.text =>
                {
                    editor::range_rects(&text.galley, &app.search.engine.matches)
                        .into_iter()
                        .find(|(found, _)| *found == index)
                        .map(|(_, rect)| text.pos + rect.center().to_vec2())
                }
                _ => None,
            })
            .expect("résultat visible dans l’éditeur");
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
        let _ = search_draw(&mut app, &ctx, vec![]);
        assert_eq!(app.range(), app.search.engine.matches[index]);
        let native = egui::TextEdit::load_state(&ctx, egui::Id::new(editor::EDITOR_ID))
            .unwrap()
            .cursor
            .char_range()
            .unwrap();
        assert_eq!(
            native.as_sorted_char_range(),
            app.search.engine.matches[index]
        );
        assert_eq!(
            app.search.engine.matches.len(),
            3,
            "l’ancienne occurrence reste un résultat surligné"
        );
        let highlights = app
            .document
            .engine
            .selection_boxes(crate::engine::SelectionBoxesParams {
                render: app.preview.render.as_ref().unwrap().reference,
                target: app
                    .document
                    .engine
                    .selection_target(app.document.selection.into())
                    .unwrap()
                    .into(),
            })
            .unwrap()
            .boxes;
        assert!(!highlights.is_empty());
        let first = highlights[0].corners;
        assert!(previous.is_none_or(|previous| previous != first));
        previous = Some(first);
    }
    let _ = search_draw(
        &mut app,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(!app.search.open);
    assert_eq!(app.range(), app.search.engine.matches[2]);
}

#[test]
fn replacement_commands_refresh_results_and_replace_all_is_a_single_document_transaction() {
    let (_ctx, mut app) = demo_app();
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "Un *été* et un _ÉTÉ_, été.".into();
    app.install_document(
        stored.clone(),
        Document::from_source(&stored.source).unwrap(),
        None,
    )
    .unwrap();
    app.apply(Command::FindReplace);
    app.search.engine.query = "été".into();
    app.search.engine.replacement = "Noël".into();
    app.apply(Command::FindNext);
    assert_eq!(app.search.engine.matches.len(), 3);
    app.apply(Command::ReplaceCurrent);
    assert_eq!(app.search.engine.matches.len(), 2);
    assert_eq!(app.range(), app.search.engine.matches[0]);
    assert!(app.document.engine.document().view.text.contains("Noël"));
    app.apply(Command::Undo);
    app.refresh_search();
    assert_eq!(app.stored_document(), stored);
    app.search.engine.replacement = "été été".into();
    app.apply(Command::ReplaceAll);
    assert_eq!(
        app.search.engine.matches.len(),
        6,
        "les nouveaux résultats ne sont pas remplacés en boucle"
    );
    assert_eq!(app.document.engine.undo().len(), 1);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), stored);
    app.apply(Command::Redo);
    app.refresh_search();
    assert_eq!(app.search.engine.matches.len(), 6);
    app.search.engine.replacement.clear();
    app.apply(Command::ReplaceAll);
    assert!(app.search.engine.matches.is_empty());
    app.apply(Command::Undo);
    app.refresh_search();
    assert_eq!(app.search.engine.matches.len(), 6);
}
