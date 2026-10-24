use super::*;

#[test]
fn recovery_dialog_defers_restores_and_saves_a_copy_without_overwriting_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("recovery");
    let original = dir.path().join("original.typsto");
    let saved = StoredDocument::demo(DocumentType::Courrier);
    storage::save(&original, &saved).unwrap();
    let mut unsaved = saved.clone();
    unsaved.source = "Un travail *non enregistré*.".into();
    unsaved.metadata.letter_mut().objet = "Nouvel objet".into();
    unsaved.template = Model::Personal.source();
    seed_recovery(&root, &unsaved, Some("original.typsto".into()));
    let (ctx, mut app) = demo_app();
    app.start_recovery(root.clone(), ctx.clone());
    assert!(app.recovery.open);
    recovery_click(&mut app, &ctx, &crate::tr!("recovery-later"));
    assert!(!app.recovery.open);
    assert_eq!(app.recovery.entries.len(), 1);
    app.apply(Command::RecoverDocuments);
    recovery_click(&mut app, &ctx, &crate::tr!("recovery-restore"));
    app.recovery.worker.as_mut().unwrap().wait();
    assert!(!app.recovery.open);
    assert!(app.recovery.entries.is_empty());
    assert!(app.is_dirty() && app.document.engine.is_recovered());
    assert!(app.document.engine.file_path().map(PathBuf::from).is_none());
    assert!(!app.document.engine.can_undo() && !app.document.engine.can_redo());
    assert_eq!(app.stored_document(), unsaved);
    assert!(
        app.recovered_filename()
            .unwrap()
            .contains("original-recupere")
    );
    app.request_action(PendingAction::Close, &ctx);
    assert!(app.file.pending_action.is_some());
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
    assert!(app.file.pending_action.is_none() && app.is_dirty());
    assert!(!app.save_path(dir.path().join("absent/fichier.typsto")));
    drop(app); // L’échec d’enregistrement laisse la récupération intacte.
    let mut app = App::with_demo_context(ctx.clone(), None);
    app.start_recovery(root.clone(), ctx.clone());
    assert_eq!(app.recovery.entries.len(), 1);
    app.recover_document(0);
    assert_eq!(app.stored_document(), unsaved);
    let copy = dir.path().join("recupere.typsto");
    assert!(app.save_path(copy.clone()));
    app.recovery.worker.as_mut().unwrap().wait();
    assert!(!app.is_dirty() && !app.document.engine.is_recovered());
    assert_eq!(storage::read(&copy).unwrap(), unsaved);
    assert_eq!(storage::read(&original).unwrap(), saved);
    drop(app);
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn explicit_delete_and_discard_remove_recovery_but_cancelling_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("recovery");
    let saved = StoredDocument::demo(DocumentType::Courrier);
    seed_recovery(&root, &saved, None);
    let (ctx, mut app) = demo_app();
    app.start_recovery(root.clone(), ctx.clone());
    recovery_click(&mut app, &ctx, &crate::tr!("recovery-delete"));
    assert!(app.recovery.entries.is_empty());
    assert!(!app.recovery.open);
    insert_fixture_text(&mut app, "À conserver. ");
    app.recovery.worker.as_mut().unwrap().make_due();
    app.checkpoint_recovery();
    app.recovery.worker.as_mut().unwrap().wait();
    app.request_action(PendingAction::Close, &ctx);
    recovery_click(&mut app, &ctx, &crate::tr!("cancel"));
    assert!(app.file.pending_action.is_none() && app.is_dirty());
    app.request_action(PendingAction::Close, &ctx);
    recovery_click(&mut app, &ctx, &crate::tr!("discard"));
    assert!(app.file.allow_close);
    drop(app);
    let (recovery, entries) =
        crate::recovery::Recovery::start(&root, std::sync::Arc::new(move || ctx.request_repaint()))
            .unwrap();
    assert!(entries.is_empty());
    drop(recovery);
}
