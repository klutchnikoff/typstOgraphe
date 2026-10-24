use super::*;

#[test]
fn creation_and_template_import_preserve_the_type_and_use_unified_history() {
    let ctx = egui::Context::default();
    let mut app = App::with_context(ctx.clone(), None);
    app.perform_action(PendingAction::New, &ctx);
    frame(&mut app, &ctx, vec![]);
    assert!(app.dialogs.new_document_open && !app.document.open);
    app.create_document(DocumentType::CompteRendu);
    assert!(app.file.error.is_some() && !app.document.open);
    app.file.error = None;
    app.create_document(DocumentType::Courrier);
    assert!(app.document.open && !app.dialogs.new_document_open);
    assert_eq!(
        app.keymap.registry().document_type(),
        DocumentType::Courrier
    );
    assert!(app.document.engine.document().source().is_empty());
    assert!(!app.is_dirty());
    assert_eq!(app.document.engine.template(), Model::Personal.source());
    let before = app.stored_document();
    assert!(
        app.install_model("#let compte-rendu(..args) = []".into())
            .is_err()
    );
    assert_eq!(app.stored_document(), before);
    let template = format!(
        "{}\n// Modèle fourni par l’organisation",
        app.document.engine.template()
    );
    app.install_model(template.clone()).unwrap();
    assert_eq!(app.document.engine.template(), template);
    assert!(app.is_dirty());
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
    assert!(!app.is_dirty());
    app.apply(Command::Redo);
    assert_eq!(app.document.engine.template(), template);
    assert_eq!(
        app.document.engine.metadata().document_type(),
        DocumentType::Courrier
    );
}

#[test]
fn information_and_presentations_work_from_palette_and_custom_shortcuts() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.template.push_str("\n// modèle embarqué");
    });
    let source = app.document.engine.document().source().to_owned();
    let revision = app.document.engine.revision();
    app.apply(Command::Palette);
    app.dialogs.palette_query = "Informations du document".into();
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            modifiers: egui::Modifiers::NONE,
            pressed: true,
            repeat: false,
            physical_key: None,
        }],
    );
    assert!(app.dialogs.information_open);
    assert!(!app.dialogs.palette_open);
    assert_eq!(app.document.engine.revision(), revision);
    assert!(!app.is_dirty());
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            modifiers: egui::Modifiers::NONE,
            pressed: true,
            repeat: false,
            physical_key: None,
        }],
    );
    app.apply(Command::Palette);
    app.dialogs.palette_query = "modèle de courrier personnel".into();
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            modifiers: egui::Modifiers::NONE,
            pressed: true,
            repeat: false,
            physical_key: None,
        }],
    );
    assert_eq!(app.document.engine.template(), Model::Personal.source());
    assert!(app.is_dirty());
    assert_eq!(app.document.engine.revision(), revision + 1);
    assert_eq!(app.stored_document().template, Model::Personal.source());
    assert_eq!(app.document.engine.document().source(), source);
    app.apply(Command::Model(Model::Personal));
    assert_eq!(app.document.engine.revision(), revision + 1); // Choisir la présentation active est sans effet.

    app.apply(Command::Undo);
    let config = serde_json::from_str(
        r#"{"document.information":[{"key":"F6"}],"courrier.model.personal":[{"key":"F7"}]}"#,
    )
    .unwrap();
    app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &config).unwrap();
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::F7,
            modifiers: egui::Modifiers::NONE,
            pressed: true,
            repeat: false,
            physical_key: None,
        }],
    );
    assert_eq!(app.document.engine.template(), Model::Personal.source());
    assert_eq!(app.document.engine.revision(), revision + 3);
    assert!(app.is_dirty());
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::F6,
            modifiers: egui::Modifiers::NONE,
            pressed: true,
            repeat: false,
            physical_key: None,
        }],
    );
    assert!(app.dialogs.information_open);
    assert_eq!(app.document.engine.document().source(), source);
}

