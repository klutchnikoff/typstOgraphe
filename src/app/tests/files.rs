use super::*;

#[test]
fn welcome_creation_and_close_use_the_catalog_without_an_implicit_document() {
    let ctx = egui::Context::default();
    let mut app = App::with_context(ctx.clone(), None);
    let dir = tempfile::tempdir().unwrap();
    app.start_recovery(dir.path().join("recovery"), ctx.clone());
    frame(&mut app, &ctx, vec![]);
    assert!(!app.document_open && !app.is_dirty());
    assert_eq!(app.window_title, "typstOgraphe");
    assert_eq!(app.submitted, 0);
    assert!(app.pages.is_empty() && app.rendered_revision.is_none());
    assert!(app.recovery_entries.is_empty());
    let original = app.stored_document();
    for command in [
        Command::Bold,
        Command::Find,
        Command::ImportModel,
        Command::File(FileCommand::Save),
        Command::File(FileCommand::Export),
        Command::Model(Model::Personal),
    ] {
        app.apply(command);
    }
    assert_eq!(app.stored_document(), original);
    assert!(!app.search_open && !app.export_open && app.file_command.is_none());
    for (command, status) in app.menu_statuses() {
        if !command.available_without_document() {
            assert!(!status.enabled);
        }
    }
    app.apply(Command::Preferences);
    assert!(app.preferences_open);
    app.preferences_open = false;
    recovery_click(&mut app, &ctx, &Command::File(FileCommand::New).label());
    assert!(app.new_document_open && !app.document_open);
    assert_eq!(app.new_document_model, Some(Model::Personal));
    recovery_click(&mut app, &ctx, &crate::tr!("cancel"));
    assert!(!app.new_document_open && !app.document_open);
    app.perform_action(PendingAction::New, &ctx);
    recovery_click(&mut app, &ctx, &crate::tr!("new-document-create"));
    assert!(app.document_open && !app.new_document_open);
    assert!(app.session.document.source().is_empty());
    assert_eq!(app.session.template, Model::Personal.source());
    let path = dir.path().join("courrier.typsto");
    assert!(app.save_path(path.clone()));
    app.handle_file_command(FileCommand::Close, &ctx);
    assert!(!app.document_open && !app.allow_close);
    assert!(app.open_path(path));
    assert!(app.document_open);
    app.session.document.view.insert_text("À conserver", 0);
    app.changed();
    app.handle_file_command(FileCommand::Close, &ctx);
    assert!(matches!(
        app.pending_action,
        Some(PendingAction::CloseDocument)
    ));
    recovery_click(&mut app, &ctx, &crate::tr!("cancel"));
    assert!(app.document_open && app.is_dirty());
    app.handle_file_command(FileCommand::Close, &ctx);
    recovery_click(&mut app, &ctx, &crate::tr!("discard"));
    assert!(!app.document_open && !app.is_dirty());
}

