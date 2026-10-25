use super::*;

fn draw(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut input = frame_input(events);
    input.screen_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(960.0, 640.0),
    ));
    ctx.run(input, |ctx| app.ui_frame(ctx))
}

fn key(app: &mut App, ctx: &egui::Context, key: egui::Key) -> egui::FullOutput {
    draw(
        app,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect(),
    );
    draw(app, ctx, vec![])
}

/// Le libellé de la commande sélectionnée doit être entièrement dans le viewport.
fn visible_selection(output: &egui::FullOutput, dark: bool) -> Option<egui::Rect> {
    let accent = crate::theme::Tokens::new(dark).accent;
    output.shapes.iter().find_map(|clipped| {
        if let egui::Shape::Text(shape) = &clipped.shape
            && shape.fallback_color == accent
            && shape.pos.y > 150.0
        {
            let rect = shape.galley.rect.translate(shape.pos.to_vec2());
            clipped.clip_rect.contains_rect(rect).then_some(rect)
        } else {
            None
        }
    })
}

fn setup(dark: bool) -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    let mut app = App::with_demo_context(ctx.clone(), None);
    ctx.all_styles_mut(|style| style.animation_time = 0.0);
    app.apply(Command::Palette);
    for _ in 0..4 {
        draw(&mut app, &ctx, vec![]);
    }
    (ctx, app)
}

#[test]
fn palette_keyboard_navigation_keeps_every_command_visible_in_both_directions_and_wraps() {
    for dark in [false, true] {
        let (ctx, mut app) = setup(dark);
        for direction in [egui::Key::ArrowDown, egui::Key::ArrowUp] {
            let mut wrapped = false;
            // Un tour du catalogue disponible, borné par le nombre total de commandes.
            for _ in 0..app.keymap.registry().definitions().count() {
                let output = key(&mut app, &ctx, direction);
                assert!(
                    visible_selection(&output, dark).is_some(),
                    "commande {} invisible : {direction:?}, dark={dark}",
                    app.dialogs.palette_index,
                );
                if app.dialogs.palette_index == 0 {
                    wrapped = true;
                    break;
                }
            }
            assert!(wrapped, "retour à la première commande : {direction:?}");
        }
    }
}

#[test]
fn palette_preserves_manual_scrolling_then_reveals_a_new_search_result() {
    let (ctx, mut app) = setup(false);
    let output = draw(&mut app, &ctx, vec![]);
    let at = visible_selection(&output, false).unwrap().center();
    draw(
        &mut app,
        &ctx,
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -600.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let output = draw(&mut app, &ctx, vec![]);
    assert_eq!(app.dialogs.palette_index, 0);
    assert!(visible_selection(&output, false).is_none());
    // Laisser finir le lissage de la molette avant de reprendre au clavier.
    for _ in 0..40 {
        draw(&mut app, &ctx, vec![]);
    }
    assert!(visible_selection(&draw(&mut app, &ctx, vec![]), false).is_none());
    key(&mut app, &ctx, egui::Key::ArrowDown);
    assert!(visible_selection(&draw(&mut app, &ctx, vec![]), false).is_some());
    for _ in 0..8 {
        key(&mut app, &ctx, egui::Key::ArrowDown);
    }
    draw(&mut app, &ctx, vec![egui::Event::Text("document".into())]);
    let output = draw(&mut app, &ctx, vec![]);
    assert_eq!(app.dialogs.palette_index, 0);
    assert!(visible_selection(&output, false).is_some());
}

#[test]
fn reopening_the_palette_reveals_the_first_command_after_scrolling() {
    let (ctx, mut app) = setup(false);
    key(&mut app, &ctx, egui::Key::ArrowUp);
    key(&mut app, &ctx, egui::Key::Escape);
    app.apply(Command::Palette);
    for _ in 0..3 {
        draw(&mut app, &ctx, vec![]);
    }
    assert_eq!(app.dialogs.palette_index, 0);
    assert!(visible_selection(&draw(&mut app, &ctx, vec![]), false).is_some());
}
