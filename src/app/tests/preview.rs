use super::*;

#[test]
fn column_drag_and_window_resize_keep_the_chosen_proportion_without_a_preview_cap() {
    let (ctx, mut app) = demo_app();
    let mut time = 0.0;
    let mut draw = |app: &mut App, width, events| {
        time += 0.1;
        let mut rects = [egui::Rect::NOTHING; 2];
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        rects = app.columns_ui(ui, false);
                    });
            },
        );
        rects
    };
    let initial = draw(&mut app, 1240.0, vec![]);
    assert!((initial[0].width() / initial[1].width() - 1.0).abs() < 0.01);
    let start = egui::pos2(initial[0].right() + 4.0, 450.0);
    draw(
        &mut app,
        1240.0,
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let end = egui::pos2(374.0, 450.0);
    draw(&mut app, 1240.0, vec![egui::Event::PointerMoved(end)]);
    draw(
        &mut app,
        1240.0,
        vec![egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    let fraction = app.preferences.editor_fraction;
    assert!((fraction - 0.3).abs() < 0.02, "{fraction}");
    for width in [960.0, 2400.0, 1240.0] {
        let rects = draw(&mut app, width, vec![]);
        let actual = rects[0].width() / (rects[0].width() + rects[1].width());
        assert!((actual - fraction).abs() < 0.001);
        if width > 2000.0 {
            assert!(rects[1].width() > 1100.0);
        }
    }
    app.preferences.editor_fraction = 0.9;
    let rects = draw(&mut app, 1240.0, vec![]);
    assert!(rects[1].width() < 300.0);
    let divider = egui::pos2(rects[0].right() + 4.0, 450.0);
    for _ in 0..2 {
        for pressed in [true, false] {
            draw(
                &mut app,
                1240.0,
                vec![
                    egui::Event::PointerMoved(divider),
                    egui::Event::PointerButton {
                        pos: divider,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    assert_eq!(app.preferences.editor_fraction, 0.5);
}

#[test]
fn synchronization_scrolls_both_panes_and_rejects_outdated_or_failed_renders() {
    let (ctx, mut app) = demo_app();
    app.document.session.document =
        Document::from_source(&"Un paragraphe à retrouver.\n\n".repeat(100)).unwrap();
    let mut world = crate::rendering::TypstWorld::default();
    let rendered = world
        .compile_with_template(
            app.document.session.document.source(),
            &app.document.session.metadata,
            &app.document.session.template,
        )
        .unwrap();
    for page in 0..rendered.pages().len() {
        app.preview.pages.push(ctx.load_texture(
            format!("test-page-{page}"),
            egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
            egui::TextureOptions::LINEAR,
        ));
    }
    app.preview.sync = Some(RenderSync::new(
        app.document.session.revision,
        world,
        rendered,
        app.document.session.document.source_map.clone(),
    ));
    let end = app.document.session.document.view.len() - 12;
    app.document.selection = editor::cursor(end..end);
    let location = app.preview.sync.as_ref().unwrap().forward(end).unwrap();

    let mut draw_pane = |preview: bool, time: f64| {
        let mut offset = 0.0;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 500.0),
                )),
                time: Some(time),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let id = ui.make_persistent_id(egui::Id::new(if preview {
                        "preview-scroll"
                    } else {
                        "editor-scroll"
                    }));
                    if preview {
                        app.preview_ui(ui);
                    } else {
                        app.editor_ui(ui);
                    }
                    offset = egui::scroll_area::State::load(ctx, id).unwrap().offset.y;
                });
            },
        );
        offset
    };
    let mut offset = 0.0;
    for i in 0..10 {
        offset = draw_pane(true, f64::from(i) / 10.0);
    }
    assert!(offset > 1000.0, "preview offset {offset}");

    app.document.selection = editor::cursor(0..0);
    app.document.session.revision += 1;
    app.navigate_backward(location, &ctx);
    assert_eq!(app.document.selection.primary.index, 0);
    app.document.session.revision -= 1;
    app.preview.error = Some("essai".into());
    app.navigate_backward(location, &ctx);
    assert_eq!(app.document.selection.primary.index, 0);
    app.preview.error = None;
    app.navigate_backward(location, &ctx);
    assert!(app.document.selection.primary.index.abs_diff(end) <= 1);
    assert!(!app.preview.forward_pending);

    for i in 10..20 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 500.0),
                )),
                time: Some(f64::from(i) / 10.0),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let id = ui.make_persistent_id(egui::Id::new("editor-scroll"));
                    app.editor_ui(ui);
                    offset = egui::scroll_area::State::load(ctx, id).unwrap().offset.y;
                });
            },
        );
    }
    assert!(offset > 1000.0, "editor offset {offset}");
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
    assert!(app.document.selection.primary.index.abs_diff(end) <= 1);
    assert!(!app.preview.editor_scroll_pending);
    assert!(!app.preview.forward_pending);
}

