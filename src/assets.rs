//! Ressources binaires du document, aux chemins relatifs contrôlés.
use comemo::Track;
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};
use typst::{
    foundations::{Bytes, Content, Smart},
    layout::{Abs, Frame, FrameItem, FrameKind, Point, Sides, Size},
    syntax::Span,
    visualize::{ExchangeFormat, Image, RasterImage, SvgImage},
};

pub type Assets = BTreeMap<String, Arc<[u8]>>;
pub const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;

pub fn valid_path(path: &str) -> bool {
    let Some(name) = path.strip_prefix("images/") else {
        return false;
    };
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    path.len() <= 128
        && !stem.is_empty()
        && stem
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        && matches!(extension, "png" | "jpg" | "svg")
}

pub fn decode(data: &[u8], extension: &str) -> Result<Image, String> {
    if data.len() > MAX_IMAGE_BYTES {
        return Err(crate::tr!("image-too-large"));
    }
    let bytes = Bytes::new(data.to_vec());
    match extension {
        "png" => RasterImage::plain(bytes, ExchangeFormat::Png)
            .map(Image::plain)
            .map_err(|e| e.to_string()),
        "jpg" | "jpeg" => RasterImage::plain(bytes, ExchangeFormat::Jpg)
            .map(Image::plain)
            .map_err(|e| e.to_string()),
        "svg" => {
            static FONTS: OnceLock<crate::rendering::TypstWorld> = OnceLock::new();
            let world = FONTS.get_or_init(crate::rendering::TypstWorld::default);
            SvgImage::with_fonts_images(
                bytes,
                (world as &dyn typst::World).track(),
                &["Libertinus Serif"],
                None,
            )
            .map(Image::plain)
            .map_err(|_| crate::tr!("image-invalid-svg"))
        }
        _ => Err(crate::tr!("image-format-unsupported")),
    }
}

/// La vignette utilise le même décodeur et le même moteur que l’aperçu,
/// sans dépendre des règles graphiques du modèle.
pub fn thumbnail(data: &[u8], extension: &str) -> Result<eframe::egui::ColorImage, String> {
    let image = decode(data, extension)?;
    let scale = (640.0 / image.width()).min(400.0 / image.height()).min(1.0);
    let size = Size::new(
        Abs::pt(image.width() * scale),
        Abs::pt(image.height() * scale),
    );
    let mut frame = Frame::new(size, FrameKind::Hard);
    frame.push(
        Point::zero(),
        FrameItem::Image(image, size, Span::detached()),
    );
    let page = typst_layout::Page {
        frame,
        bleed: Sides::default(),
        fill: Smart::Custom(None),
        numbering: None,
        supplement: Content::empty(),
        number: 1,
    };
    let pixmap = typst_render::render(&page, &typst_render::RenderOptions::default());
    Ok(eframe::egui::ColorImage::from_rgba_premultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    ))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn thumbnails_decode_png_jpeg_and_svg_with_their_actual_aspect_ratio() {
        for (extension, data, ratio) in [
            (
                "png",
                include_bytes!("../tests/fixtures/colours.png").as_slice(),
                1.0,
            ),
            (
                "jpg",
                include_bytes!("../tests/fixtures/colours.jpg").as_slice(),
                1.0,
            ),
            (
                "svg",
                include_bytes!("../tests/fixtures/parc.svg").as_slice(),
                2.4,
            ),
        ] {
            let image = decode(data, extension).unwrap();
            assert!((image.width() / image.height() - ratio).abs() < 0.001);
            let thumbnail = thumbnail(data, extension).unwrap();
            assert!((thumbnail.width() as f64 / thumbnail.height() as f64 - ratio).abs() < 0.01);
            assert!(thumbnail.pixels.iter().any(|p| p.a() > 0));
        }
        assert!(decode(b"not an image", "png").is_err());
        assert!(decode(include_bytes!("../tests/fixtures/parc.svg"), "jpg").is_err());
        assert!(decode(&vec![0; MAX_IMAGE_BYTES + 1], "svg").is_err());
        for path in [
            "../file.png",
            "images/../file.png",
            "images/nested/a.svg",
            "images/a.exe",
            "images/a\\b.png",
        ] {
            assert!(!valid_path(path));
        }
    }
}
