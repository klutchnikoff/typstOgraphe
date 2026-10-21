use super::*;

#[test]
fn personal_profile_seeds_new_documents_and_signature_edits_are_independent_and_undoable() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("signature.svg");
    std::fs::write(
        &original,
        include_bytes!("../../../tests/fixtures/signature.svg"),
    )
    .unwrap();
    let (name, image) = crate::profile::Profile::import_signature(dir.path(), &original).unwrap();
    app.preferences.profile.name = "Zoé Martin".into();
    app.preferences.profile.organisation = "Les Tilleuls".into();
    app.preferences.profile.signature = Some(name);
    app.profile_signature = Some(image);
    app.create_document(DocumentType::Courrier);
    let initial = app.stored_document();
    assert_eq!(initial.metadata.courrier().expediteur, "Zoé Martin");
    assert_eq!(
        initial.metadata.signature_path(),
        Some("images/signature-1.svg")
    );
    assert!(!app.is_dirty());
    app.preferences.profile.name = "Autre nom".into();
    app.preferences.profile.signature = None;
    app.profile_signature = None;
    assert_eq!(app.stored_document(), initial);
    assert!(!app.is_dirty());
    let replacement = crate::assets::ImageData {
        extension: "png".into(),
        data: include_bytes!("../../../tests/fixtures/colours.png")
            .as_slice()
            .into(),
    };
    app.set_document_signature(Some(&replacement)).unwrap();
    let changed = app.stored_document();
    assert_ne!(
        initial.metadata.signature_path(),
        changed.metadata.signature_path()
    );
    assert!(app.is_dirty());
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), initial);
    app.apply(Command::Redo);
    assert_eq!(app.stored_document(), changed);
    // Une absence de signature dans le profil ne doit pas effacer celle du courrier.
    assert!(app.use_profile_signature().is_err());
    assert_eq!(app.stored_document(), changed);
    app.preferences.profile.signature = Some("signature-profile.svg".into());
    app.profile_signature = Some(crate::assets::ImageData {
        extension: "svg".into(),
        data: initial.assets[initial.metadata.signature_path().unwrap()].clone(),
    });
    let preferences = app.preferences.clone();
    app.use_profile_signature().unwrap();
    let restored = app.stored_document();
    assert_eq!(
        restored.assets[restored.metadata.signature_path().unwrap()],
        initial.assets[initial.metadata.signature_path().unwrap()]
    );
    assert_eq!(app.preferences, preferences);
    let revision = app.session.revision;
    let assets = app.session.document.assets.len();
    app.use_profile_signature().unwrap();
    assert_eq!(app.session.revision, revision);
    assert_eq!(app.session.document.assets.len(), assets);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), changed);
    app.apply(Command::Redo);
    assert_eq!(app.stored_document(), restored);
    app.set_document_signature(None).unwrap();
    app.use_profile_signature().unwrap();
    assert!(app.session.metadata.signature_path().is_some());
    // Revenir à la signature remplacée pour poursuivre le test d'archivage.
    app.apply(Command::Undo);
    app.apply(Command::Undo);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), changed);
    let path = dir.path().join("document.typsto");
    assert!(app.save_path(path.clone()));
    let saved = storage::read(&path).unwrap();
    assert_eq!(saved.assets.len(), 1);
    assert_eq!(
        saved.metadata.signature_path(),
        changed.metadata.signature_path()
    );
    app.set_document_signature(None).unwrap();
    assert!(app.save_path(path.clone()));
    assert!(storage::read(&path).unwrap().assets.is_empty());
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), changed);
    assert!(app.save_path(path.clone()));
    app.create_document(DocumentType::CompteRendu);
    assert!(app.file_error.is_some());
    assert_eq!(app.stored_document(), changed);
    assert!(app.open_path(path));
    assert_eq!(app.session.metadata.courrier().expediteur, "Zoé Martin");
    assert_eq!(app.preferences.profile.name, "Autre nom");
}

