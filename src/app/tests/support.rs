use super::*;

pub(super) fn search_draw(
    app: &mut App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1240.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| app.ui_frame(ctx),
    )
}

pub(super) fn recovery_click(app: &mut App, ctx: &egui::Context, label: &str) {
    let draw = |app: &mut App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1240.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.ui_frame(ctx),
        )
    };
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
    let (mut recovery, _) = crate::recovery::Recovery::start(root, Default::default()).unwrap();
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
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1240.0, 900.0),
        )),
        events,
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| app.ui_frame(ctx));
}
