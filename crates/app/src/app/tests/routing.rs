use super::*;
use crate::{
    commands::Route,
    engine::{Availability, Command as Core, Empty, Query},
};

fn body(app: &mut App, source: &str, range: Range<usize>) {
    fixture(app, |data| {
        data.document = Document::from_source(source).unwrap();
    });
    app.document.selection = editor::cursor(range);
    app.document.force_selection = true;
}
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}
fn binding(app: &mut App, ctx: &egui::Context, command: Command) {
    let overrides =
        serde_json::from_value(serde_json::json!({command.definition().id:[{"key":"F6"}]}))
            .unwrap();
    app.keymap = Keymap::new(app.keymap.registry(), ctx.os().is_mac(), &overrides).unwrap();
}
fn text_point(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
    output.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text() == label => {
            Some(t.pos + t.galley.rect.center().to_vec2())
        }
        _ => None,
    })
}
fn draw(
    app: &mut App,
    ctx: &egui::Context,
    toolbar: bool,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut action = None;
    let output = ctx.run(frame_input(events), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            action = if toolbar {
                crate::toolbar::show(
                    ui,
                    &app.document.engine.document().view,
                    app.range(),
                    &app.keymap,
                    |c| app.command_enabled(c),
                )
            } else {
                menus::egui_bar(ui, &app.keymap, &app.menu_statuses())
            };
        });
    });
    if let Some(action) = action {
        app.dispatch_action(action, ctx);
    }
    output
}
fn click(app: &mut App, ctx: &egui::Context, toolbar: bool, point: egui::Pos2) {
    for pressed in [true, false] {
        draw(
            app,
            ctx,
            toolbar,
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
}

#[test]
fn every_registered_ui_action_has_an_explicit_route_and_documentary_routes_are_registered() {
    for kind in DocumentType::ALL {
        for presentation in Registry::new(kind).definitions() {
            match presentation.command.route(Default::default()) {
                Route::Document(core) => {
                    assert_eq!(presentation.id, core.id());
                    assert!(
                        Core::command_specs()
                            .iter()
                            .any(|spec| spec.id.as_str() == core.id())
                    );
                }
                Route::Prepared(intent) => {
                    let expected = if presentation.command == Command::ImportModel {
                        "document.set_model"
                    } else {
                        presentation.id
                    };
                    assert_eq!(intent.id(), expected);
                    assert!(
                        Core::command_specs()
                            .iter()
                            .any(|spec| spec.id.as_str() == intent.id())
                    );
                }
                Route::Contextual => assert!(
                    Core::command_specs()
                        .iter()
                        .any(|spec| spec.id.as_str() == presentation.id)
                ),
                Route::Interface => assert!(matches!(
                    presentation.command,
                    Command::File(FileCommand::Quit)
                        | Command::Palette
                        | Command::Preferences
                        | Command::Assistance
                        | Command::Information
                        | Command::RecoverDocuments
                        | Command::Find
                        | Command::FindReplace
                        | Command::ToggleSpelling
                        | Command::SpellingSuggestions
                        | Command::LearnSpelling
                )),
                Route::Unsupported => panic!("unrouted UI action {}", presentation.id),
            }
        }
    }
}

#[test]
fn availability_uses_guarded_ui_selection_without_moving_the_engine_cursor_or_history() {
    let (_, mut app) = demo_app();
    body(&mut app, "Été\n\nNoël", 0..9);
    let before = app.document.engine.query(Query::Inspect(Empty {})).unwrap();
    let source = app.stored_document();
    assert!(!app.command_enabled(Command::Link));
    app.document.selection = editor::cursor(0..3);
    assert!(app.command_enabled(Command::Link));
    let statuses = app.menu_statuses();
    for (command, status) in statuses {
        assert_eq!(
            status.enabled,
            app.command_enabled(command),
            "{}",
            command.definition().id
        );
    }
    assert_eq!(
        app.document.engine.query(Query::Inspect(Empty {})).unwrap(),
        before
    );
    assert_eq!(app.stored_document(), source);
    app.document.history_field = Some(egui::Id::new("metadata"));
    assert!(!app.command_enabled(Command::Clear));
    // Modalité graphique : les menus et dispatch sont bloqués, pas l’évaluation
    // de la palette dans sa propre fenêtre.
    app.dialogs.palette_open = true;
    assert!(app.command_enabled(Command::Bold));
    assert!(
        app.menu_statuses()
            .iter()
            .all(|(_, status)| !status.enabled)
    );
}

#[test]
fn menus_toolbar_palette_and_custom_shortcut_publish_engine_transactions_and_undo() {
    for surface in ["menu", "toolbar", "palette", "shortcut"] {
        let (ctx, mut app) = demo_app();
        let clear = surface == "menu";
        body(&mut app, if clear { "Un *été*." } else { "Un été." }, 3..6);
        let before = app.stored_document();
        let revision = app.document.engine.revision();
        let command = if clear { Command::Clear } else { Command::Bold };
        match surface {
            "menu" => {
                let output = draw(&mut app, &ctx, false, vec![]);
                click(
                    &mut app,
                    &ctx,
                    false,
                    text_point(&output, &crate::commands::MenuGroup::Edit.label()).unwrap(),
                );
                let output = draw(&mut app, &ctx, false, vec![]);
                click(
                    &mut app,
                    &ctx,
                    false,
                    text_point(&output, &command.label()).unwrap(),
                );
            }
            "toolbar" => {
                let output = draw(&mut app, &ctx, true, vec![]);
                click(
                    &mut app,
                    &ctx,
                    true,
                    icon_rect(&output, &ctx, crate::icons::Icon::Bold).center(),
                );
            }
            "palette" => {
                app.apply(Command::Palette);
                app.dialogs.palette_query = command.label();
                frame(&mut app, &ctx, vec![]);
                frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
                assert!(!app.dialogs.palette_open);
            }
            "shortcut" => {
                binding(&mut app, &ctx, command);
                frame(&mut app, &ctx, vec![]);
                frame(&mut app, &ctx, vec![key(egui::Key::F6)]);
            }
            _ => unreachable!(),
        }
        assert_eq!(app.document.engine.revision(), revision + 1, "{surface}");
        assert_eq!(app.document.engine.history().undo, 1, "{surface}");
        assert!(
            app.document.engine.document().view.styles[3..6]
                .iter()
                .all(|s| s.bold != clear),
            "{surface}"
        );
        assert_eq!(app.document.engine.selection().range(), 3..6, "{surface}");
        app.apply(Command::Undo);
        assert_eq!(app.stored_document(), before, "{surface}");
    }
}

#[test]
fn disabled_link_stays_disabled_for_toolbar_palette_shortcut_and_direct_dispatch() {
    let (ctx, mut app) = demo_app();
    body(&mut app, "Un texte.", 0..0);
    let before = app.stored_document();
    let output = draw(&mut app, &ctx, true, vec![]);
    click(
        &mut app,
        &ctx,
        true,
        icon_rect(&output, &ctx, crate::icons::Icon::Link).center(),
    );
    assert!(!band_is_link(&app));
    app.apply(Command::Palette);
    app.dialogs.palette_query = Command::Link.label();
    let output = search_draw(&mut app, &ctx, vec![]);
    assert!(text_point(&output, &Command::Link.label()).is_none());
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert!(app.dialogs.palette_open && !band_is_link(&app));
    app.dialogs.palette_open = false;
    binding(&mut app, &ctx, Command::Link);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![key(egui::Key::F6)]);
    app.dispatch_action(Command::Link, &ctx);
    assert!(!band_is_link(&app));
    assert_eq!(app.stored_document(), before);
    assert!(!app.document.engine.can_undo());
}

#[test]
fn search_and_spelling_availability_read_the_engine_instead_of_graphical_result_lists() {
    let (ctx, mut app) = demo_app();
    body(&mut app, "Un courrrier.", 3..3);
    app.search.engine.query = "courrrier".into();
    assert!(app.search.engine.matches.is_empty());
    assert!(app.spelling.checker.issues.is_empty());
    assert!(app.command_enabled(Command::FindNext));
    assert!(app.command_enabled(Command::NextSpelling));
    assert!(app.command_enabled(Command::IgnoreSpelling));
    let before = app.stored_document();
    app.apply(Command::IgnoreSpelling);
    assert_eq!(app.document.engine.history().undo, 1);
    assert!(!app.command_enabled(Command::NextSpelling));
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
    app.preferences.spelling.enabled = false;
    assert!(!app.command_enabled(Command::NextSpelling));
    assert!(!app.command_enabled(Command::IgnoreSpelling));
    // Une commande inconnue du type ou un document fermé ne peut pas être forcé
    // par un raccourci ou un appel d’adaptateur.
    let mut closed = App::with_context(ctx.clone(), None);
    assert!(!closed.command_enabled(Command::Bold));
    closed.dispatch_action(Command::Bold, &ctx);
    assert!(!closed.document.engine.is_open());
    assert!(matches!(
        closed
            .document
            .engine
            .available(&Core::Bold(Default::default())),
        Availability::Disabled { .. }
    ));
}

#[test]
fn protected_image_insertion_and_stale_replacements_are_disabled_by_the_engine() {
    let (_, mut app) = demo_app();
    body(&mut app, "#quote(block: true)[Une citation.]", 3..3);
    assert!(!app.command_enabled(Command::InsertImage));
    assert!(
        !app.menu_statuses()
            .iter()
            .find(|(c, _)| *c == Command::InsertImage)
            .unwrap()
            .1
            .enabled
    );
    body(&mut app, "Un texte.", 0..0);
    app.search.engine.query = "texte".into();
    app.search.engine.replacement = "mot".into();
    app.refresh_search();
    app.select_search_result(0);
    assert!(app.command_enabled(Command::ReplaceCurrent));
    let before = app.document.engine.target_guard();
    app.run_document_command(Core::InsertText(crate::engine::InsertTextParams {
        target: Default::default(),
        text: "autre".into(),
        input_rules: Default::default(),
    }));
    assert_ne!(app.document.engine.target_guard(), before);
    app.document.selection = editor::cursor(3..8);
    assert!(!app.command_enabled(Command::ReplaceCurrent));
    app.search.engine.query = "autre".into();
    app.search.engine.replacement = "deux\nlignes".into();
    assert!(!app.command_enabled(Command::ReplaceAll));
}