#[test]
fn double_click_on_the_scaled_preview_returns_focus_to_the_editor() {
    let (ctx, mut app) = demo_app();
    let mut world = crate::rendering::TypstWorld::default();
    let rendered = world
        .compile_with_template(
            app.document.session.document.source(),
            &app.document.session.metadata,
            &app.document.session.template,
        )
        .unwrap();
    let texture = ctx.load_texture(
        "click-test",
        egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
        egui::TextureOptions::LINEAR,
    );
    let texture_id = texture.id();
    app.preview.pages = vec![texture];
    app.preview.sync = Some(RenderSync::new(
        app.document.session.revision,
        world,
        rendered,
        app.document.session.document.source_map.clone(),
    ));
    app.preview.submitted = app.document.session.revision;
    app.preview.revision = Some(app.document.session.revision);
    let index = app
        .document
        .session
        .document
        .view
        .text
        .find("Notre")
        .map(|byte| {
            app.document.session.document.view.text[..byte]
                .chars()
                .count()
        })
        .unwrap();
    let location = app.preview.sync.as_ref().unwrap().forward(index).unwrap();
    let size = app
        .preview
        .sync
        .as_ref()
        .unwrap()
        .page_size(location.page)
        .unwrap();
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
    draw(&mut app, 0.0, vec![]);
    let output = draw(&mut app, 0.1, vec![]);
    let rect = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Rect(rect)
                if rect
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == texture_id) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .expect("image de l’aperçu");
    let pointer = rect.min
        + egui::vec2(
            location.point[0] / size[0] * rect.width(),
            location.point[1] / size[1] * rect.height(),
        );
    for (step, pressed) in [true, false, true, false].into_iter().enumerate() {
        draw(
            &mut app,
            0.2 + step as f64 * 0.05,
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
    assert_eq!(app.document.selection.primary.index, index);
    assert!(app.document.selection.is_empty());
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
    let mut expected = app.document.session.document.view.text.clone();
    expected.insert_str(
        app.document.session.document.view.byte_index(index),
        "Ajout ",
    );
    draw(&mut app, 0.5, vec![egui::Event::Text("Ajout ".into())]);
    assert_eq!(app.document.session.document.view.text, expected);
}

#[test]
fn preview_paints_selection_at_page_scale_and_clears_it_when_unavailable() {
    let (ctx, mut app) = demo_app();
    let mut world = crate::rendering::TypstWorld::default();
    let rendered = world
        .compile_with_template(
            app.document.session.document.source(),
            &app.document.session.metadata,
            &app.document.session.template,
        )
        .unwrap();
    app.preview.pages = vec![ctx.load_texture(
        "highlight-test",
        egui::ColorImage::new([2, 3], vec![Color32::WHITE; 6]),
        egui::TextureOptions::LINEAR,
    )];
    app.preview.sync = Some(RenderSync::new(
        app.document.session.revision,
        world,
        rendered,
        app.document.session.document.source_map.clone(),
    ));
    app.preview.submitted = app.document.session.revision;
    app.preview.revision = Some(app.document.session.revision);
    let byte = app
        .document
        .session
        .document
        .view
        .text
        .find("jeudi")
        .unwrap();
    let index = app.document.session.document.view.text[..byte]
        .chars()
        .count();
    app.document.selection = editor::cursor(index..index + 5);
    let draw = |app: &mut App, width| {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.preview_ui(ui));
            },
        );
        output
            .shapes
            .into_iter()
            .filter_map(|shape| match shape.shape {
                egui::epaint::Shape::Path(path) if path.fill == PREVIEW_SELECTION => {
                    Some((shape.clip_rect, path))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    draw(&mut app, 600.0);
    let wide = draw(&mut app, 600.0);
    assert_eq!(wide.len(), 1);
    let (clip, path) = &wide[0];
    assert!(path.points.iter().all(|point| clip.contains(*point)));
    let wide_width = path.points[1].x - path.points[0].x;
    let narrow = draw(&mut app, 400.0);
    assert_eq!(narrow.len(), 1);
    assert!(narrow[0].1.points[1].x - narrow[0].1.points[0].x < wide_width);
    // Le sens de la sélection ne change pas le passage surligné.
    app.document.selection = editor::cursor(index + 5..index);
    assert_eq!(draw(&mut app, 400.0)[0].1.points, narrow[0].1.points);
    app.document.selection = editor::cursor(index..index);
    assert!(draw(&mut app, 400.0).is_empty());
    app.document.selection = editor::cursor(index..index + 5);
    app.document.session.revision += 1;
    assert!(draw(&mut app, 400.0).is_empty());
    app.document.session.revision -= 1;
    app.preview.error = Some("essai".into());
    assert!(draw(&mut app, 400.0).is_empty());
    app.preview.error = None;
    assert!(!draw(&mut app, 400.0).is_empty());
}
