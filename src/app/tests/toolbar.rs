use super::*;

#[test]
fn compact_toolbar_and_status_show_unicode_totals_selection_and_tooltips() {
    fn text_rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                let bounds = text
                    .galley
                    .rows
                    .iter()
                    .fold(egui::Rect::NOTHING, |bounds, row| {
                        bounds.union(row.rect_without_leading_space())
                    });
                Some(bounds.translate(text.pos.to_vec2()))
            }
            _ => None,
        })
    }
    let (ctx, mut app) = demo_app();
    ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);
    fixture(&mut app.document.engine, |session| {
        session.document = Document::from_source("#strong[e\u{301}] 👩🏽‍💻").unwrap();
    });
    app.document.force_selection = true;
    let mut time = 0.0;
    let mut draw = |app: &mut App, events| {
        time += 0.2;
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 640.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| app.ui_frame(ctx),
        )
    };
    draw(&mut app, vec![]);
    let output = draw(&mut app, vec![]);
    let brand = text_rect(&output, "typstOgraphe").unwrap();
    let heading = icon_rect(&output, &ctx, crate::icons::Icon::Heading);
    assert!(brand.right() < heading.left());
    assert!(
        (brand.center().y - heading.center().y).abs() < 2.0,
        "nom {brand:?}, titres {heading:?}"
    );
    assert_eq!(output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "typstOgraphe")).count(), 1);
    let total = crate::tr!("document-stats", words = 1, signs = 3);
    assert!(text_rect(&output, &total).is_some());
    assert!(text_rect(&output, "Point d’insertion").is_none());
    app.document.selection = editor::cursor(0..2); // e + accent combiné : un signe.
    app.document.force_selection = true;
    draw(&mut app, vec![]);
    let output = draw(&mut app, vec![]);
    let selection = crate::tr!("selection-stats", words = 1, signs = 1);
    assert!(text_rect(&output, &total).is_some());
    assert!(text_rect(&output, &selection).is_some());
    for (label, without_spaces) in [(&total, 2), (&selection, 1)] {
        let output = draw(&mut app, vec![]);
        let pos = text_rect(&output, label).unwrap().center();
        draw(&mut app, vec![egui::Event::PointerMoved(pos)]);
        let mut output = draw(&mut app, vec![]);
        for _ in 0..4 {
            output = draw(&mut app, vec![]);
        }
        assert!(
            text_rect(
                &output,
                &crate::tr!("statistics-hint", without_spaces = without_spaces)
            )
            .is_some(),
            "détail hors espaces de {label}"
        );
    }
    draw(&mut app, vec![egui::Event::Text("Deux mots".into())]);
    let output = draw(&mut app, vec![]);
    let edited = crate::tr!("document-stats", words = 2, signs = 11);
    assert!(text_rect(&output, &edited).is_some());
    assert!(text_rect(&output, &selection).is_none());
    app.apply(Command::Undo);
    let output = draw(&mut app, vec![]);
    assert!(text_rect(&output, &total).is_some());
    assert!(text_rect(&output, &selection).is_some());
    app.apply(Command::Redo);
    let output = draw(&mut app, vec![]);
    assert!(text_rect(&output, &edited).is_some());
    app.create_document(DocumentType::Courrier);
    let output = draw(&mut app, vec![]);
    assert!(text_rect(&output, &crate::tr!("document-stats", words = 0, signs = 0)).is_some());
}

