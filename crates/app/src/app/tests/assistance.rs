use super::*;

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
