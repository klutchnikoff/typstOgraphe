mod app;
mod commands;
mod document;
mod editor;
mod i18n;
mod menus;
mod navigation;
mod rendering;
mod storage;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // La même langue d’interface est utilisée par l’UI et le thread de rendu.
    let _ = i18n::loader();
    let mut args = std::env::args().skip(1);
    let mut capture = None;
    let mut open_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--render-demo" => {
                let dir = args.next().ok_or(crate::tr!("cli-usage-render"))?;
                let mut world = rendering::TypstWorld::default();
                let document = document::Document::demo();
                let request = rendering::RenderRequest {
                    revision: 1,
                    source: document.source().into(),
                    source_map: document.source_map.clone(),
                    metadata: document::Metadata::default(),
                    compact: false,
                    template: rendering::TEMPLATE.into(),
                    template_contract: document::TemplateContract::DocumentOptions,
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
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, capture, open_path)?))),
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
