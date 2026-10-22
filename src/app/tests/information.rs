use super::*;

#[test]
fn today_button_keeps_a_fixed_date_through_history_and_reopening() {
    let ctx = egui::Context::default();
    let mut app = App::with_context(ctx.clone(), None);
    app.create_document(DocumentType::Courrier);
    app.dialogs.information_open = true;
    app.dialogs.information_group = InformationGroup::Fields("information");
    let source = app.document.engine.document.source().to_owned();
    recovery_click(&mut app, &ctx, &crate::tr!("date-today"));
    let fixed = app.document.engine.metadata.courrier().date.clone();
    assert_eq!(fixed, crate::dates::today());
    app.apply(Command::Undo);
    assert!(app.document.engine.metadata.courrier().date.is_empty());
    app.apply(Command::Redo);
    assert_eq!(app.document.engine.metadata.courrier().date, fixed);
    assert_eq!(app.document.engine.document.source(), source);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("date-fixe.typsto");
    assert!(app.save_path(path.clone()));
    app.close_document();
    assert!(app.open_path(path));
    assert_eq!(app.document.engine.metadata.courrier().date, fixed);
    assert!(!app.is_dirty());
}

#[test]
fn general_settings_and_template_selection_preserve_letter_content() {
    let (ctx, mut app) = demo_app();
    app.document.engine.template = "#let courrier(..args, options: (:), body) = { set page(paper: options.paper, flipped: options.landscape); body }".into();
    app.document.engine.saved.template = app.document.engine.template.clone();
    app.dialogs.information_open = true;
    ctx.style_mut(|s| s.animation_time = 0.0);
    let source = app.document.engine.document.source().to_owned();
    let revision = app.document.engine.revision;
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
            |ctx| app.ui_frame(ctx),
        )
    };
    fn point(output: &egui::FullOutput, value: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == value => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("champ ou choix absent : {value}"))
    }
    fn click(
        app: &mut App,
        draw: &mut impl FnMut(&mut App, Vec<egui::Event>) -> egui::FullOutput,
        pos: egui::Pos2,
    ) {
        for pressed in [true, false] {
            draw(
                app,
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
    fn choose(
        app: &mut App,
        draw: &mut impl FnMut(&mut App, Vec<egui::Event>) -> egui::FullOutput,
        current: &str,
        next: &str,
    ) {
        draw(app, vec![]);
        let output = draw(app, vec![]);
        click(app, draw, point(&output, current));
        let output = draw(app, vec![]);
        click(app, draw, point(&output, next));
    }
    choose(&mut app, &mut draw, "A4", "A3");
    choose(&mut app, &mut draw, "Portrait", "Paysage");
    choose(&mut app, &mut draw, "Français", "Anglais");
    choose(
        &mut app,
        &mut draw,
        "Modèle du document",
        "Courrier personnel",
    );
    assert_eq!(
        app.document.engine.metadata.options_mut().paper,
        PaperFormat::A4
    );
    assert!(!app.document.engine.metadata.options_mut().landscape);
    assert_eq!(
        app.document.engine.metadata.options_mut().language,
        DocumentLanguage::English
    );
    assert_eq!(app.document.engine.template, Model::Personal.source());
    assert!(app.is_dirty());
    assert_eq!(app.document.engine.revision, revision + 4);
    assert_eq!(app.document.engine.document.source(), source);

    assert!(!app.file.window_title.contains("typstOgraphe"));
}

#[test]
fn inline_information_keeps_text_accessible_and_metadata_edits_refresh_the_document() {
    fn text_rect(output: &egui::FullOutput, value: &str) -> egui::Rect {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == value => Some(
                    text.galley
                        .rect
                        .translate(text.pos.to_vec2())
                        .intersect(shape.clip_rect),
                ),
                _ => None,
            })
            .unwrap_or_else(|| panic!("texte absent de l’interface : {value}"))
    }
    fn click(
        app: &mut App,
        draw: &mut impl FnMut(&mut App, Vec<egui::Event>) -> egui::FullOutput,
        pos: egui::Pos2,
    ) {
        for pressed in [true, false] {
            draw(
                app,
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
    let (ctx, mut app) = demo_app();
    ctx.style_mut(|style| style.animation_time = 0.0);
    app.document.selection = editor::cursor(10..20);
    app.document.force_selection = true;
    let source = app.document.engine.document.source().to_owned();
    let selection = app.document.selection;
    let revision = app.document.engine.revision;
    let mut time = 0.0;
    let mut draw = |app: &mut App, events| {
        time += 0.1;
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
    let closed = draw(&mut app, vec![]);
    let header = text_rect(&closed, "Général");
    assert!(header.top() > icon_rect(&closed, &ctx, crate::icons::Icon::Heading).bottom());
    for label in ["Destinataire", "Informations", "Expéditeur"] {
        let button = text_rect(&closed, label);
        assert_eq!(button.center().y, header.center().y);
    }
    let has_text = |output: &egui::FullOutput, value: &str| {
        output.shapes.iter().any(
            |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == value),
        )
    };
    assert!(!has_text(&closed, "Format du papier"));
    let preview = text_rect(&closed, "Aperçu Typst");
    click(&mut app, &mut draw, header.center());
    draw(&mut app, vec![]);
    let opened = draw(&mut app, vec![]);
    assert!(app.dialogs.information_open);
    assert!(!app.modal_open());
    assert!(!app.is_dirty());
    assert_eq!(app.document.engine.revision, revision);
    assert_eq!(text_rect(&opened, "Aperçu Typst"), preview);
    assert!(has_text(&opened, "Format du papier"));
    assert!(!has_text(&opened, "Adresse"));
    assert!(!has_text(&opened, "Nom complet"));
    // Même à la taille minimale de la fenêtre, le formulaire reste dans la
    // colonne de gauche et laisse une zone de rédaction utilisable.
    let body = text_rect(&opened, &app.document.engine.document.view.text);
    assert!(body.height() > 150.0, "zone de rédaction : {body:?}");
    click(
        &mut app,
        &mut draw,
        text_rect(&opened, "Expéditeur").center(),
    );
    draw(&mut app, vec![]);
    let sender = draw(&mut app, vec![]);
    assert!(has_text(&sender, "Nom complet"));
    assert!(!has_text(&sender, "Format du papier"));
    assert!(!has_text(&sender, "Adresse"));
    let sender_button = text_rect(&sender, "Expéditeur");
    click(&mut app, &mut draw, sender_button.center());
    let collapsed = draw(&mut app, vec![]);
    assert!(!app.dialogs.information_open);
    assert!(!has_text(&collapsed, "Nom complet"));
    assert!(
        text_rect(&collapsed, &app.document.engine.document.view.text).height() >= body.height()
    );
    app.apply(Command::Information);
    let reopened = draw(&mut app, vec![]);
    assert!(app.dialogs.information_open);
    assert_eq!(
        app.dialogs.information_group,
        InformationGroup::Fields("sender")
    );
    assert!(has_text(&reopened, "Nom complet"));
    assert_eq!(app.document.engine.revision, revision);
    assert!(!app.is_dirty());
    assert_eq!(app.document.engine.document.source(), source);
    assert_eq!(app.document.selection, selection);
    click(
        &mut app,
        &mut draw,
        text_rect(&reopened, "Informations").center(),
    );
    draw(&mut app, vec![]);
    let correspondence = draw(&mut app, vec![]);
    let original_subject = app.document.engine.metadata.courrier_mut().objet.clone();
    let subject = text_rect(
        &correspondence,
        &app.document.engine.metadata.courrier_mut().objet,
    );
    assert!(subject.is_positive());
    assert!(subject.right() < preview.left());
    click(&mut app, &mut draw, subject.center());
    draw(
        &mut app,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Text("Objet modifié".into()),
        ],
    );
    assert_eq!(
        app.document.engine.metadata.courrier_mut().objet,
        "Objet modifié"
    );
    assert_eq!(app.document.engine.document.source(), source);
    assert_eq!(app.document.selection, selection);
    assert_eq!(app.document.engine.revision, revision + 1);
    assert!(app.is_dirty());
    assert!(app.dialogs.information_open);
    app.apply(Command::Palette);
    app.dialogs.palette_query = "Annuler".into();
    draw(&mut app, vec![]);
    draw(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            modifiers: egui::Modifiers::NONE,
            physical_key: None,
            pressed: true,
            repeat: false,
        }],
    );
    assert!(!app.dialogs.palette_open);
    draw(&mut app, vec![]);
    assert_eq!(
        app.document.engine.metadata.courrier_mut().objet,
        original_subject
    );
    assert_eq!(app.document.engine.document.source(), source);
    app.dispatch_action(Command::Redo, &ctx);
    draw(&mut app, vec![]);
    assert_eq!(
        app.document.engine.metadata.courrier_mut().objet,
        "Objet modifié"
    );
    assert_eq!(app.document.engine.revision, revision + 3);
    let current = draw(&mut app, vec![]);
    click(
        &mut app,
        &mut draw,
        text_rect(&current, "Informations").center(),
    );
    draw(&mut app, vec![]);
    assert!(!app.dialogs.information_open);
    assert_eq!(
        app.document.engine.metadata.courrier_mut().objet,
        "Objet modifié"
    );
    assert_eq!(app.document.engine.revision, revision + 3);
}

#[test]
fn document_information_offers_only_the_markers_allowed_by_the_template() {
    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut app = App::with_demo_context(ctx.clone(), None);
    let mut world = crate::rendering::TypstWorld::default();
    let draw = |app: &mut App, events| {
        ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.bullet_marker_ui(ui));
            },
        )
    };
    fn point(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
    }
    let output = draw(&mut app, vec![]);
    assert!(point(&output, "Automatique (modèle)").is_some());
    assert!(point(&output, "▪").is_none());
    assert!(point(&output, "Caractère personnalisé…").is_none());
    app.document.presentation = world
        .model_presentation("#let editor-options = (bullet-markers: (\"▪\",),)")
        .unwrap();
    for label in ["Automatique (modèle)", "▪"] {
        draw(&mut app, vec![]);
        let output = draw(&mut app, vec![]);
        let pos = point(&output, label).unwrap();
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
    assert_eq!(
        app.document
            .engine
            .metadata
            .options()
            .bullet_marker
            .as_deref(),
        Some("▪")
    );
    let output = draw(&mut app, vec![]);
    assert!(point(&output, "Caractère personnalisé…").is_none());
}
