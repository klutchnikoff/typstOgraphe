//! Écriture des captures natives de développement.
use std::path::Path;

pub(crate) fn write_screenshot(
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
