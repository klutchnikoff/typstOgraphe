use super::*;
use egui::{Event, ImeEvent};

fn editing(source: &str, range: Range<usize>) -> (egui::Context, App) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source(source).unwrap();
    });
    app.document
        .engine
        .update_adapter_cursor(editor::to_core(app.document.selection))
        .unwrap();
    app.document.selection = editor::cursor(range);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    (ctx, app)
}

fn ime(app: &mut App, ctx: &egui::Context, events: Vec<ImeEvent>) {
    frame(app, ctx, events.into_iter().map(Event::Ime).collect());
}

fn start(app: &mut App, ctx: &egui::Context) {
    ime(app, ctx, vec![ImeEvent::Enabled]);
    ime(app, ctx, vec![ImeEvent::Preedit("にほん".into())]);
}

fn key(key: egui::Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn preedit_survives_frames_and_is_visible_without_publishing_any_session_state() {
    let (ctx, mut app) = editing("Un *été* ici.", 3..6);
    let stored = app.stored_document();
    let view = app.document.engine.document().view.clone();
    let selection = app.document.selection;
    let target = app
        .document
        .engine
        .selection_target(editor::to_core(selection))
        .unwrap();
    let revision = app.document.engine.revision();
    let changed_at = app.document.engine.changed_at();
    let source_map = app.document.engine.document().source_map.clone();
    start(&mut app, &ctx);
    for text in ["にほん", "日本語", "", "日本"] {
        let output = search_draw(
            &mut app,
            &ctx,
            vec![Event::Ime(ImeEvent::Preedit(text.into()))],
        );
        let expected = format!("Un {text} ici.");
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape, egui::Shape::Text(text) if text.galley.text() == expected
            )),
            "préédition absente : {expected}"
        );
        frame(&mut app, &ctx, vec![]);
        assert_eq!(
            app.document.ime.draft.as_ref().unwrap().input.view.text,
            expected
        );
        assert_eq!(app.document.engine.document().view, view);
        assert_eq!(app.document.engine.document().source_map, source_map);
        assert!(app.stored_document() == stored);
        assert_eq!(app.document.selection, selection);
        assert_eq!(app.document.engine.revision(), revision);
        assert_eq!(app.document.engine.changed_at(), changed_at);
        assert_eq!(app.document.engine.resolve_target(target).unwrap(), (3, 6));
        assert!(!app.document.engine.can_undo() && !app.document.engine.can_redo());
        assert!(!app.is_dirty());
        let counts = app
            .document
            .engine
            .statistics(crate::engine::StatisticsParams {
                target: Some(target.into()),
            })
            .unwrap();
        assert_eq!(
            counts.document,
            crate::statistics::TextStatistics::count(&view.text)
        );
        assert_eq!(
            counts.target,
            Some(crate::statistics::TextStatistics::count("été"))
        );
    }
    ime(&mut app, &ctx, vec![ImeEvent::Commit("日本".into())]);
    assert_eq!(
        app.document.engine.document().source(),
        "Un #strong[日本] ici."
    );
    assert_eq!(app.document.selection.as_sorted_char_range(), 5..5);
    assert_eq!(app.document.engine.revision(), revision + 1);
    assert_eq!(app.document.engine.history().undo, 1);
    assert!(app.document.ime.draft.is_none());
    app.apply(Command::Undo);
    assert!(app.stored_document() == stored);
    assert_eq!(app.document.selection, selection);
    app.apply(Command::Redo);
    assert_eq!(
        app.document.engine.document().source(),
        "Un #strong[日本] ici."
    );
}

#[test]
fn commit_without_preedit_and_same_frame_composition_support_dead_keys_and_unicode() {
    for events in [
        vec![ImeEvent::Commit("ê, é, Noël".into())],
        vec![
            ImeEvent::Enabled,
            ImeEvent::Commit("ê, é, Noël".into()),
            ImeEvent::Disabled,
        ],
        vec![
            ImeEvent::Enabled,
            ImeEvent::Preedit("e".into()),
            ImeEvent::Commit("ê, é, Noël".into()),
            ImeEvent::Disabled,
        ],
    ] {
        let (ctx, mut app) = editing("Avant après", 6..6);
        ime(&mut app, &ctx, events);
        assert_eq!(
            app.document.engine.document().view.text,
            "Avant ê, é, Noëlaprès"
        );
        assert_eq!(app.document.engine.history().undo, 1);
        assert!(app.document.ime.draft.is_none());
    }
    let (ctx, mut app) = editing("", 0..0);
    ime(&mut app, &ctx, vec![ImeEvent::Enabled]);
    frame(&mut app, &ctx, vec![Event::Text("Saisie directe".into())]);
    assert_eq!(app.document.engine.document().view.text, "Saisie directe");
}

