use super::*;

#[test]
fn native_capture_ignores_live_input_but_regular_editing_keeps_it() {
    for capture in [false, true] {
        let ctx = egui::Context::default();
        let mut app = App::with_demo_context(
            ctx.clone(),
            capture.then(|| PathBuf::from("unused-capture.png")),
        );
        let screenshot = egui::Event::Screenshot {
            viewport_id: egui::ViewportId::ROOT,
            user_data: egui::UserData::default(),
            image: std::sync::Arc::new(egui::ColorImage::new([1, 1], vec![egui::Color32::WHITE])),
        };
        let mut input = frame_input(vec![
            egui::Event::Text("Texte extérieur".into()),
            egui::Event::Ime(egui::ImeEvent::Preedit("にほん".into())),
            screenshot,
        ]);
        input.focused = false;
        input.modifiers = egui::Modifiers::CTRL;
        eframe::App::raw_input_hook(&mut app, &ctx, &mut input);
        assert!(matches!(
            input.events.last(),
            Some(egui::Event::Screenshot { .. })
        ));
        if capture {
            assert_eq!(input.events.len(), 1);
            assert!(input.focused);
            assert_eq!(input.modifiers, egui::Modifiers::NONE);
        } else {
            assert_eq!(input.events.len(), 3);
            assert!(!input.focused);
            assert_eq!(input.modifiers, egui::Modifiers::CTRL);
        }
    }
}
