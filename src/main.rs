mod annotations;
mod app;
mod assets;
mod commands;
mod document;
mod document_types;
mod editor;
mod i18n;
mod icons;
mod menus;
mod navigation;
mod preferences;
mod recovery;
mod rendering;
mod search;
mod session;
mod statistics;
mod storage;
#[cfg(test)]
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
    let mut open_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
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
            "--capture-search" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or(crate::tr!("cli-usage-capture-search"))?,
                ));
                capture_search = Some(args.next().ok_or(crate::tr!("cli-usage-capture-search"))?);
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
