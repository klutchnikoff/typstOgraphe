use super::*;

#[test]
fn saving_prunes_images_on_disk_without_changing_undo_redo_or_clean_state() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx, None);
    fixture(&mut app, |data| {
        data.document = Document::from_source("").unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.svg");
    std::fs::write(
        &original,
        include_bytes!("../../../../../fixtures/parc.svg"),
    )
    .unwrap();
    app.import_image(&original, None).unwrap();
    let first = app.document.engine.document().view.images[&1].path.clone();
    let path = dir.path().join("Courrier.typsto");
    assert!(app.save_path(path.clone()));
    app.import_image(&original, Some(1)).unwrap();
    let replacement = app.document.engine.document().view.images[&1].path.clone();
    std::fs::remove_file(&original).unwrap();
    assert!(app.save_path(path.clone()));
    assert!(!app.is_dirty());
    assert_eq!(
        storage::read(&path)
            .unwrap()
            .assets
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec![replacement.clone()]
    );
    assert!(app.document.engine.document().assets.contains_key(&first));
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document().view.images[&1].path, first);
    assert!(app.is_dirty());
    assert!(app.save_path(path.clone()));
    assert!(!app.is_dirty());
    assert_eq!(
        storage::read(&path)
            .unwrap()
            .assets
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec![first]
    );
    app.apply(Command::Redo);
    assert_eq!(
        app.document.engine.document().view.images[&1].path,
        replacement
    );
    app.remove_image(1);
    assert!(app.save_path(path.clone()));
    assert!(!app.is_dirty());
    assert!(storage::read(&path).unwrap().assets.is_empty());
    let export = dir.path().join("export");
    let mut request = app.document.engine.preview_request().unwrap();
    request.export_dir = Some(export.clone());
    crate::rendering::render(&mut crate::rendering::TypstWorld::default(), &request).unwrap();
    assert!(!export.join("images").exists());
    app.apply(Command::Undo);
    assert_eq!(
        app.document.engine.document().view.images[&1].path,
        replacement
    );
    assert!(app.is_dirty());
    assert!(
        app.document
            .engine
            .document()
            .assets
            .contains_key(&replacement)
    );
    assert!(app.save_path(path.clone()));
    assert_eq!(storage::read(&path).unwrap().assets.len(), 1);
    app.apply(Command::Redo);
    assert!(app.document.engine.document().view.images.is_empty());
    assert!(app.save_path(path.clone()));
    assert!(storage::read(&path).unwrap().assets.is_empty());
    assert!(!app.is_dirty());
}

#[test]
fn image_remove_button_deletes_the_selected_block_and_caption_with_undo() {
    for caption in ["", ", caption: [Le parc.]"] {
        let (ctx, mut app) = demo_app();
        fixture(&mut app, |data| {
            data.document = Document::from_source(&format!(
                "Avant.\n\n#figure(image(\"images/parc.svg\"){caption})\n\nAprès."
            ))
            .unwrap();
        });
        fixture(&mut app, |data| {
            data.document.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../../../../../fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
        });
        let original = app.document.engine.document().clone();
        let at = app
            .document
            .engine
            .document()
            .view
            .image_range(1)
            .unwrap()
            .start;
        // Boutons accessibles depuis l’objet sélectionné comme depuis la légende.
        let selection = if caption.is_empty() {
            at..at + 1
        } else {
            at + 1..at + 1
        };
        app.document.selection = editor::cursor(selection.clone());
        app.document.force_selection = true;
        frame(&mut app, &ctx, vec![]);
        let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == crate::tr!("image-remove")
                {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                } else {
                    None
                }
            })
            .expect("bouton Supprimer de l’image sélectionnée");
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
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
            if pressed {
                assert_eq!(
                    app.range(),
                    selection,
                    "l’appui sur le bouton ne déplace pas le curseur dans le texte sous-jacent"
                );
            }
        }
        assert!(
            app.document.engine.document().view.images.is_empty(),
            "le clic doit supprimer l’image entière"
        );
        assert!(
            !app.document
                .engine
                .document()
                .view
                .text
                .contains("Le parc.")
        );
        assert!(app.document.engine.document().view.text.contains("Avant."));
        assert!(app.document.engine.document().view.text.contains("Après."));
        app.apply(Command::Undo);
        assert_eq!(app.document.engine.document(), &original);
    }
}

