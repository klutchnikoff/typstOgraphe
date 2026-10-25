use super::*;

#[test]
fn sharing_is_explicit_palette_available_and_does_not_follow_later_user_navigation() {
    let (ctx, mut app) = demo_app();
    app.document.selection = editor::cursor(0..6);
    app.sync_published_selection();
    assert!(!app.command_enabled(Command::ShareSelection));
    app.start_assistance(&ctx);
    let before = app.stored_document();
    let user = app.document.engine.primary_context();
    app.dialogs.palette_open = true;
    assert!(app.command_enabled(Command::ShareSelection));
    app.dialogs.palette_open = false;
    app.apply(Command::ShareSelection);
    assert_eq!(app.document.engine.primary_context(), user);
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.engine.history().undo, 0);
    app.assistance.native_ime = true;
    assert!(!app.command_enabled(Command::ShareSelection));
    app.assistance.native_ime = false;
    app.document.selection = editor::cursor(8..8);
    app.sync_published_selection();
    assert!(!app.command_enabled(Command::ShareSelection));
}

#[test]
fn file_grants_are_visible_and_save_effects_clear_recovery_without_taking_focus() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("recovery");
    let path = dir.path().join("letter.typsto");
    let (ctx, mut app) = demo_app();
    assert!(app.save_path(path.clone()));
    app.start_recovery(root.clone(), ctx.clone());
    insert_fixture_text(&mut app, "À enregistrer par l’agent. ");
    app.recovery.worker.as_mut().unwrap().make_due();
    app.checkpoint_recovery();
    app.recovery.worker.as_mut().unwrap().wait();
    app.apply(Command::Assistance);
    assert!(!app.assistance.save_current && app.assistance.export_root.is_none());
    recovery_click(&mut app, &ctx, &crate::tr!("assistance-save-allow"));
    assert!(app.assistance.save_current);
    recovery_click(&mut app, &ctx, &crate::tr!("assistance-authorize"));
    let permission = app
        .assistance
        .session
        .as_ref()
        .unwrap()
        .file_permissions(&app.document.engine);
    assert_eq!(
        permission["current_file"],
        serde_json::json!(path.canonicalize().unwrap())
    );
    let user = app.document.engine.primary_context();
    let selection = app.document.selection;
    let focus = egui::Id::new("retained-field");
    ctx.memory_mut(|m| m.request_focus(focus));
    app.document.force_selection = false;
    let outcome = app
        .document
        .engine
        .run(crate::engine::Command::SaveDocument(
            crate::engine::SaveDocumentParams {
                guard: app.document.engine.target_guard(),
                path: None,
            },
        ))
        .unwrap();
    app.assistance_file_event(typstographe_mcp::local::FileEvent::Saved(outcome));
    app.recovery.worker.as_mut().unwrap().wait();
    assert_eq!(ctx.memory(|m| m.focused()), Some(focus));
    assert_eq!(app.document.engine.primary_context(), user);
    assert_eq!(app.document.selection, selection);
    assert!(!app.document.force_selection && !app.is_dirty());
    drop(app); // The active recovery slot retains its lock until the worker exits.
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn assistance_is_opt_in_and_its_public_ui_command_targets_the_open_document() {
    let ctx = egui::Context::default();
    let mut app = App::with_context(ctx.clone(), None);
    assert!(app.assistance.session.is_none());
    assert!(!app.command_enabled(Command::Assistance));
    app.load_demo();
    assert!(app.command_enabled(Command::Assistance));
    app.apply(Command::Assistance);
    assert!(app.dialogs.assistance_open);
    assert!(app.modal_open());
    recovery_click(&mut app, &ctx, &crate::tr!("assistance-authorize"));
    let path = app
        .assistance
        .session
        .as_ref()
        .unwrap()
        .invitation_path()
        .to_path_buf();
    assert!(path.exists());
    recovery_click(&mut app, &ctx, &crate::tr!("assistance-copy"));
    let output = ctx.run(frame_input(vec![]), |ctx| app.ui_frame(ctx));
    assert!(!output.shapes.is_empty());
    recovery_click(&mut app, &ctx, &crate::tr!("assistance-revoke"));
    assert!(app.assistance.session.is_none());
    assert!(!path.exists());
    assert!(!app.document.engine.document().source().is_empty());
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
    assert!(!app.dialogs.assistance_open);
}

#[test]
fn replacing_or_closing_the_document_revokes_the_invitation_without_following_the_new_one() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    app.start_assistance(&ctx);
    let path = app
        .assistance
        .session
        .as_ref()
        .unwrap()
        .invitation_path()
        .to_path_buf();
    app.load_demo();
    app.poll_assistance();
    assert!(app.assistance.session.is_none());
    assert!(!path.exists());
    app.start_assistance(&ctx);
    let path = app
        .assistance
        .session
        .as_ref()
        .unwrap()
        .invitation_path()
        .to_path_buf();
    app.execute_document_command(crate::engine::Command::CloseDocument(
        crate::engine::CloseDocumentParams {
            guard: app.document.engine.target_guard(),
            discard_changes: true,
        },
    ))
    .unwrap();
    app.poll_assistance();
    assert!(app.assistance.session.is_none());
    assert!(!path.exists());
}