#[test]
fn composition_is_one_history_step_separate_from_surrounding_typing() {
    let (ctx, mut app) = editing("", 0..0);
    frame(&mut app, &ctx, vec![Event::Text("A".into())]);
    start(&mut app, &ctx);
    ime(&mut app, &ctx, vec![ImeEvent::Commit("日本".into())]);
    frame(&mut app, &ctx, vec![Event::Text("B".into())]);
    assert_eq!(app.document.engine.history().undo, 3);
    for expected in ["A日本", "A", ""] {
        app.apply(Command::Undo);
        assert_eq!(app.document.engine.document().view.text, expected);
    }
}

#[test]
fn cancellation_restores_selected_text_and_keeps_redo_and_following_typing() {
    for terminal in [
        Event::Ime(ImeEvent::Disabled),
        Event::Ime(ImeEvent::Commit(String::new())),
        key(egui::Key::Escape),
    ] {
        let (ctx, mut app) = editing("Avant après", 0..5);
        frame(&mut app, &ctx, vec![Event::Text("Remplacé".into())]);
        app.apply(Command::Undo);
        frame(&mut app, &ctx, vec![]);
        let stored = app.stored_document();
        let revision = app.document.engine.revision();
        start(&mut app, &ctx);
        frame(&mut app, &ctx, vec![terminal]);
        assert!(app.stored_document() == stored);
        assert_eq!(app.document.selection.as_sorted_char_range(), 0..5);
        assert_eq!(app.document.engine.revision(), revision);
        assert!(!app.document.engine.can_undo());
        assert_eq!(app.document.engine.history().redo, 1);
        assert!(app.document.ime.draft.is_none());
        frame(&mut app, &ctx, vec![Event::Text("Été".into())]);
        ime(&mut app, &ctx, vec![ImeEvent::Commit("ê".into())]);
        assert_eq!(app.document.engine.document().view.text, "Étéê après");
    }
}

#[test]
fn composition_keeps_zone_identity_and_uses_shared_boundary_validation() {
    for (source, range, expected) in [
        ("/ Mot: Définition", 0..3, "/ 日本: Définition"),
        (
            "#quote(block: true, attribution: [Auteur])[Texte]",
            6..12,
            "#quote(block: true, attribution: [日本])[Texte]",
        ),
        (
            "#figure(image(\"images/parc.svg\"), caption: [Cap])",
            1..4,
            "#figure(image(\"images/parc.svg\", alt: none), caption: [日本],)",
        ),
    ] {
        let (ctx, mut app) = editing(source, range);
        let target = app
            .document
            .engine
            .selection_target(editor::to_core(app.document.selection))
            .unwrap();
        start(&mut app, &ctx);
        ime(&mut app, &ctx, vec![ImeEvent::Commit("日本".into())]);
        assert_eq!(app.document.engine.document().source(), expected);
        let after = app
            .document
            .engine
            .selection_target(editor::to_core(app.document.selection))
            .unwrap();
        let crate::document::structure::targets::Position::Text(before) = target.selection.anchor
        else {
            panic!("zone textuelle attendue");
        };
        let crate::document::structure::targets::Position::Text(after) = after.selection.anchor
        else {
            panic!("zone textuelle attendue");
        };
        assert_eq!(before.zone_id, after.zone_id);
        assert_eq!(after.offset, 2);
    }
}

#[test]
fn invalid_preedit_or_cross_zone_selection_cannot_publish_a_partial_replacement() {
    for (source, range, preedit) in [
        ("/ Mot: Définition", 2..6, "日本"),
        ("/ Mot: Définition", 0..3, "Deux\nlignes"),
        (
            "#figure(image(\"images/parc.svg\"), caption: [Cap])",
            1..4,
            "Deux\tlignes",
        ),
    ] {
        let (ctx, mut app) = editing(source, range.clone());
        let stored = app.stored_document();
        let revision = app.document.engine.revision();
        ime(
            &mut app,
            &ctx,
            vec![ImeEvent::Enabled, ImeEvent::Preedit(preedit.into())],
        );
        ime(&mut app, &ctx, vec![ImeEvent::Commit("Correct".into())]);
        assert!(app.stored_document() == stored);
        assert_eq!(app.document.engine.revision(), revision);
        assert_eq!(app.document.selection.as_sorted_char_range(), range);
        assert!(!app.document.engine.can_undo());
        assert!(app.dialogs.message.is_some());
    }
}

#[test]
fn commands_and_new_documents_cancel_composition_and_ignore_its_late_commit() {
    let (ctx, mut app) = editing("Texte", 0..5);
    start(&mut app, &ctx);
    app.apply(Command::Find);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    ime(&mut app, &ctx, vec![ImeEvent::Commit("Ancien".into())]);
    assert_eq!(app.document.engine.document().source(), "Texte");
    assert!(app.document.ime.draft.is_none());
    assert!(app.search.engine.query.is_empty());
    app.close_search();
    frame(&mut app, &ctx, vec![]);
    start(&mut app, &ctx);
    app.close_document();
    app.load_demo();
    let stored = app.stored_document();
    ime(&mut app, &ctx, vec![ImeEvent::Commit("Ancien".into())]);
    assert!(app.stored_document() == stored);
    assert!(!app.document.engine.can_undo());
}