#[test]
fn toolbar_icons_dispatch_commands_and_keep_headings_separate_from_quotes() {
    use crate::icons::Icon;
    fn draw(
        app: &mut App,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Option<Command>) {
        let mut action = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(520.0, 260.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    action = crate::toolbar::show(
                        ui,
                        &app.document.engine.document().view,
                        app.range(),
                        &app.keymap,
                        |command| app.command_enabled(command),
                    );
                });
            },
        );
        // Vérifier le dispatch de l’image sans ouvrir un sélecteur natif pendant le test.
        if let Some(command) = action
            && command != Command::InsertImage
        {
            app.apply(command);
        }
        (output, action)
    }
    fn click(app: &mut App, ctx: &egui::Context, pos: egui::Pos2) -> Option<Command> {
        let mut action = None;
        for pressed in [true, false] {
            action = draw(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            )
            .1
            .or(action);
        }
        action
    }
    fn text_point(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
    }
    for dark in [false, true] {
        let (ctx, mut app) = demo_app();
        ctx.set_visuals(if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        ctx.style_mut(|style| style.animation_time = 0.0);
        fixture(&mut app.document.engine, |session| {
            session.document = Document::from_source("Été à Paris.").unwrap();
        });
        app.document.selection = editor::cursor(0..3);
        let text = app.document.engine.document().view.text.clone();
        for (icon, command, mark) in [
            (Icon::Bold, Command::Bold, Mark::Bold),
            (Icon::Italic, Command::Italic, Mark::Italic),
        ] {
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let pos = icon_rect(&output, &ctx, icon).center();
            assert_eq!(click(&mut app, &ctx, pos), Some(command));
            assert!(app.document.engine.document().view.mark_state(0..3, mark).0);
            assert_eq!(app.range(), 0..3);
            let (output, _) = draw(&mut app, &ctx, vec![]);
            let texture = icon.texture(&ctx, crate::toolbar::ICON_SIZE).id();
            let tint = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect
                            .brush
                            .as_ref()
                            .is_some_and(|brush| brush.fill_texture_id == texture) =>
                    {
                        Some(rect.fill)
                    }
                    _ => None,
                })
                .expect("icône active peinte");
            assert_eq!(tint.r(), tint.g());
            assert_eq!(tint.g(), tint.b());
        }
        let mut heading_icon = Icon::Heading;
        for (kind, next_icon) in [
            (BlockKind::Heading(1), Icon::Heading1),
            (BlockKind::Heading(2), Icon::Heading2),
            (BlockKind::Heading(3), Icon::Heading3),
            (BlockKind::Paragraph, Icon::Heading),
        ] {
            let (output, _) = draw(&mut app, &ctx, vec![]);
            click(
                &mut app,
                &ctx,
                icon_rect(&output, &ctx, heading_icon).center(),
            );
            let (menu, _) = draw(&mut app, &ctx, vec![]);
            for icon in [
                Icon::Paragraph,
                Icon::Heading1,
                Icon::Heading2,
                Icon::Heading3,
            ] {
                assert!(icon_rect(&menu, &ctx, icon).is_positive());
            }
            assert!(text_point(&menu, &Command::Block(BlockKind::Quote).label()).is_none());
            let command = Command::Block(kind);
            let pos = text_point(&menu, &command.short_label()).expect("entrée du menu Titres");
            assert_eq!(click(&mut app, &ctx, pos), Some(command));
            assert_eq!(app.document.engine.document().view.blocks[0], kind);
            assert_eq!(app.range(), 0..3);
            heading_icon = next_icon;
        }
        let (output, _) = draw(&mut app, &ctx, vec![]);
        let pos = icon_rect(&output, &ctx, Icon::Link).center();
        assert_eq!(click(&mut app, &ctx, pos), Some(Command::Link));
        assert!(app.dialogs.link_open);
        assert_eq!(app.dialogs.link_range, 0..3);
        app.dialogs.link_open = false;
        let (output, _) = draw(&mut app, &ctx, vec![]);
        let pos = icon_rect(&output, &ctx, Icon::Image).center();
        assert_eq!(click(&mut app, &ctx, pos), Some(Command::InsertImage));
        assert_eq!(app.document.engine.document().view.text, text);
        let (output, _) = draw(&mut app, &ctx, vec![]);
        let pos = icon_rect(&output, &ctx, Icon::Quote).center();
        assert_eq!(
            click(&mut app, &ctx, pos),
            Some(Command::Block(BlockKind::Quote))
        );
        assert_eq!(
            app.document.engine.document().view.blocks,
            [BlockKind::Quote, BlockKind::QuoteAttribution]
        );
        let (output, _) = draw(&mut app, &ctx, vec![]);
        let pos = icon_rect(&output, &ctx, Icon::Quote).center();
        assert_eq!(
            click(&mut app, &ctx, pos),
            Some(Command::Block(BlockKind::Paragraph))
        );
        assert_eq!(
            app.document.engine.document().view.blocks[0],
            BlockKind::Paragraph
        );
    }
}

#[test]
fn toolbar_groups_the_three_list_commands_in_a_single_menu() {
    let ctx = egui::Context::default();
    ctx.style_mut(|s| s.animation_time = 0.0);
    let mut app = App::with_demo_context(ctx.clone(), None);
    let draw = |app: &mut App, events| {
        ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.toolbar(ui));
            },
        )
    };
    fn point(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|s| match &s.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
    }
    for kind in [
        BlockKind::Bullet,
        BlockKind::Numbered,
        BlockKind::Description,
    ] {
        draw(&mut app, vec![]);
        let output = draw(&mut app, vec![]);
        assert!(point(&output, "• Liste").is_none());
        assert!(point(&output, "1. Liste").is_none());
        let pos = icon_rect(&output, &ctx, crate::icons::Icon::List).center();
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
        for icon in [
            crate::icons::Icon::ListOrdered,
            crate::icons::Icon::ListDescription,
        ] {
            assert!(icon_rect(&output, &ctx, icon).is_positive());
        }
        for list in [
            BlockKind::Bullet,
            BlockKind::Numbered,
            BlockKind::Description,
        ] {
            assert!(point(&output, &Command::Block(list).label()).is_some());
        }
        let pos = point(&output, &Command::Block(kind).label()).unwrap();
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
        assert_eq!(app.document.engine.document().view.blocks[0], kind);
    }
}
