use super::*;

#[test]
fn spelling_context_menu_corrects_rich_text_with_undo_and_never_applies_a_stale_suggestion() {
    let ctx = egui::Context::default();
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut app = App::with_demo_context(ctx.clone(), None);
    let mut letter = StoredDocument::blank(DocumentType::Courrier);
    letter.source = "Un *courrrier* et une _réservvation_.".into();
    app.install_document(letter, None).unwrap();
    let original = app.stored_document();
    let _ = search_draw(&mut app, &ctx, vec![]);
    app.spelling.checker.wait();
    assert_eq!(app.spelling.checker.issues.len(), 2);
    let issue = app.spelling.checker.issues[0].clone();
    let target = app.spelling_reference(&issue).unwrap();
    let output = search_draw(&mut app, &ctx, vec![]);
    let point = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.text() == app.document.engine.document().view.text =>
            {
                editor::range_rects(&text.galley, std::slice::from_ref(&issue.range))
                    .first()
                    .map(|(_, rect)| text.pos + rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap();
    for pressed in [true, false] {
        let _ = search_draw(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(app.spelling.target, Some(target.clone()));
    assert_eq!(app.range(), issue.range);
    app.spelling.checker.wait();
    recovery_click(&mut app, &ctx, "courrier");
    assert_eq!(
        app.document.engine.document().view.text,
        "Un courrier et une réservvation."
    );
    assert!(
        app.document.engine.document().view.styles[3..11]
            .iter()
            .all(|style| style.bold)
    );
    assert_eq!(app.document.engine.history().undo, 1);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), original);
    // Même mot et mêmes positions après annulation : le ticket ancien reste invalide.
    app.correct_spelling(&target, "courrier");
    assert_eq!(app.stored_document(), original);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.document.selection = editor::cursor(0..0);
    app.apply(Command::NextSpelling);
    app.apply(Command::SpellingSuggestions);
    app.spelling.checker.wait();
    let _ = search_draw(&mut app, &ctx, vec![]);
    let output = search_draw(&mut app, &ctx, vec![]);
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "courrier")), "les suggestions sont accessibles sans souris");
    let _ = search_draw(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    app.apply(Command::Redo);
    assert_eq!(
        app.document.engine.document().view.text,
        "Un courrier et une réservvation."
    );
}

#[test]
fn spelling_exceptions_are_scoped_persisted_and_undoable_without_affecting_typst() {
    let (ctx, mut app) = demo_app();
    let mut letter = StoredDocument::blank(DocumentType::Courrier);
    letter.source = "Courrrier courrrier réservvation".into();
    app.install_document(letter, None).unwrap();
    let original_main = app.document.engine.metadata().main_source();
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    assert_eq!(app.spelling.checker.issues.len(), 3);
    app.apply(Command::NextSpelling);
    let first = app.range();
    app.apply(Command::NextSpelling);
    assert_ne!(app.range(), first);
    app.apply(Command::PreviousSpelling);
    assert_eq!(app.range(), first);
    app.apply(Command::IgnoreSpelling);
    assert!(app.is_dirty());
    assert_eq!(app.document.engine.metadata().main_source(), original_main);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    assert_eq!(app.spelling.checker.issues.len(), 1);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orthographe.typsto");
    assert!(app.save_path(path.clone()));
    let stored = storage::read(&path).unwrap();
    assert_eq!(
        stored.metadata.options().spelling_ignored["fr"],
        std::collections::BTreeSet::from(["courrrier".into()])
    );
    app.apply(Command::Undo);
    assert!(
        app.document
            .engine
            .metadata()
            .options()
            .spelling_ignored
            .is_empty()
    );
    app.apply(Command::Redo);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.apply(Command::NextSpelling);
    app.apply(Command::LearnSpelling);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    assert!(app.spelling.checker.issues.is_empty());
    assert!(
        !app.is_dirty(),
        "apprendre un mot ne modifie pas le document"
    );
    let encoded = serde_json::to_string(&app.preferences).unwrap();
    let restored: Preferences = serde_json::from_str(&encoded).unwrap();
    assert!(restored.spelling.personal["fr"].contains("réservvation"));
    app.apply(Command::ResetSpelling);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    assert_eq!(app.spelling.checker.issues.len(), 2);
    assert!(
        app.set_document_option(crate::engine::DocumentOption::Language(
            DocumentLanguage::German
        ))
    );
    app.refresh_spelling(&ctx);
    assert!(app.spelling.checker.issues.is_empty() && !app.spelling.checker.ready);
    app.apply(Command::ToggleSpelling);
    assert!(!app.preferences.spelling.enabled);
    app.apply(Command::ToggleSpelling);
    let mut another = StoredDocument::blank(DocumentType::CompteRendu);
    another.source = stored.source.clone();
    another.template = "#let compte-rendu(..args) = []".into();
    app.install_document(another, None).unwrap();
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    assert_eq!(
        app.spelling.checker.issues.len(),
        2,
        "les mots ignorés ne fuient pas vers un autre document ; le dictionnaire personnel reste partagé"
    );
    app.install_document(stored, Some(path)).unwrap();
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    assert!(
        app.spelling.checker.issues.is_empty(),
        "la réouverture réapplique les deux niveaux d’exceptions"
    );
}