#[test]
fn image_controls_edit_alt_and_keyboard_deletes_the_whole_block_with_undo() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source("Avant.\n\nAprès.").unwrap();
    });
    app.document.selection = editor::cursor(6..6);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("parc.svg");
    std::fs::write(&path, include_bytes!("../../../../../fixtures/parc.svg")).unwrap();
    app.import_image(&path, None).unwrap();
    frame(&mut app, &ctx, vec![]);
    let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
    let text_point = |output: &egui::FullOutput, label: &str| {
        output
            .shapes
            .iter()
            .rev()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == label
                {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("Texte absent : {label}"))
    };
    let click = |app: &mut App, pos| {
        frame(
            app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            app,
            &ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    };
    click(
        &mut app,
        text_point(&output, &crate::tr!("image-information")),
    );
    assert_eq!(app.dialogs.image_info, Some(1));
    app.dialogs.image_alt = "Des arbres et un chemin".into();
    let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
    click(&mut app, text_point(&output, &crate::tr!("image-apply")));
    assert!(app.dialogs.image_info.is_none());
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("Des arbres et un chemin")
    );
    frame(&mut app, &ctx, vec![egui::Event::Text("Le parc.".into())]);
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("caption: [Le parc.]"),
        "{} / {:?}",
        app.document.engine.document().source(),
        app.range()
    );
    let before = app.document.engine.document().source().to_owned();
    let at = app
        .document
        .engine
        .document()
        .view
        .image_range(1)
        .unwrap()
        .start
        + 1;
    app.document.selection = editor::cursor(at..at);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    let backspace = || egui::Event::Key {
        key: egui::Key::Backspace,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![backspace()]);
    assert_eq!(app.range(), at - 1..at);
    frame(&mut app, &ctx, vec![backspace()]);
    assert!(app.document.engine.document().view.images.is_empty());
    assert!(
        !app.document
            .engine
            .document()
            .view
            .text
            .contains("Le parc.")
    );
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document().source(), before);
}

#[test]
fn images_insert_edit_caption_replace_undo_and_reopen_without_original_file() {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source("Avant.\n\nAprès.").unwrap();
    });
    app.document.selection = editor::cursor(6..6);
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("Parc.svg");
    std::fs::write(
        &original,
        include_bytes!("../../../../../fixtures/parc.svg"),
    )
    .unwrap();
    app.import_image(&original, None).unwrap();
    assert_eq!(app.document.engine.document().view.images.len(), 1);
    let id = *app
        .document
        .engine
        .document()
        .view
        .images
        .keys()
        .next()
        .unwrap();
    let object = app
        .document
        .engine
        .document()
        .view
        .image_range(id)
        .unwrap()
        .start;
    assert_eq!(app.range(), object + 1..object + 1);
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("Le parc municipal.".into())],
    );
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("caption: [Le parc municipal.]"),
        "{}",
        app.document.engine.document().source()
    );
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
    assert_eq!(app.range(), object + 1..object + 1);
    app.execute_document_command(crate::engine::Command::SetImageAlt(
        crate::engine::ImageAltParams {
            target: app.document.engine.image_target(id).unwrap(),
            alt: "Trois arbres autour d’un chemin".into(),
        },
    ))
    .unwrap();
    app.import_image(&original, Some(id)).unwrap();
    assert_eq!(
        app.document.engine.document().view.images[&id].alt,
        "Trois arbres autour d’un chemin"
    );
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("Le parc municipal.")
    );
    app.apply(Command::Undo);
    assert_eq!(
        app.document.engine.document().view.images[&id].path,
        "images/image-1.svg"
    );
    app.apply(Command::Redo);
    assert_eq!(
        app.document.engine.document().view.images[&id].path,
        "images/image-2.svg"
    );
    let archive = dir.path().join("Courrier.typsto");
    assert!(app.save_path(archive.clone()));
    std::fs::remove_file(&original).unwrap();
    let saved_source = app.document.engine.document().source().to_owned();
    app.remove_image(id);
    assert!(app.document.engine.document().view.images.is_empty());
    app.apply(Command::Undo);
    assert_eq!(app.document.engine.document().source(), saved_source);
    assert!(app.open_path(archive));
    assert!(!app.is_dirty());
    assert_eq!(app.document.engine.document().source(), saved_source);
    assert_eq!(app.document.engine.document().assets.len(), 1);
    app.document.selection = editor::cursor(object + 1..object + 1);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert_eq!(
        app.document.engine.document().view.blocks[app
            .document
            .engine
            .document()
            .view
            .line_at(app.range().start)],
        BlockKind::Paragraph
    );
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("Le parc municipal.")
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("Suite du courrier.".into())],
    );
    assert!(
        app.document
            .engine
            .document()
            .source()
            .contains("Suite du courrier.")
    );
}
