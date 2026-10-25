use super::super::bands::{BandAction, BandContent, EditorAction};
use super::*;
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}
use crate::editor::WidgetKind;

fn setup(source: &str) -> (egui::Context, App) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source(source).unwrap()
    });
    app.document.selection = editor::cursor(0..0);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    (ctx, app)
}
fn open(app: &mut App, ctx: &egui::Context, kind: WidgetKind, remove: bool) {
    let widget = editor::widgets(&app.document.engine.document().view)
        .into_iter()
        .find(|w| w.kind == kind)
        .unwrap();
    app.dispatch_editor_action(
        if remove {
            EditorAction::Remove(widget)
        } else {
            EditorAction::Settings(widget)
        },
        ctx,
    );
    for _ in 0..3 {
        frame(app, ctx, vec![]);
    }
}
#[test]
fn list_envelope_is_complete_and_type_change_is_one_guarded_undoable_transaction() {
    let (ctx, mut app) = setup("Avant.\n\n- Un\n- Deux\n- Trois\n\nAprès.");
    let before = app.stored_document();
    let guard = app.document.engine.target_guard();
    let widgets = editor::widgets(&app.document.engine.document().view);
    assert_eq!(widgets.len(), 1);
    assert_eq!(widgets[0].lines.len(), 3);
    open(&mut app, &ctx, WidgetKind::List, false);
    assert_eq!(app.document.engine.target_guard(), guard);
    if let BandContent::List { kind } = &mut app.bands.open.as_mut().unwrap().content {
        *kind = BlockKind::Numbered;
    } else {
        panic!("list settings");
    }
    recovery_click(&mut app, &ctx, &crate::tr!("apply"));
    assert!(app.bands.open.is_none());
    assert_eq!(app.document.engine.history().undo, 1);
    assert_eq!(
        app.document
            .engine
            .document()
            .view
            .blocks
            .iter()
            .filter(|k| **k == BlockKind::Numbered)
            .count(),
        3
    );
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
}
#[test]
fn deleting_an_entire_widget_removes_its_annotations_and_restores_them_in_one_undo() {
    for (kind, source, absent) in [
        (
            WidgetKind::List,
            "Avant.\n\n/ Un: Première définition\n/ Deux: Seconde définition\n\nAprès.",
            "définition",
        ),
        (
            WidgetKind::Quote,
            "Avant.\n\n#quote(block: true, attribution: [Auteur])[Première phrase.\n\nSeconde phrase.]\n\nAprès.",
            "Auteur",
        ),
        (
            WidgetKind::Postscript,
            "Avant.\n\n#postscript[Une précision.]\n\nAprès.",
            "précision",
        ),
        (
            WidgetKind::Attachment,
            "Avant.\n\n#attachment[Formulaire]\n#attachment[Programme]\n\nAprès.",
            "Formulaire",
        ),
    ] {
        let (ctx, mut app) = setup(source);
        let before = app.stored_document();
        open(&mut app, &ctx, kind, true);
        assert_eq!(app.stored_document(), before);
        recovery_click(&mut app, &ctx, &crate::tr!("widget-delete"));
        assert!(
            app.bands.open.is_none(),
            "{source}: {:?}",
            app.dialogs.message
        );
        assert!(!app.document.engine.document().view.text.contains(absent));
        assert!(app.document.engine.document().view.text.contains("Avant."));
        assert!(app.document.engine.document().view.text.contains("Après."));
        assert_eq!(app.document.engine.history().undo, 1);
        assert!(
            !editor::widgets(&app.document.engine.document().view)
                .iter()
                .any(|w| w.kind == kind)
        );
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), before);
    }
}
#[test]
fn changing_or_closing_a_band_discards_its_draft_and_never_changes_the_document() {
    let (ctx, mut app) = setup("Avant après\n\n- Un\n- Deux");
    let before = app.stored_document();
    let guard = app.document.engine.target_guard();
    app.document.selection = editor::cursor(0..5);
    app.document
        .engine
        .update_adapter_cursor(editor::to_core(app.document.selection))
        .unwrap();
    app.open_link();
    set_band_text(&mut app, "https://discard.example");
    open(&mut app, &ctx, WidgetKind::List, false);
    assert!(matches!(
        app.bands.open.as_ref().unwrap().content,
        BandContent::List { .. }
    ));
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert!(app.bands.open.is_none());
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.engine.target_guard(), guard);
    assert_eq!(app.document.engine.history().undo, 0);
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
}
#[test]
fn a_stale_band_cannot_modify_a_newer_document_even_when_its_geometry_is_reportable() {
    let (ctx, mut app) = setup("- Un\n- Deux");
    open(&mut app, &ctx, WidgetKind::List, true);
    set_subject(&mut app, "Autre objet");
    let before = app.stored_document();
    let history = app.document.engine.history().undo;
    app.apply_band_action(BandAction::Remove, &ctx);
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.engine.history().undo, history);
    assert_eq!(
        app.bands.open.as_ref().unwrap().error.as_deref(),
        Some(crate::tr!("edit-stale-draft").as_str())
    );
}
#[test]
fn the_interline_band_shifts_native_rows_without_entering_text_statistics_or_search() {
    let (ctx, mut app) = setup("Avant.\n\n- Un\n- Deux\n\nAprès.");
    let before = app.stored_document();
    let stats = app
        .document
        .engine
        .statistics(crate::engine::StatisticsParams::default())
        .unwrap();
    open(&mut app, &ctx, WidgetKind::List, false);
    let output = search_draw(&mut app, &ctx, vec![]);
    let labels: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect();
    assert!(labels.contains(&crate::tr!("widget-list-settings").as_str()));
    assert_eq!(app.stored_document(), before);
    assert_eq!(
        app.document
            .engine
            .statistics(crate::engine::StatisticsParams::default())
            .unwrap(),
        stats
    );
    let found = app
        .document
        .engine
        .query(crate::engine::Query::Find(crate::engine::SearchParams {
            query: crate::tr!("widget-list-settings"),
            case_sensitive: false,
        }))
        .unwrap();
    assert!(found["matches"].as_array().unwrap().is_empty());
}
#[test]
fn native_ime_in_a_parameter_field_stays_private_until_apply() {
    let (ctx, mut app) = setup("Texte du courrier.");
    app.document.selection = editor::cursor(0..5);
    app.open_link();
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.stored_document();
    let guard = app.document.engine.target_guard();
    for event in [
        egui::ImeEvent::Enabled,
        egui::ImeEvent::Preedit("にほん".into()),
        egui::ImeEvent::Commit("日本".into()),
        egui::ImeEvent::Disabled,
    ] {
        frame(&mut app, &ctx, vec![egui::Event::Ime(event)]);
        assert_eq!(app.stored_document(), before);
        assert_eq!(app.document.engine.target_guard(), guard);
    }
    let BandContent::Link { url } = &app.bands.open.as_ref().unwrap().content else {
        panic!("link field");
    };
    assert!(url.contains("日本"), "{url:?}");
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert!(app.bands.open.is_none());
    assert_eq!(app.stored_document(), before);
}

