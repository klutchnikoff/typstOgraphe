use super::*;

#[test]
fn tab_places_the_native_caret_on_each_empty_annotation_placeholder() {
    for source in [
        "#figure(image(\"images/parc.svg\"))\n\nSuite.",
        "#quote(block: true)[Une pensée.]\n\nSuite.",
    ] {
        let (ctx, mut app) = demo_app();
        // Vérifier la géométrie réellement peinte, indépendamment du clignotement.
        ctx.all_styles_mut(|style| style.visuals.text_cursor.blink = false);
        fixture(&mut app, |data| {
            data.document = Document::from_source(source).unwrap();
        });
        fixture(&mut app, |data| {
            data.document.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../../../../../fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
        });
        let annotation = app.document.engine.document().view.annotations().remove(0);
        let at = annotation.content.start;
        let placeholder = crate::i18n::text(annotation.kind.placeholder_key());
        app.document.selection = editor::cursor(annotation.main_target());
        app.document.force_selection = true;
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
                    focused: true,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        draw(&mut app, vec![]);
        let tab = || egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        for _ in 0..2 {
            let output = draw(&mut app, vec![tab()]);
            assert_eq!(app.range(), at..at);
            assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
            fn annotation_hint(shape: &egui::Shape, placeholder: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.text() == placeholder => Some(text.pos),
                    egui::Shape::Vec(shapes) => shapes
                        .iter()
                        .find_map(|shape| annotation_hint(shape, placeholder)),
                    _ => None,
                }
            }
            let hint = output
                .shapes
                .iter()
                .find_map(|shape| annotation_hint(&shape.shape, &placeholder))
                .expect("texte indicatif de l’annotation vide");
            let caret = output
                .platform_output
                .ime
                .expect("curseur natif du TextEdit")
                .cursor_rect;
            assert!(
                (caret.top() - hint.y).abs() < 2.0,
                "curseur {caret:?}, annotation {hint:?}"
            );
            assert!((caret.left() - hint.x).abs() < 2.0);
            assert!(caret.height() < app.preferences.editor.size * 2.0);
            assert!(output.shapes.iter().any(|shape| matches!(shape.shape, egui::Shape::LineSegment { points, .. } if (points[0] - caret.center_top()).length() < 1.0 && (points[1] - caret.center_bottom()).length() < 1.0)), "le curseur doit être peint dans l’annotation");
            assert_eq!(
                app.document.engine.document().source(),
                source,
                "le texte indicatif reste virtuel"
            );
        }
        draw(&mut app, vec![egui::Event::Text("Une annotation.".into())]);
        assert!(
            app.document
                .engine
                .document()
                .source()
                .contains("[Une annotation.]")
        );
        draw(&mut app, vec![tab()]);
        assert_eq!(app.range(), at..at);
        draw(&mut app, vec![egui::Event::Text("Été : ".into())]);
        assert!(
            app.document
                .engine
                .document()
                .view
                .text
                .contains("Été : Une annotation.")
        );
    }
}

#[test]
fn annotation_navigation_and_exit_preserve_content_selection_and_undo() {
    for source in [
        "#figure(image(\"images/parc.svg\"), caption: [Été *à Paris*.])\n\nSuite.",
        "#quote(block: true, attribution: [Été *à Paris*.])[Une pensée.]\n\nSuite.",
    ] {
        let (ctx, mut app) = demo_app();
        fixture(&mut app, |data| {
            data.document = Document::from_source(source).unwrap();
        });
        let annotation = app.document.engine.document().view.annotations().remove(0);
        let at = annotation.content.start;
        app.document.selection = editor::cursor(annotation.content.clone());
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Tab, egui::Modifiers::SHIFT)],
        );
        assert_eq!(app.range(), annotation.main_target());
        assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
        );
        assert_eq!(app.range(), at..at);
        assert!(
            !app.document.engine.can_undo(),
            "la navigation ne modifie pas le document"
        );
        let original = app.document.engine.document().clone();
        // Entrée quitte toute la structure sans remplacer l’annotation sélectionnée.
        app.document.selection = editor::cursor(annotation.content.clone());
        let selection = app.document.selection;
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
        );
        assert_eq!(
            app.range(),
            annotation.structure.end + 1..annotation.structure.end + 1
        );
        assert_eq!(
            app.document.engine.document().view.blocks[app
                .document
                .engine
                .document()
                .view
                .line_at(app.range().start)],
            BlockKind::Paragraph
        );
        assert_eq!(
            app.document.engine.document().view.annotations()[0],
            annotation
        );
        assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
        frame(&mut app, &ctx, vec![egui::Event::Text("Après.".into())]);
        assert!(
            app.document
                .engine
                .document()
                .view
                .text
                .contains("Été à Paris.\nAprès.")
        );
        app.apply(Command::Undo);
        app.apply(Command::Undo);
        assert_eq!(app.document.engine.document(), &original);
        assert_eq!(app.document.selection, selection);
    }
}

#[test]
fn annotation_navigation_scrolls_to_the_target_in_both_structures() {
    for source in [
        format!(
            "{}#figure(image(\"images/parc.svg\"))",
            "Avant.\n\n".repeat(80)
        ),
        format!("#quote(block: true)[{}]", "Une pensée.\n\n".repeat(80)),
    ] {
        let (ctx, mut app) = demo_app();
        fixture(&mut app, |data| {
            data.document = Document::from_source(&source).unwrap();
        });
        let annotation = app.document.engine.document().view.annotations().remove(0);
        app.document.selection = editor::cursor(annotation.main_target());
        app.document.force_selection = true;
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
                    focused: true,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        draw(&mut app, vec![]);
        let key = |modifiers| egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        for modifiers in [egui::Modifiers::NONE, egui::Modifiers::SHIFT] {
            draw(&mut app, vec![key(modifiers)]);
            // Laisser le défilement animé d’egui atteindre la cible.
            let mut output = draw(&mut app, vec![]);
            for _ in 0..8 {
                output = draw(&mut app, vec![]);
            }
            let ime = output.platform_output.ime.expect("focus natif conservé");
            assert!(
                ime.cursor_rect.top() >= 100.0 && ime.cursor_rect.bottom() <= 900.0,
                "annotation {:?}, cible {:?}, curseur {:?}",
                annotation.kind,
                app.range(),
                ime.cursor_rect
            );
            assert_eq!(
                app.range(),
                if modifiers.shift {
                    annotation.main_target()
                } else {
                    annotation.content.clone()
                }
            );
        }
        assert_eq!(app.document.engine.document().source(), source);
    }
}
