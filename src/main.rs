#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod annotations;
mod app;
mod assets;
mod commands;
mod dates;
mod document;
mod document_types;
mod editor;
mod engine;
mod errors;
mod exporting;
mod fonts;
mod i18n;
mod icons;
mod identity;
mod menus;
mod navigation;
mod packages;
mod preferences;
mod profile;
mod recovery;
mod rendering;
mod search;
mod session;
mod spelling;
mod statistics;
mod storage;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
mod toolbar;

use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // La même langue d’interface est utilisée par l’UI et le thread de rendu.
    let _ = i18n::loader();
    let mut args = std::env::args().skip(1);
    let mut capture = None;
    let mut capture_recovery = None;
    let mut capture_search = None;
    let mut capture_spelling = false;
    let mut capture_profile = None;
    let mut capture_creation = false;
    let mut open_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--commands-catalogue" | "--commands-catalogue-json" => {
                let value = engine::registry::catalogue_json();
                if arg.ends_with("-json") {
                    println!("{}", serde_json::to_string_pretty(&value)?);
                } else {
                    print!("{}", engine::registry::catalogue_markdown(&value));
                }
                return Ok(());
            }
            "--render-demo" => {
                let dir = args.next().ok_or(crate::tr!("cli-usage-render"))?;
                let mut world = rendering::TypstWorld::default();
                let kind = document_types::DocumentType::Courrier;
                let document = document::Document::from_source(kind.demo_source())?;
                let request = rendering::RenderRequest {
                    revision: 1,
                    assets: Default::default(),
                    source: document.source().into(),
                    source_map: document.source_map.clone(),
                    metadata: document_types::Metadata::demo(
                        document_types::DocumentType::Courrier,
                    ),
                    dependencies: packages::for_template(&kind.default_model()),
                    template: kind.default_model(),
                    export_dir: Some(PathBuf::from(dir)),
                };
                let result = rendering::render(&mut world, &request)?;
                println!(
                    "{}",
                    crate::tr!(
                        "cli-render-result",
                        pages = result.pages.len(),
                        milliseconds = result.elapsed.as_millis() as f64,
                        path = result.export_dir.unwrap().display().to_string()
                    )
                );
                return Ok(());
            }
            "--capture" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or(crate::tr!("cli-usage-capture"))?,
                ))
            }
            "--capture-new-document" => {
                capture = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-creation"))?,
                ));
                capture_creation = true;
            }
            "--capture-search" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or(crate::tr!("cli-usage-capture-search"))?,
                ));
                capture_search = Some(args.next().ok_or(crate::tr!("cli-usage-capture-search"))?);
            }
            "--capture-spelling" => {
                capture = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-spelling"))?,
                ));
                capture_spelling = true;
            }
            "--capture-profile" | "--capture-profile-letter" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or(crate::tr!("cli-usage-capture-profile"))?,
                ));
                capture_profile = Some(arg == "--capture-profile");
            }
            "--capture-recovery" => {
                capture = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-recovery"))?,
                ));
                capture_recovery = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-recovery"))?,
                ));
            }
            "--help" | "-h" => {
                println!("{}", crate::tr!("cli-help"));
                return Ok(());
            }
            _ if !arg.starts_with('-') && open_path.is_none() => {
                open_path = Some(PathBuf::from(arg))
            }
            _ => return Err(crate::tr!("cli-unknown-option", option = arg).into()),
        }
    }
    let options = eframe::NativeOptions {
        persist_window: capture.is_none(),
        viewport: eframe::egui::ViewportBuilder::default()
            // eframe emploie aussi ce titre initial pour le menu de l’application
            // macOS. Le titre du document est renseigné à la première frame.
            .with_title("typstOgraphe")
            .with_inner_size([1240.0, 900.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "typstOgraphe",
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc, capture, open_path, capture_recovery)?;
            if let Some(query) = capture_search {
                app.capture_search(query);
            }
            if capture_spelling {
                app.capture_spelling(&cc.egui_ctx);
            }
            if let Some(preferences) = capture_profile {
                app.capture_profile(&cc.egui_ctx, preferences);
            }
            if capture_creation {
                app.capture_creation(&cc.egui_ctx);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_screenshot(
    path: &Path,
    image: &eframe::egui::ColorImage,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(writer, image.width() as u32, image.height() as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let data: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|p| p.to_srgba_unmultiplied())
        .collect();
    encoder.write_header()?.write_image_data(&data)?;
    Ok(())
}
