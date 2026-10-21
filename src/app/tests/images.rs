use super::*;

#[test]
fn discarded_images_in_memory_do_not_exhaust_the_document_size_limit() {
    let mut app = App::with_demo_context(egui::Context::default(), None);
    app.document.session.document = Document::from_source("").unwrap();
    // Simuler plus de 64 Mio d’anciennes ressources partagées en mémoire.
    // Elles ne sont plus référencées par le document ni par son modèle.
    let unused: std::sync::Arc<[u8]> = vec![0; 1024 * 1024].into();
    for index in 0..65 {
        app.document
            .session
            .document
            .assets
            .insert(format!("images/old-{index}.png"), unused.clone());
    }
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("parc.svg");
    std::fs::write(&image, include_bytes!("../../../tests/fixtures/parc.svg")).unwrap();
    app.import_image(&image, None).unwrap();
    let path = dir.path().join("courrier.typsto");
    assert!(app.save_path(path.clone()), "{:?}", app.file.error);
    assert_eq!(storage::read(&path).unwrap().assets.len(), 1);
    assert!(!app.is_dirty());
}

#[test]
fn saving_prunes_images_on_disk_without_changing_undo_redo_or_clean_state() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx, None);
    app.document.session.document = Document::from_source("").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.svg");
    std::fs::write(
        &original,
        include_bytes!("../../../tests/fixtures/parc.svg"),
    )
    .unwrap();
    app.import_image(&original, None).unwrap();
    let first = app.document.session.document.view.images[&1].path.clone();
    let path = dir.path().join("Courrier.typsto");
    assert!(app.save_path(path.clone()));
    app.import_image(&original, Some(1)).unwrap();
    let replacement = app.document.session.document.view.images[&1].path.clone();
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
    assert!(app.document.session.document.assets.contains_key(&first));
    app.apply(Command::Undo);
    assert_eq!(app.document.session.document.view.images[&1].path, first);
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
        app.document.session.document.view.images[&1].path,
        replacement
    );
    app.remove_image(1);
    assert!(app.save_path(path.clone()));
    assert!(!app.is_dirty());
    assert!(storage::read(&path).unwrap().assets.is_empty());
    let export = dir.path().join("export");
    crate::rendering::render(
        &mut crate::rendering::TypstWorld::default(),
        &app.request(Some(export.clone())),
    )
    .unwrap();
    assert!(!export.join("images").exists());
    app.apply(Command::Undo);
    assert_eq!(
        app.document.session.document.view.images[&1].path,
        replacement
    );
    assert!(app.is_dirty());
    assert!(
        app.document
            .session
            .document
            .assets
            .contains_key(&replacement)
    );
    assert!(app.save_path(path.clone()));
    assert_eq!(storage::read(&path).unwrap().assets.len(), 1);
    app.apply(Command::Redo);
    assert!(app.document.session.document.view.images.is_empty());
    assert!(app.save_path(path.clone()));
    assert!(storage::read(&path).unwrap().assets.is_empty());
    assert!(!app.is_dirty());
}

