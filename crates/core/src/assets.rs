//! Ressources binaires du document, aux chemins relatifs contrôlés.
use comemo::Track;
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};
use typst::{
    foundations::Bytes,
    visualize::{ExchangeFormat, Image, RasterImage, SvgImage},
};

pub type Assets = BTreeMap<String, Arc<[u8]>>;
pub const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    thiserror::Error,
)]
pub enum ImageError {
    #[error("image exceeds the size limit")]
    TooLarge,
    #[error("unsupported image format")]
    Unsupported,
    #[error("invalid SVG image")]
    InvalidSvg,
    #[error("{0}")]
    Decode(String),
    #[error("{0}")]
    Io(String),
}

#[derive(Clone, Debug)]
pub struct ImageData {
    pub extension: String,
    pub data: Arc<[u8]>,
}
impl ImageData {
    pub fn read(path: &std::path::Path) -> Result<Self, ImageError> {
        use std::io::Read;
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let extension = if extension == "jpeg" {
            "jpg".into()
        } else {
            extension
        };
        let mut data = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| ImageError::Io(e.to_string()))?
            .take(MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|e| ImageError::Io(e.to_string()))?;
        decode(&data, &extension)?;
        Ok(Self {
            extension,
            data: data.into(),
        })
    }
    /// Ne remplace aucune ancienne ressource : les instantanés d'annulation la gardent.
    pub fn insert(&self, assets: &mut Assets, stem: &str) -> String {
        let mut number = 1;
        loop {
            let path = format!("images/{stem}-{number}.{}", self.extension);
            if !assets.contains_key(&path) {
                assets.insert(path.clone(), self.data.clone());
                return path;
            }
            number += 1;
        }
    }
}

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

pub fn decode(data: &[u8], extension: &str) -> Result<Image, ImageError> {
    if data.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::TooLarge);
    }
    let bytes = Bytes::new(data.to_vec());
    match extension {
        "png" => RasterImage::plain(bytes, ExchangeFormat::Png)
            .map(Image::plain)
            .map_err(|e| ImageError::Decode(e.to_string())),
        "jpg" | "jpeg" => RasterImage::plain(bytes, ExchangeFormat::Jpg)
            .map(Image::plain)
            .map_err(|e| ImageError::Decode(e.to_string())),
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
            .map_err(|_| ImageError::InvalidSvg)
        }
        _ => Err(ImageError::Unsupported),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn image_decoder_checks_formats_dimensions_and_resource_paths() {
        for (extension, data, ratio) in [
            (
                "png",
                include_bytes!("../../../fixtures/colours.png").as_slice(),
                1.0,
            ),
            (
                "jpg",
                include_bytes!("../../../fixtures/colours.jpg").as_slice(),
                1.0,
            ),
            (
                "svg",
                include_bytes!("../../../fixtures/parc.svg").as_slice(),
                2.4,
            ),
        ] {
            let image = decode(data, extension).unwrap();
            assert!((image.width() / image.height() - ratio).abs() < 0.001);
        }
        assert!(decode(b"not an image", "png").is_err());
        assert!(decode(include_bytes!("../../../fixtures/parc.svg"), "jpg").is_err());
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