#[test]
fn egui_file_menu_exports_and_quit_preserves_unsaved_work_without_header_duplicates() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    app.session.file_path = Some(PathBuf::from("exemple.typsto"));
    let source = app.session.document.source().to_owned();
    let mut time = 0.0;
    let mut draw = |app: &mut App, events| {
        time += 0.1;
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1240.0, 900.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| app.ui_frame(ctx),
        )
    };
    let text_point = |output: &egui::FullOutput, value: &str| {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == value => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
    };
    draw(&mut app, vec![]);
    let output = draw(&mut app, vec![]);
    assert!(text_point(&output, "exemple.typsto").is_none());
    assert!(text_point(&output, "Enregistrer").is_none());
    assert!(text_point(&output, "Exporter").is_none());
    for label in ["Effacer", "Retirer la mise en forme", "Annuler", "Rétablir"] {
        assert!(
            text_point(&output, label).is_none(),
            "{label} reste dans la barre d’outils"
        );
    }
    assert!(app.window_title.contains("exemple.typsto"));
    let pos = text_point(&output, "Fichier").unwrap();
    for pressed in [true, false] {
        draw(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let output = draw(&mut app, vec![]);
    let pos = text_point(&output, &Command::File(FileCommand::Export).label())
        .expect("export dans Fichier");
    for pressed in [true, false] {
        draw(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(app.export_open);
    assert_eq!(app.session.document.source(), source);
    // Une commande native déjà en transit ne contourne pas un dialogue.
    app.dispatch_action(Command::File(FileCommand::Quit), &ctx);
    assert!(app.file_command.is_none());
    app.export_open = false;
    app.session.metadata.courrier_mut().objet = "Brouillon à conserver".into();
    let before = app.stored_document();
    app.dispatch_action(Command::File(FileCommand::Quit), &ctx);
    let output = draw(&mut app, vec![]);
    assert!(matches!(app.pending_action, Some(PendingAction::Close)));
    assert!(!app.allow_close);
    assert_eq!(app.stored_document(), before);
    assert!(
        output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .iter()
            .all(|command| !matches!(command, egui::ViewportCommand::Close))
    );
}

#[test]
fn save_reopen_restores_the_letter_and_failed_open_preserves_current_work() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("courrier.typsto");
    assert!(!app.is_dirty());
    app.selection = editor::cursor(20..35);
    app.apply(Command::Bold);
    assert!(app.is_dirty());
    app.apply(Command::Undo);
    assert!(!app.is_dirty());
    app.session.metadata.courrier_mut().objet = "Objet personnalisé".into();
    app.session.template = Model::Personal.source();
    app.session.template = app
        .session
        .template
        .replace("VILLE DE VALMONT", "MODÈLE ARCHIVÉ");
    app.session.document.view.insert_text("Élodie : ", 0);
    app.changed();
    let letter = app.stored_document();
    assert!(app.save_path(path.clone()));
    assert!(!app.is_dirty());
    app.session.document.view.insert_text("Brouillon ", 0);
    app.changed();
    assert!(app.is_dirty());
    assert!(app.open_path(path.clone()));
    assert_eq!(app.stored_document(), letter);
    assert_eq!(app.session.file_path, Some(path.clone()));
    assert!(!app.is_dirty());
    assert!(app.session.undo.is_empty() && app.session.redo.is_empty());
    assert_eq!(app.selection.primary.index, 0);
    assert!(app.pages.is_empty() && app.sync.is_none());
    assert_eq!(app.request(None).template, letter.template);
    app.session.document.view.insert_text("À conserver ", 0);
    app.changed();
    let before = app.stored_document();
    let bad = dir.path().join("illisible.typsto");
    std::fs::write(&bad, "invalide").unwrap();
    assert!(!app.open_path(bad));
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.session.file_path, Some(path.clone()));
    assert!(app.is_dirty());
    assert!(!app.save_path(dir.path().join("absent/impossible.typsto")));
    assert_eq!(app.session.file_path, Some(path.clone()));
    assert!(app.is_dirty());
    assert_eq!(storage::read(&path).unwrap(), letter);
}

#[test]
fn save_shortcut_and_save_as_keep_both_files_and_track_clean_state() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("premier.typsto");
    assert!(app.save_path(first.clone()));
    let initial = storage::read(&first).unwrap();
    app.session.document.view.insert_text("Un ajout. ", 0);
    app.changed();
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    assert!(!app.is_dirty());
    assert_ne!(storage::read(&first).unwrap(), initial);
    let saved = app.stored_document();
    app.session.metadata.courrier_mut().objet = "Deuxième courrier".into();
    let second = dir.path().join("second.typsto");
    assert!(app.save_path(second.clone()));
    assert_eq!(app.session.file_path, Some(second));
    assert_eq!(storage::read(&first).unwrap(), saved);
    assert!(!app.is_dirty());
}

#[test]
fn unsaved_work_blocks_closing_or_replacing_until_the_user_decides() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    app.session.metadata.courrier_mut().objet = "Travail à conserver".into();
    let before = app.stored_document();
    app.request_action(PendingAction::Open, &ctx);
    assert!(app.pending_action.is_some()); // Aucune fenêtre de fichier tant que la décision manque.
    assert_eq!(app.stored_document(), before);
    frame(
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
    assert!(app.pending_action.is_none());
    assert_eq!(app.stored_document(), before);

    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    let output = ctx.run(input, |ctx| app.ui_frame(ctx));
    assert!(
        output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
    );
    assert!(app.pending_action.is_some());
    assert_eq!(app.stored_document(), before);
    app.pending_action = None;
    app.request_action(PendingAction::New, &ctx);
    let draw = |app: &mut App, time, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1240.0, 900.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| app.ui_frame(ctx),
        )
    };
    draw(&mut app, 1.0, vec![]);
    let output = draw(&mut app, 1.1, vec![]);
    let pointer = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "Ne pas enregistrer" => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .expect("bouton de confirmation");
    for (i, pressed) in [true, false].into_iter().enumerate() {
        draw(
            &mut app,
            1.2 + i as f64 * 0.05,
            vec![
                egui::Event::PointerMoved(pointer),
                egui::Event::PointerButton {
                    pos: pointer,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(app.pending_action.is_none());
    assert!(app.new_document_open);
    app.create_document(DocumentType::Courrier);
    assert!(app.session.document.source().is_empty());
    assert!(app.session.file_path.is_none());
    assert!(!app.is_dirty());
}
