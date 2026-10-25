//! Isolated native acceptance owner. Only built explicitly with mcp-acceptance.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("pass a new absolute diagnostic directory")?;
    typstographe::i18n::initialize(&["fr".parse()?]);
    let options = eframe::NativeOptions {
        persist_window: false,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("typstOgraphe — recette isolée")
            .with_inner_size([1240.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "typstOgraphe — recette isolée",
        options,
        Box::new(move |cc| {
            Ok(Box::new(typstographe::app::App::acceptance(
                cc,
                root.into(),
            )?))
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
