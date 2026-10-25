use super::*;
fn prepared() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut app = App::with_context(ctx.clone(), None);
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.source = "Cheres collègues,\n\nMerci pour votre retour.".into();
    app.install_document(stored, None).unwrap();
    app.start_assistance(&ctx);
    let step = crate::engine::Call {
        id: "text.insert".to_owned().try_into().unwrap(),
        params: serde_json::json!({"selector":{"select":"Cheres"},"text":"Chères"}),
    };
    app.assistance
        .session
        .as_mut()
        .unwrap()
        .prepare_proposal(&app.document.engine, "correction", vec![step])
        .unwrap();
    assert!(app.poll_assistance());
    assert!(!app.poll_assistance());
    (ctx, app)
}
#[test]
fn notification_and_review_never_change_the_document_and_escape_only_returns_to_writing() {
    let (ctx, mut app) = prepared();
    let before = app.stored_document();
    let guard = app.document.engine.target_guard();
    assert!(!app.review_open());
    assert!(app.command_enabled(Command::ReviewProposal));
    recovery_click(&mut app, &ctx, &Command::ReviewProposal.label());
    assert!(app.review_open());
    assert!(!app.command_enabled(Command::Bold));
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
    // The initial focus is the return button, never approval.
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.engine.target_guard(), guard);
    if !app.review_open() {
        app.apply(Command::ReviewProposal);
    }
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
    assert!(!app.review_open());
    assert!(app.assistance.review.is_some());
    assert_eq!(app.document.engine.history().undo, 0);
}
#[test]
fn human_acceptance_keeps_user_selection_style_focus_and_creates_one_undo() {
    let (ctx, mut app) = prepared();
    let end = app.document.engine.document().view.len();
    app.document.selection = editor::cursor(end..end);
    app.sync_published_selection();
    app.apply(Command::Italic);
    let style = app.document.engine.primary_context().unwrap().typing_style;
    let before = app.stored_document();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(editor::EDITOR_ID)));
    frame(&mut app, &ctx, vec![]);
    app.apply(Command::ReviewProposal);
    recovery_click(&mut app, &ctx, &Command::AcceptProposal.label());
    assert!(!app.review_open());
    assert!(app.assistance.review.is_none());
    assert!(
        app.document
            .engine
            .document()
            .view
            .text
            .starts_with("Chères")
    );
    assert_eq!(app.document.selection.as_sorted_char_range(), end..end);
    assert_eq!(
        app.document.engine.primary_context().unwrap().typing_style,
        style
    );
    assert_eq!(app.document.engine.history().undo, 1);
    assert!(app.command_enabled(Command::RevealAgentChange));
    let user = app.document.engine.primary_context();
    app.apply(Command::RevealAgentChange);
    assert_eq!(app.document.engine.primary_context(), user);
    assert!(app.preview.sync.reveal_target().is_some());
    app.assistance.reveal_until = Some(Instant::now() - Duration::from_secs(1));
    frame(&mut app, &ctx, vec![]);
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
    assert!(app.preview.sync.reveal_target().is_none());
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
    app.apply(Command::Redo);
    assert!(
        app.document
            .engine
            .document()
            .view
            .text
            .starts_with("Chères")
    );
}
#[test]
fn rejection_stale_revision_and_modal_or_ime_guards_prevent_acceptance() {
    let (ctx, mut app) = prepared();
    let before = app.stored_document();
    app.bands.parameter_ime = true;
    assert!(!app.command_enabled(Command::ReviewProposal));
    app.bands.parameter_ime = false;
    app.apply(Command::ReviewProposal);
    app.dialogs.preferences_open = true;
    assert!(!app.command_enabled(Command::AcceptProposal));
    app.apply(Command::AcceptProposal);
    assert_eq!(app.stored_document(), before);
    app.dialogs.preferences_open = false;
    recovery_click(&mut app, &ctx, &Command::RejectProposal.label());
    assert!(app.assistance.review.is_none());
    assert_eq!(app.document.engine.history().undo, 0);
    assert_eq!(app.stored_document(), before);
    let (ctx, mut app) = prepared();
    app.apply(Command::ReviewProposal);
    app.apply(Command::CloseReview);
    app.execute_document_command(
        crate::engine::Command::from_call("text.insert", serde_json::json!({"text":"Un "}))
            .unwrap(),
    )
    .unwrap();
    app.poll_assistance();
    app.apply(Command::ReviewProposal);
    assert!(app.review_open());
    assert!(!app.command_enabled(Command::AcceptProposal));
    app.apply(Command::AcceptProposal);
    assert!(
        app.document
            .engine
            .document()
            .view
            .text
            .starts_with("Un Cheres")
    );
    recovery_click(&mut app, &ctx, &Command::RejectProposal.label());
    assert!(app.assistance.review.is_none());
    assert_eq!(app.document.engine.history().undo, 1);
}
#[test]
fn invitation_revocation_drops_review_without_publishing() {
    let (ctx, mut app) = prepared();
    let before = app.stored_document();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(editor::EDITOR_ID)));
    frame(&mut app, &ctx, vec![]);
    app.apply(Command::ReviewProposal);
    frame(&mut app, &ctx, vec![]);
    app.assistance.session = None;
    app.poll_assistance();
    frame(&mut app, &ctx, vec![]);
    assert!(!app.review_open());
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.engine.history().undo, 0);
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
}