#[test]
fn remove_link_keeps_other_styles_and_selection_and_can_be_undone() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx, None);
    fixture(&mut app, |data| {
        data.document = Document::from_source(
            "Avant #link(\"https://example.org\")[*Élodie* et _Noël_] après.",
        )
        .unwrap();
    });
    let source = app.document.engine.document().source().to_owned();
    let start = 6;
    let end_byte = app
        .document
        .engine
        .document()
        .view
        .text
        .find(" après.")
        .unwrap();
    let end = app.document.engine.document().view.text[..end_byte]
        .chars()
        .count();
    app.document.selection = editor::cursor(end..start);
    let selection = app.document.selection;
    let text = app.document.engine.document().view.text.clone();
    let styles = app.document.engine.document().view.styles.clone();
    app.apply(Command::RemoveLink);
    assert_eq!(app.document.selection, selection);
    assert_eq!(app.document.engine.document().view.text, text);
    for (before, after) in styles
        .iter()
        .zip(&app.document.engine.document().view.styles)
    {
        assert_eq!(before.bold, after.bold);
        assert_eq!(before.italic, after.italic);
        assert!(after.link.is_none());
    }
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document().source(), source);
    assert_eq!(app.document.selection, selection);
    app.apply(Command::Redo);
    assert!(
        app.document
            .engine
            .document()
            .view
            .styles
            .iter()
            .all(|style| style.link.is_none())
    );
}

#[test]
fn shortcut_override_updates_dispatch_and_hints_without_changing_the_document() {
    let (ctx, mut app) = demo_app();
    frame(&mut app, &ctx, vec![]);
    let command = Command::File(FileCommand::Export);
    let original = app.keymap.hint(&ctx, command);
    let config = serde_json::from_str(r#"{"document.export":[{"key":"F6"}]}"#).unwrap();
    app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &config).unwrap();
    let hint = app.keymap.hint(&ctx, command);
    assert_ne!(hint, original);
    assert!(hint.contains("F6"));
    let source = app.document.engine.document().source().to_owned();
    for (key, modifiers) in [
        (egui::Key::E, egui::Modifiers::COMMAND),
        (egui::Key::F6, egui::Modifiers::NONE),
    ] {
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
        );
        assert_eq!(app.file.export_open, key == egui::Key::F6);
    }
    assert_eq!(app.document.engine.document().source(), source);
    assert!(!app.is_dirty());
}

#[test]
fn cross_paragraph_formatting_undo_restores_source_and_selection() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx, None);
    let source = app.document.engine.document().source().to_owned();
    app.document.selection = editor::cursor(3..180);
    let selection = app.document.selection;
    app.apply(Command::Bold);
    assert!(
        app.document
            .engine
            .document()
            .view
            .mark_state(3..180, Mark::Bold)
            .0
    );
    assert_eq!(app.document.selection, selection);
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document().source(), source);
    assert_eq!(app.document.selection, selection);
    app.apply(Command::Redo);
    assert!(
        app.document
            .engine
            .document()
            .view
            .mark_state(3..180, Mark::Bold)
            .0
    );
}

#[test]
fn changing_model_restores_automatic_markers_without_altering_the_body() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx, None);
    fixture(&mut app, |data| {
        data.template.push_str("\n// modèle personnalisé");
    });
    let source = app.document.engine.document().source().to_owned();
    fixture(&mut app, |data| {
        data.metadata.options_mut().bullet_marker = Some("◆".into());
    });
    app.document.presentation.custom_bullet_marker = true;
    app.apply(Command::Model(Model::Personal));
    assert!(
        app.document
            .engine
            .metadata()
            .options()
            .bullet_marker
            .is_none()
    );
    assert_eq!(app.document.presentation, ModelPresentation::default());
    assert_eq!(app.document.engine.document().source(), source);
}

#[test]
fn model_dependencies_follow_imports_choices_and_undo_without_changing_the_body() {
    let mut app = App::with_demo_context(egui::Context::default(), None);
    let original = app.document.engine.dependencies().clone();
    assert!(!original.is_empty());
    let body = app.document.engine.document().source().to_owned();
    app.install_model(app.document.engine.template().to_owned() + "\n// Modèle dérivé")
        .unwrap();
    assert!(app.document.engine.dependencies() == &original);
    app.install_model("#let courrier(..args) = args.pos().last()".into())
        .unwrap();
    assert!(app.document.engine.dependencies().is_empty());
    assert!(
        app.document
            .engine
            .preview_request()
            .unwrap()
            .dependencies
            .is_empty()
    );
    app.apply(Command::Undo);
    assert!(app.document.engine.dependencies() == &original);
    assert!(app.stored_document().dependencies == original);
    app.apply(Command::Redo);
    assert!(app.document.engine.dependencies().is_empty());
    app.apply(Command::Model(Model::Personal));
    assert!(app.document.engine.dependencies() == &original);
    app.apply(Command::Undo);
    assert!(app.document.engine.dependencies().is_empty());
    assert_eq!(app.document.engine.document().source(), body);
}
