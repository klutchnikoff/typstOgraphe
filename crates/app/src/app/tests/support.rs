use super::*;

pub(super) fn search_draw(
    app: &mut App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(frame_input(events), |ctx| app.ui_frame(ctx))
}

pub(super) fn recovery_click(app: &mut App, ctx: &egui::Context, label: &str) {
    let draw = |app: &mut App, events| ctx.run(frame_input(events), |ctx| app.ui_frame(ctx));
    let _ = draw(app, vec![]);
    let output = draw(app, vec![]);
    let point = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("bouton absent : {label}"));
    for pressed in [true, false] {
        let _ = draw(
            app,
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

pub(super) fn seed_recovery(root: &std::path::Path, letter: &StoredDocument, name: Option<String>) {
    let (mut recovery, _) =
        crate::recovery::Recovery::start(root, std::sync::Arc::new(|| {})).unwrap();
    recovery.make_due();
    recovery
        .checkpoint(1, true, || (letter.clone(), name))
        .unwrap();
    recovery.wait();
    assert!(recovery.poll().into_iter().all(|result| result.is_ok()));
}

pub(super) fn icon_rect(
    output: &egui::FullOutput,
    ctx: &egui::Context,
    icon: crate::icons::Icon,
) -> egui::Rect {
    fn find(shape: &egui::Shape, texture: egui::TextureId) -> Option<egui::Rect> {
        match shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh.calc_bounds()),
            egui::Shape::Rect(rect)
                if rect
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == texture) =>
            {
                Some(rect.rect)
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, texture)),
            _ => None,
        }
    }
    let texture = icon.texture(ctx, crate::toolbar::ICON_SIZE).id();
    output
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape, texture))
        .unwrap_or_else(|| panic!("icône absente : {icon:?}"))
}

pub(super) fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
    let raw = frame_input(events);
    let _ = ctx.run(raw, |ctx| app.ui_frame(ctx));
}

impl App {
    pub(in crate::app) fn with_demo_context(ctx: egui::Context, capture: Option<PathBuf>) -> Self {
        let mut app = Self::with_context(ctx, capture);
        app.load_demo();
        app
    }
}

/// Valeurs possédées, détachées du moteur. Ne contient ni historique ni révision.
pub(super) struct Fixture {
    pub document: Document,
    pub metadata: crate::document_types::Metadata,
    pub template: String,
    pub dependencies: crate::packages::Dependencies,
    pub file_path: Option<PathBuf>,
}
pub(super) fn fixture(app: &mut App, setup: impl FnOnce(&mut Fixture)) {
    let engine = &app.document.engine;
    let mut data = Fixture {
        document: engine.document().clone(),
        metadata: engine.metadata().clone(),
        template: engine.template().into(),
        dependencies: engine.dependencies().clone(),
        file_path: engine.file_path().map(PathBuf::from),
    };
    setup(&mut data);
    // Ressource partagée des scénarios de géométrie d’image.
    for image in data.document.view.images.values() {
        if image.path == "images/parc.svg" {
            data.document
                .assets
                .entry(image.path.clone())
                .or_insert_with(|| {
                    include_bytes!("../../../../../fixtures/parc.svg")
                        .as_slice()
                        .into()
                });
        }
    }
    app.document.engine = crate::engine::Engine::from_stored(
        StoredDocument {
            source: data.document.source().into(),
            metadata: data.metadata,
            template: data.template,
            dependencies: data.dependencies,
            assets: data.document.assets,
        },
        data.file_path,
    )
    .unwrap();
    // Construire une identité neuve doit aussi être signalé à la vue. Les tests
    // peuvent encore préparer leur curseur natif après cette fixture.
    let selection = app
        .document
        .engine
        .selection_target(app.document.engine.selection())
        .unwrap();
    app.preview.sync.apply(ViewEvent::DocumentReplaced {
        selection: Some(selection),
    });
}

pub(super) fn preview_location(app: &App, index: usize) -> PagePoint {
    let target = app
        .document
        .engine
        .selection_target(editor::to_core(editor::cursor(index..index)))
        .unwrap();
    app.document
        .engine
        .locate(crate::engine::LocateParams {
            render: app.preview.render.as_ref().unwrap().reference,
            position: crate::engine::RenderPosition::ToPreview {
                target: target.into(),
            },
        })
        .unwrap()
        .point
        .unwrap()
}

pub(super) fn compile_preview(app: &mut App) {
    let info = app
        .document
        .engine
        .compile(crate::engine::GuardParams {
            guard: app.document.engine.target_guard(),
        })
        .unwrap()
        .render
        .unwrap();
    app.install_preview_render(info);
}
pub(super) fn insert_fixture_text(app: &mut App, text: &str) {
    let target = app
        .document
        .engine
        .selection_target(editor::to_core(editor::cursor(0..0)))
        .unwrap();
    app.execute_document_command(crate::engine::Command::InsertText(
        crate::engine::InsertTextParams {
            target: target.into(),
            text: text.into(),
            input_rules: crate::engine::InputRules::Literal,
        },
    ))
    .unwrap();
}
pub(super) fn set_subject(app: &mut App, value: &str) {
    app.execute_document_command(crate::engine::Command::SetMetadata(
        crate::engine::SetMetadataParams {
            guard: app.document.engine.target_guard(),
            field: crate::document_types::MetadataField::LetterSubject,
            value: value.into(),
            input: crate::engine::MetadataInput::Discrete,
        },
    ))
    .unwrap();
}

pub(super) fn demo_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let app = App::with_demo_context(ctx.clone(), None);
    (ctx, app)
}

pub(super) fn frame_input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1240.0, 900.0),
        )),
        events,
        ..Default::default()
    }
}

pub(super) trait LetterFixture {
    fn letter(&self) -> &crate::document_types::courrier::Metadata;
    fn letter_mut(&mut self) -> &mut crate::document_types::courrier::Metadata;
}
impl LetterFixture for crate::document_types::Metadata {
    fn letter(&self) -> &crate::document_types::courrier::Metadata {
        let Self::Courrier(data) = self else {
            panic!("fixture Courrier attendue")
        };
        data
    }
    fn letter_mut(&mut self) -> &mut crate::document_types::courrier::Metadata {
        let Self::Courrier(data) = self else {
            panic!("fixture Courrier attendue")
        };
        data
    }
}

pub(super) fn band_is_link(app: &App) -> bool {
    matches!(
        app.bands.open.as_ref().map(|b| &b.content),
        Some(super::super::bands::BandContent::Link { .. })
    )
}
pub(super) fn set_band_text(app: &mut App, text: &str) {
    match &mut app.bands.open.as_mut().expect("open band").content {
        super::super::bands::BandContent::Link { url } => *url = text.into(),
        super::super::bands::BandContent::Image { alt, .. } => *alt = text.into(),
        _ => panic!("text parameter band"),
    }
}