#[test]
fn rendering_an_accepted_lot_does_not_follow_the_agent_or_reset_the_origin() {
    let (_, mut app) = prepared();
    let old = app
        .document
        .engine
        .compile(crate::engine::GuardParams {
            guard: app.document.engine.target_guard(),
        })
        .unwrap()
        .render
        .unwrap();
    app.install_preview_render(old);
    app.preview.sync.preview_actions(&app.document.engine);
    app.apply(Command::ReviewProposal);
    app.apply(Command::AcceptProposal);
    // A routine frame publication must not turn an agent result into a User fact.
    app.sync_published_selection();
    let render = app
        .document
        .engine
        .compile(crate::engine::GuardParams {
            guard: app.document.engine.target_guard(),
        })
        .unwrap()
        .render
        .unwrap();
    app.install_preview_render(render);
    assert!(
        !app.preview
            .sync
            .preview_actions(&app.document.engine)
            .iter()
            .any(|action| matches!(action, ViewAction::RevealInPreview { .. }))
    );
}

#[test]
fn acceptance_preserves_the_scrolled_passage_after_an_insertion_above_it() {
    let ctx = egui::Context::default();
    let mut app = App::with_context(ctx.clone(), None);
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.source=(0..45).map(|i|format!("Passage {i} : un paragraphe destiné à vérifier le maintien du défilement après insertion.")).collect::<Vec<_>>().join("\n\n");
    app.install_document(stored, None).unwrap();
    app.start_assistance(&ctx);
    app.assistance
        .session
        .as_mut()
        .unwrap()
        .prepare_proposal(
            &app.document.engine,
            "prefix",
            vec![crate::engine::Call {
                id: "text.insert".to_owned().try_into().unwrap(),
                params: serde_json::json!({"text":"Une précision ajoutée en tête.\n\n"}),
            }],
        )
        .unwrap();
    for _ in 0..4 {
        frame(&mut app, &ctx, vec![]);
    }
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::PointerMoved(egui::pos2(200.0, 500.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -700.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..15 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.preview.sync.visible_position().unwrap();
    assert!(app.document.engine.resolve_target(before).unwrap().1 > 100);
    app.apply(Command::ReviewProposal);
    app.apply(Command::AcceptProposal);
    let expected = app
        .document
        .engine
        .position_map()
        .unwrap()
        .map_retained(before)
        .unwrap();
    for _ in 0..15 {
        frame(&mut app, &ctx, vec![]);
    }
    assert_eq!(app.preview.sync.visible_position(), Some(expected));
}
