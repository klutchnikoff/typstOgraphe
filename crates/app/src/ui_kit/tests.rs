use super::gallery::Orientation;
use super::*;
use crate::{icons::Icon, theme::Tokens};
use eframe::egui::{self, Rect, pos2};
use std::cell::Cell;

fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            pos2(0.0, 0.0),
            egui::vec2(640.0, 480.0),
        )),
        events,
        ..Default::default()
    }
}
fn context(dark: bool) -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    ctx.all_styles_mut(|style| style.animation_time = 0.0);
    ctx
}
fn click(ctx: &egui::Context, run: &mut impl FnMut(&egui::Context), at: egui::Pos2) {
    for pressed in [true, false] {
        let _ = ctx.run(
            input(vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]),
            &mut *run,
        );
    }
}

#[test]
fn every_button_kind_keeps_the_minimum_control_height_and_its_label() {
    for dark in [false, true] {
        let ctx = context(dark);
        let mut heights = vec![];
        let _ = ctx.run(input(vec![]), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for kind in [
                    ButtonKind::Primary,
                    ButtonKind::Secondary,
                    ButtonKind::Ghost,
                    ButtonKind::Danger,
                ] {
                    heights.push(button(ui, kind, "Valider").rect.height());
                }
                heights.push(icon_button(ui, Icon::Bold, false, "Gras").rect.height());
            });
        });
        assert!(heights.iter().all(|h| *h >= CONTROL_HEIGHT), "{heights:?}");
    }
}

#[test]
fn a_disabled_button_does_not_report_clicks() {
    let ctx = context(false);
    let clicked = Cell::new(false);
    let rect = Cell::new(Rect::NOTHING);
    let mut run = |ctx: &egui::Context| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let response = button_enabled(ui, ButtonKind::Primary, false, "Valider");
            rect.set(response.rect);
            clicked.set(clicked.get() || response.clicked());
        });
    };
    let _ = ctx.run(input(vec![]), &mut run);
    click(&ctx, &mut run, rect.get().center());
    assert!(!clicked.get());
}

#[test]
fn segmented_selects_the_clicked_segment_and_reports_the_change_once() {
    let ctx = context(false);
    let value = Cell::new(Orientation::Portrait);
    let changes = Cell::new(0);
    let group = Cell::new(Rect::NOTHING);
    let options = [
        (Orientation::Portrait, "Portrait".to_owned()),
        (Orientation::Landscape, "Paysage".to_owned()),
    ];
    let mut run = |ctx: &egui::Context| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut current = value.get();
            let response = segmented(ui, &mut current, &options);
            value.set(current);
            group.set(response.rect);
            changes.set(changes.get() + usize::from(response.changed()));
        });
    };
    let _ = ctx.run(input(vec![]), &mut run);
    let at = pos2(group.get().right() - 14.0, group.get().center().y);
    click(&ctx, &mut run, at);
    assert_eq!(value.get(), Orientation::Landscape);
    assert_eq!(changes.get(), 1);
    click(&ctx, &mut run, at);
    assert_eq!(
        changes.get(),
        1,
        "choisir la valeur déjà active ne change rien"
    );
}

#[test]
fn a_focused_field_paints_the_accent_ring_and_an_unfocused_one_does_not() {
    fn ring(shapes: &[egui::epaint::ClippedShape], color: egui::Color32) -> bool {
        fn has(shape: &egui::Shape, color: egui::Color32) -> bool {
            match shape {
                egui::Shape::Rect(rect) => rect.stroke.width == 3.0 && rect.stroke.color == color,
                egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has(shape, color)),
                _ => false,
            }
        }
        shapes.iter().any(|clipped| has(&clipped.shape, color))
    }
    for dark in [false, true] {
        let ctx = context(dark);
        let soft = Tokens::new(dark).accent_soft;
        let text = std::cell::RefCell::new(String::from("valeur"));
        let id = Cell::new(None);
        let mut run = |ctx: &egui::Context| {
            egui::CentralPanel::default().show(ctx, |ui| {
                id.set(Some(text_field(ui, "field", &mut text.borrow_mut()).id));
            });
        };
        let before = ctx.run(input(vec![]), &mut run);
        assert!(!ring(&before.shapes, soft));
        ctx.memory_mut(|memory| memory.request_focus(id.get().expect("field id")));
        let _ = ctx.run(input(vec![]), &mut run);
        let after = ctx.run(input(vec![]), &mut run);
        assert!(ring(&after.shapes, soft), "dark={dark}");
    }
}

#[test]
fn dialog_footer_puts_the_first_action_on_the_far_right() {
    let ctx = context(false);
    let (primary, secondary) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
    let mut titled = false;
    // La première passe d’une zone sert à mesurer ; la seconde la dessine.
    let mut run = |ctx: &egui::Context| {
        Dialog::new("test-dialog", "Nouveau document")
            .description("Choisissez un type.")
            .width(420.0)
            .show(
                ctx,
                |ui| {
                    ui.label("Corps");
                },
                |ui| {
                    primary.set(button(ui, ButtonKind::Primary, "Créer").rect);
                    secondary.set(button(ui, ButtonKind::Ghost, "Annuler").rect);
                },
            );
    };
    let _ = ctx.run(input(vec![]), &mut run);
    let output = ctx.run(input(vec![]), &mut run);
    fn has_text(shape: &egui::Shape, wanted: &str) -> bool {
        match shape {
            egui::Shape::Text(text) => text.galley.text() == wanted,
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has_text(shape, wanted)),
            _ => false,
        }
    }
    for shape in &output.shapes {
        titled |= has_text(&shape.shape, "Nouveau document");
    }
    assert!(titled, "titre absent");
    let (primary, secondary) = (primary.get(), secondary.get());
    assert!(
        primary.left() > secondary.right(),
        "{primary:?} {secondary:?}"
    );
}

#[test]
fn gallery_renders_in_both_themes_with_the_theme_tokens() {
    for dark in [false, true] {
        let ctx = context(dark);
        let mut state = Gallery::default();
        let output = ctx.run(input(vec![]), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| gallery(ui, &mut state));
        });
        let t = Tokens::new(dark);
        fn fills(shape: &egui::Shape, out: &mut Vec<egui::Color32>) {
            match shape {
                egui::Shape::Rect(rect) => out.push(rect.fill),
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| fills(s, out)),
                _ => {}
            }
        }
        let mut all = vec![];
        for clipped in &output.shapes {
            fills(&clipped.shape, &mut all);
        }
        for expected in [t.accent, t.accent_soft, t.ok_soft, t.danger_soft] {
            assert!(all.contains(&expected), "dark={dark} : {expected:?} absent");
        }
    }
}
