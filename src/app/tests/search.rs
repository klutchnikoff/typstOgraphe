use super::*;

#[test]
fn incremental_search_keeps_one_selection_and_clicking_another_result_updates_preview_highlights() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    let mut letter = StoredDocument::blank(DocumentType::Courrier);
    letter.source = "Un *été*, puis un ÉTÉ et encore un _été_.".into();
    let document = Document::from_source(&letter.source).unwrap();
    app.install_document(letter, document, None).unwrap();
    let before = app.stored_document();
    let mut world = crate::rendering::TypstWorld::default();
    let compiled = world
        .compile_with_template(
            app.session.document.source(),
            &app.session.metadata,
            &app.session.template,
        )
        .unwrap();
    app.sync = Some(RenderSync::new(
        app.session.revision,
        world,
        compiled,
        app.session.document.source_map.clone(),
    ));
    app.rendered_revision = Some(app.session.revision);
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
    let caret = app.selection;
    let _ = search_draw(&mut app, &ctx, vec![egui::Event::Text("été".into())]);
    assert_eq!(app.search.engine.matches.len(), 3);
    assert_eq!(
        app.selection, caret,
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
                    if text.galley.job.text == app.session.document.view.text =>
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
        let highlights = app.sync.as_ref().unwrap().selection_highlights(app.range());
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
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
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
    assert!(app.session.document.view.text.contains("Noël"));
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
    assert_eq!(app.session.undo.len(), 1);
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