#[test]
fn focus_loss_and_stale_drafts_never_restore_over_a_newer_commit() {
    let (ctx, mut app) = editing("Texte", 0..5);
    start(&mut app, &ctx);
    ctx.memory_mut(|memory| memory.surrender_focus(egui::Id::new(editor::EDITOR_ID)));
    frame(&mut app, &ctx, vec![]);
    assert!(app.document.ime.draft.is_none());
    editor::restore_cursor(&ctx, app.document.selection);
    ime(&mut app, &ctx, vec![ImeEvent::Commit("Ancien".into())]);
    assert_eq!(app.document.engine.document().source(), "Texte");
    start(&mut app, &ctx);
    let target = app
        .document
        .engine
        .selection_target(editor::to_core(app.document.selection))
        .unwrap();
    app.document
        .engine
        .run(crate::engine::Command::InsertText(
            crate::engine::InsertTextParams {
                target: target.into(),
                text: "Récent".into(),
                input_rules: crate::engine::InputRules::Literal,
            },
        ))
        .unwrap();
    let selection = app.document.engine.selection();
    app.document.selection = editor::to_editor(selection);
    ime(&mut app, &ctx, vec![ImeEvent::Commit("Ancien".into())]);
    assert_eq!(app.document.engine.document().source(), "Récent");
    assert_eq!(app.document.engine.history().undo, 1);
    assert_eq!(app.document.selection.as_sorted_char_range(), 6..6);
}

#[test]
fn preedit_enter_tab_and_literal_shortcuts_do_not_escape_into_document_edits() {
    let (ctx, mut app) = editing("/ Mot: Définition", 0..3);
    let stored = app.stored_document();
    start(&mut app, &ctx);
    for event in [
        key(egui::Key::Enter),
        key(egui::Key::Tab),
        Event::Text("= ".into()),
    ] {
        frame(&mut app, &ctx, vec![event]);
        assert!(app.stored_document() == stored);
        assert!(!app.document.engine.can_undo());
        assert!(app.document.ime.draft.is_some());
    }
    ime(&mut app, &ctx, vec![ImeEvent::Commit("日本".into())]);
    assert_eq!(
        app.document.engine.document().source(),
        "/ 日本: Définition"
    );
}

#[test]
fn saving_and_recovery_only_contain_committed_text() {
    for commit in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let (ctx, mut app) = editing("", 0..0);
        frame(&mut app, &ctx, vec![Event::Text("A".into())]);
        app.start_recovery(root.clone(), ctx.clone());
        start(&mut app, &ctx);
        if commit {
            ime(&mut app, &ctx, vec![ImeEvent::Commit("日本".into())]);
        }
        app.recovery.worker.as_mut().unwrap().make_due();
        app.checkpoint_recovery();
        app.recovery.worker.as_mut().unwrap().wait();
        drop(app);
        let (_worker, entries) = crate::recovery::Recovery::start(
            &root,
            std::sync::Arc::new(move || ctx.request_repaint()),
        )
        .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].load().unwrap().source,
            if commit { "A日本" } else { "A" }
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let (ctx, mut app) = editing("Texte", 0..5);
    start(&mut app, &ctx);
    let path = dir.path().join("courrier.typsto");
    assert!(app.save_path(path.clone()));
    ime(&mut app, &ctx, vec![ImeEvent::Commit("Ancien".into())]);
    assert_eq!(storage::read(&path).unwrap().source, "Texte");
    assert_eq!(app.document.engine.document().source(), "Texte");
    assert!(!app.is_dirty());
}

#[test]
fn ime_in_search_stays_native_and_does_not_edit_the_document() {
    let (ctx, mut app) = editing("Texte", 0..5);
    app.apply(Command::Find);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let stored = app.stored_document();
    start(&mut app, &ctx);
    assert!(app.document.ime.draft.is_none());
    assert_eq!(app.search.engine.query, "にほん");
    ime(&mut app, &ctx, vec![ImeEvent::Commit("日本".into())]);
    assert_eq!(app.search.engine.query, "日本");
    assert!(app.stored_document() == stored);
    assert!(!app.document.engine.can_undo());
}

