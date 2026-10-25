use super::*;
use crate::ui_kit::SidePanel;

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}
fn panel() -> SidePanel {
    SidePanel {
        id: "test",
        icon: crate::icons::Icon::List,
        label: "Panneau".into(),
    }
}
fn columns(app: &mut App, ctx: &egui::Context, time: f64) -> [egui::Rect; 2] {
    let mut rects = [egui::Rect::NOTHING; 2];
    let _ = ctx.run(
        egui::RawInput {
            time: Some(time),
            ..frame_input(vec![])
        },
        |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| rects = app.columns_ui(ui, false));
        },
    );
    rects
}

#[test]
fn without_a_declared_panel_the_rail_keeps_no_state_and_no_area() {
    let (ctx, mut app) = demo_app();
    app.dialogs.side_rail.open = Some("test");
    let rects = columns(&mut app, &ctx, 1.0);
    assert_eq!(app.dialogs.side_rail.open, None);
    assert_eq!(rects[1].right(), 1240.0);
    assert!(
        ctx.memory(|m| m.area_rect(egui::Id::new("side-rail-drawer")))
            .is_none()
    );
}

#[test]
fn an_open_drawer_overlays_the_preview_without_moving_the_editor_or_its_focus() {
    let (ctx, mut app) = demo_app();
    let before = columns(&mut app, &ctx, 1.0);
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(editor::EDITOR_ID)));
    let _ = columns(&mut app, &ctx, 1.1);
    let selection = app.document.selection;
    app.dialogs.side_panels.push(panel());
    app.dialogs.side_rail.open = Some("test");
    for time in [1.2, 1.3, 1.4] {
        let after = columns(&mut app, &ctx, time);
        assert_eq!(after, before, "la mise en page des colonnes ne change pas");
    }
    let drawer = ctx
        .memory(|m| m.area_rect(egui::Id::new("side-rail-drawer")))
        .expect("tiroir ouvert");
    assert!(
        drawer.width() <= before[1].width() * 2.0 / 3.0 + 25.0,
        "{drawer:?}"
    );
    assert!(drawer.left() >= before[1].left());
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
    assert_eq!(app.document.selection, selection);
}

#[test]
fn escape_closes_the_drawer_without_a_declared_command() {
    let (ctx, mut app) = demo_app();
    app.dialogs.side_panels.push(panel());
    app.dialogs.side_rail.open = Some("test");
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.dialogs.side_rail.open, Some("test"));
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert_eq!(app.dialogs.side_rail.open, None);
}