#[test]
fn global_profile_saves_after_changes_or_closing_without_modifying_the_document() {
    #[derive(Default)]
    struct Store {
        data: std::collections::HashMap<String, String>,
        writes: usize,
    }
    impl eframe::Storage for Store {
        fn get_string(&self, key: &str) -> Option<String> {
            self.data.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.data.insert(key.into(), value);
            self.writes += 1;
        }
        fn flush(&mut self) {}
    }
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    let before = app.stored_document();
    let mut store = Store::default();
    let now = Instant::now();
    app.persist_preferences(&mut store, now, false);
    app.preferences.profile.name = "Zoé".into();
    app.persist_preferences(&mut store, now, false);
    assert_eq!(store.writes, 0);
    app.persist_preferences(
        &mut store,
        now + crate::preferences::SaveTracker::DELAY,
        false,
    );
    assert_eq!(store.writes, 1);
    assert_eq!(Preferences::load(Some(&store)).profile.name, "Zoé");
    app.persist_preferences(&mut store, now + Duration::from_secs(100), false);
    assert_eq!(store.writes, 1);
    app.preferences.profile.email = "zoe@example.org".into();
    eframe::App::save(&mut app, &mut store); // Fermeture avant le délai de saisie.
    assert_eq!(
        Preferences::load(Some(&store)).profile.email,
        "zoe@example.org"
    );
    assert_eq!(store.writes, 2);
    assert_eq!(eframe::App::auto_save_interval(&app), Duration::MAX);
    app.preferences_open = true;
    app.preferences_page = PreferencesPage::Profile;
    frame(&mut app, &ctx, vec![]);
    app.preferences_page = PreferencesPage::Appearance;
    frame(&mut app, &ctx, vec![]);
    app.preferences_page = PreferencesPage::Spelling;
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.stored_document(), before);
    assert!(!app.is_dirty());
    app.capture = Some("ignored.png".into());
    assert!(!eframe::App::persist_egui_memory(&app));
    app.preferences.profile.name = "Capture isolée".into();
    eframe::App::save(&mut app, &mut store);
    assert_eq!(store.writes, 2);
}

#[test]
fn editor_preferences_change_glyphs_and_theme_without_touching_the_letter() {
    let ctx = egui::Context::default();
    let mut app = App::with_demo_context(ctx.clone(), None);
    let letter = app.stored_document();
    let revision = app.session.revision;
    app.dispatch_action(Command::Preferences, &ctx);
    assert!(app.preferences_open);
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
                ..Default::default()
            },
            |ctx| app.preferences_ui(ctx),
        )
    };
    for (current, next) in [
        ("Libertinus Serif", "DejaVu Sans Mono"),
        ("Système", "Sombre"),
    ] {
        for label in [current, next] {
            draw(&mut app, vec![]);
            let output = draw(&mut app, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("choix absent : {label}"));
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
        }
    }
    assert_eq!(app.preferences.editor.font, EditorFont::Monospace);
    app.preferences.editor.size = 24.0;
    let _ = ctx.run(egui::RawInput::default(), |ctx| app.preferences_ui(ctx));
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    app.preferences.appearance = Appearance::System;
    let _ = ctx.run(
        egui::RawInput {
            system_theme: Some(egui::Theme::Light),
            ..Default::default()
        },
        |ctx| app.preferences_ui(ctx),
    );
    let _ = ctx.run(
        egui::RawInput {
            system_theme: Some(egui::Theme::Light),
            ..Default::default()
        },
        |_| {},
    );
    assert_eq!(ctx.theme(), egui::Theme::Light);
    let mut view = Document::from_source("Normal\n\n= Titre").unwrap().view;
    view.toggle_mark(0..3, Mark::Bold);
    view.toggle_mark(0..3, Mark::Italic);
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let galley = editor::layout(ui, &view, &view.text, 500.0, app.preferences.editor);
            assert_eq!(
                galley.job.sections[0].format.font_id,
                EditorFont::Monospace.font_id(24.0, 3)
            );
            let h1_size = 24.0 * 1.2_f32.powi(3);
            assert!(
                galley
                    .job
                    .sections
                    .iter()
                    .any(|s| (s.format.font_id.size - h1_size).abs() < 0.001)
            );
        });
    });
    assert_eq!(app.stored_document(), letter);
    assert_eq!(app.session.revision, revision);
    assert!(!app.is_dirty());
}