#[test]
fn losing_window_focus_or_clicking_cancels_preedit_without_retargeting_its_commit() {
    for click in [false, true] {
        let (ctx, mut app) = editing("Texte", 0..5);
        start(&mut app, &ctx);
        let mut raw = frame_input(vec![]);
        if click {
            raw.events = vec![Event::PointerButton {
                pos: egui::pos2(5.0, 5.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }];
        } else {
            raw.focused = false;
            raw.events = vec![Event::WindowFocused(false)];
        }
        let _ = ctx.run(raw, |ctx| app.ui_frame(ctx));
        assert!(app.document.ime.draft.is_none());
        ime(&mut app, &ctx, vec![ImeEvent::Commit("Ancien".into())]);
        assert_eq!(app.document.engine.document().source(), "Texte");
        assert!(!app.document.engine.can_undo());
    }
}

#[test]
fn two_compositions_in_one_frame_keep_distinct_commits_and_escape_can_cancel_the_first_frame() {
    let (ctx, mut app) = editing("", 0..0);
    ime(
        &mut app,
        &ctx,
        vec![
            ImeEvent::Enabled,
            ImeEvent::Preedit("に".into()),
            ImeEvent::Commit("日".into()),
            ImeEvent::Enabled,
            ImeEvent::Preedit("ほん".into()),
        ],
    );
    assert_eq!(app.document.engine.document().source(), "日");
    assert_eq!(app.document.engine.history().undo, 1);
    assert_eq!(
        app.document.ime.draft.as_ref().unwrap().input.view.text,
        "日ほん"
    );
    ime(
        &mut app,
        &ctx,
        vec![ImeEvent::Commit("本".into()), ImeEvent::Disabled],
    );
    assert_eq!(app.document.engine.document().source(), "日本");
    assert_eq!(app.document.engine.history().undo, 2);
    frame(
        &mut app,
        &ctx,
        vec![
            Event::Ime(ImeEvent::Preedit("語".into())),
            key(egui::Key::Escape),
        ],
    );
    assert!(app.document.ime.draft.is_none());
    assert_eq!(app.document.engine.document().source(), "日本");
    assert_eq!(app.document.engine.history().undo, 2);
}

#[test]
fn ime_commit_and_explicit_replacement_have_the_same_literal_source_styles_and_selection() {
    for (source, range, text) in [
        ("Un *été*.", 3..6, "e\u{301}té"),
        ("/ Mot: Définition", 4..14, "日本"),
        (
            "#quote(block: true, attribution: [Auteur])[Texte]",
            6..12,
            "Élodie",
        ),
        ("Texte", 0..5, "== #strong[Texte]"),
    ] {
        let (ctx, mut app) = editing(source, range.clone());
        let stored = app.stored_document();
        let mut explicit: crate::engine::Engine =
            crate::engine::Engine::from_stored(stored, None).unwrap();
        let mut selection = editor::cursor(range);
        std::mem::swap(&mut selection.primary, &mut selection.secondary);
        app.document.selection = selection;
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let target = explicit
            .selection_target(editor::to_core(selection))
            .unwrap();
        let command = crate::engine::Command::from_call(
            "text.insert",
            serde_json::json!({
                "target": crate::engine::TextTarget::from(target), "text": text,
            }),
        )
        .unwrap();
        explicit.run_call(command.to_call()).unwrap();
        let expected_selection = explicit.selection();
        start(&mut app, &ctx);
        ime(&mut app, &ctx, vec![ImeEvent::Commit(text.into())]);
        assert_eq!(
            app.document.engine.document().source(),
            explicit.document().source()
        );
        assert_eq!(
            app.document.engine.document().view,
            explicit.document().view
        );
        assert_eq!(
            app.document.selection,
            editor::to_editor(expected_selection)
        );
        assert_eq!(app.document.engine.history().undo, 1);
        app.apply(Command::Undo);
        assert_eq!(app.document.selection, selection);
    }
}

#[test]
fn japanese_ime_commit_keeps_unicode_and_displays_real_glyphs_in_each_editor_font() {
    let (ctx, mut app) = editing("Test : ", 7..7);
    start(&mut app, &ctx);
    ime(&mut app, &ctx, vec![ImeEvent::Commit("にほん 日本".into())]);
    assert_eq!(
        app.document.engine.document().view.text,
        "Test : にほん 日本"
    );
    assert_eq!(
        app.document.engine.document().source(),
        "Test \\: にほん 日本"
    );
    assert_eq!(app.document.engine.history().undo, 1);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("japonais.typsto");
    assert!(app.save_path(path.clone()));
    app.close_document();
    assert!(app.open_path(path));
    frame(&mut app, &ctx, vec![]);
    assert_eq!(
        app.document.engine.document().view.text,
        "Test : にほん 日本"
    );
    for font in EditorFont::ALL {
        for variant in 0..4 {
            assert!(
                ctx.fonts_mut(|fonts| fonts
                    .has_glyphs(&font.font_id(18.0, variant), "にほん日本中国繁體中文")),
                "glyphes japonais absents : {font:?}, variante {variant}"
            );
        }
    }
}