#[test]
fn image_remove_button_deletes_the_selected_block_and_caption_with_undo() {
    for caption in ["", ", caption: [Le parc.]"] {
        let (ctx, mut app) = demo_app();
        app.document.session.document = Document::from_source(&format!(
            "Avant.\n\n#figure(image(\"images/parc.svg\"){caption})\n\nAprès."
        ))
        .unwrap();
        app.document.session.document.assets.insert(
            "images/parc.svg".into(),
            include_bytes!("../../../tests/fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        let original = app.document.session.document.clone();
        let at = app
            .document
            .session
            .document
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
            app.document.session.document.view.images.is_empty(),
            "le clic doit supprimer l’image entière"
        );
        assert!(!app.document.session.document.view.text.contains("Le parc."));
        assert!(app.document.session.document.view.text.contains("Avant."));
        assert!(app.document.session.document.view.text.contains("Après."));
        app.apply(Command::Undo);
        assert_eq!(app.document.session.document, original);
    }
}

#[test]
fn image_controls_edit_alt_and_keyboard_deletes_the_whole_block_with_undo() {
    let (ctx, mut app) = demo_app();
    app.document.session.document = Document::from_source("Avant.\n\nAprès.").unwrap();
    app.document.selection = editor::cursor(6..6);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("parc.svg");
    std::fs::write(&path, include_bytes!("../../../tests/fixtures/parc.svg")).unwrap();
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
            .session
            .document
            .source()
            .contains("Des arbres et un chemin")
    );
    frame(&mut app, &ctx, vec![egui::Event::Text("Le parc.".into())]);
    assert!(
        app.document
            .session
            .document
            .source()
            .contains("caption: [Le parc.]"),
        "{} / {:?}",
        app.document.session.document.source(),
        app.range()
    );
    let before = app.document.session.document.source().to_owned();
    let at = app
        .document
        .session
        .document
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
    assert!(app.document.session.document.view.images.is_empty());
    assert!(!app.document.session.document.view.text.contains("Le parc."));
    app.apply(Command::Undo);
    assert_eq!(app.document.session.document.source(), before);
}

#[test]
fn images_insert_edit_caption_replace_undo_and_reopen_without_original_file() {
    let (ctx, mut app) = demo_app();
    app.document.session.document = Document::from_source("Avant.\n\nAprès.").unwrap();
    app.document.selection = editor::cursor(6..6);
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("Parc.svg");
    std::fs::write(
        &original,
        include_bytes!("../../../tests/fixtures/parc.svg"),
    )
    .unwrap();
    app.import_image(&original, None).unwrap();
    assert_eq!(app.document.session.document.view.images.len(), 1);
    let id = *app
        .document
        .session
        .document
        .view
        .images
        .keys()
        .next()
        .unwrap();
    let object = app
        .document
        .session
        .document
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
            .session
            .document
            .source()
            .contains("caption: [Le parc municipal.]"),
        "{}",
        app.document.session.document.source()
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
    let before_alt = app.snapshot();
    app.document
        .session
        .document
        .view
        .images
        .get_mut(&id)
        .unwrap()
        .alt = "Trois arbres autour d’un chemin".into();
    app.remember(before_alt, false);
    app.import_image(&original, Some(id)).unwrap();
    assert_eq!(
        app.document.session.document.view.images[&id].alt,
        "Trois arbres autour d’un chemin"
    );
    assert!(
        app.document
            .session
            .document
            .source()
            .contains("Le parc municipal.")
    );
    app.apply(Command::Undo);
    assert_eq!(
        app.document.session.document.view.images[&id].path,
        "images/image-1.svg"
    );
    app.apply(Command::Redo);
    assert_eq!(
        app.document.session.document.view.images[&id].path,
        "images/image-2.svg"
    );
    let archive = dir.path().join("Courrier.typsto");
    assert!(app.save_path(archive.clone()));
    std::fs::remove_file(&original).unwrap();
    let saved_source = app.document.session.document.source().to_owned();
    app.remove_image(id);
    assert!(app.document.session.document.view.images.is_empty());
    app.apply(Command::Undo);
    assert_eq!(app.document.session.document.source(), saved_source);
    assert!(app.open_path(archive));
    assert!(!app.is_dirty());
    assert_eq!(app.document.session.document.source(), saved_source);
    assert_eq!(app.document.session.document.assets.len(), 1);
    app.document.selection = editor::cursor(object + 1..object + 1);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert_eq!(
        app.document.session.document.view.blocks[app
            .document
            .session
            .document
            .view
            .line_at(app.range().start)],
        BlockKind::Paragraph
    );
    assert!(
        app.document
            .session
            .document
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
            .session
            .document
            .source()
            .contains("Suite du courrier.")
    );
}
