mod app;
mod document;
mod editor;
mod rendering;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut capture = None;
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--render-demo" => {
                let dir = args.next().ok_or("Usage : --render-demo DOSSIER")?;
                let mut world = rendering::TypstWorld::default();
                let document = document::Document::demo();
                let request = rendering::RenderRequest {
                    revision: 1,
                    source: document.source().into(),
                    metadata: document::Metadata::default(),
                    compact: false,
                    export_dir: Some(PathBuf::from(dir)),
                };
                let result = rendering::render(&mut world, &request)?;
                println!(
                    "{} page(s), {} ms, export : {}",
                    result.pages.len(),
                    result.elapsed.as_millis(),
                    result.export_dir.unwrap().display()
                );
                return Ok(());
            }
            "--capture" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or("Usage : --capture IMAGE.png")?,
                ))
            }
            "--help" | "-h" => {
                println!(
                    "typstOgraphe\n  cargo run\n  cargo run -- --render-demo DOSSIER\n  cargo run -- --capture IMAGE.png"
                );
                return Ok(());
            }
            _ => return Err(format!("Option inconnue : {arg}").into()),
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("typstOgraphe — Courrier")
            .with_inner_size([1240.0, 900.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "typstOgraphe",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, capture)))),
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