#[test]
fn parameter_focus_disables_document_commands_in_all_ui_entries_and_ime_owns_escape() {
    let (ctx, mut app) = setup("Une phrase.");
    app.document.selection = editor::cursor(0..3);
    app.open_link();
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.stored_document();
    assert!(!app.command_enabled(Command::Bold));
    assert!(!app.command_enabled(Command::Undo));
    app.dispatch_action(Command::Bold, &ctx);
    assert_eq!(app.stored_document(), before);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Enabled)],
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("にほん".into()))],
    );
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert!(app.bands.open.is_some(), "the parameter IME owns Escape");
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert!(app.bands.open.is_none());
    assert_eq!(app.stored_document(), before);
}

#[test]
fn resizing_and_wrapping_keep_the_band_visible_and_its_buttons_usable() {
    let (ctx, mut app) = setup(
        "Avant.\n\n- Une très longue ligne avec des accents, 日本, et suffisamment de texte pour se replier lorsque la colonne se rétrécit.\n- Deuxième élément\n\nAprès.",
    );
    let before = app.stored_document();
    open(&mut app, &ctx, WidgetKind::List, false);
    let draw = |app: &mut App, width, events| {
        let mut input = frame_input(events);
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width, 900.0),
        ));
        ctx.run(input, |ctx| app.ui_frame(ctx))
    };
    let mut point = egui::Pos2::ZERO;
    for width in [800.0, 600.0, 1240.0, 600.0] {
        for frame in 0..4 {
            let output = draw(&mut app, width, vec![]);
            // La hauteur du bandeau est mesurée à la frame du redimensionnement et
            // appliquée à la suivante : seule cette première frame peut rogner.
            if frame == 0 {
                continue;
            }
            point = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == crate::tr!("apply") => {
                        let rect = text.galley.rect.translate(text.pos.to_vec2());
                        assert!(shape.clip_rect.contains_rect(rect), "{width}: {rect:?}");
                        Some(rect.center())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("visible apply button after resize at {width}"));
        }
        assert_eq!(app.stored_document(), before);
    }
    for pressed in [true, false] {
        let _ = draw(
            &mut app,
            600.0,
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
    assert!(app.bands.open.is_none());
    assert_eq!(app.stored_document(), before);
}
