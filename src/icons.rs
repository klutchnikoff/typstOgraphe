//! Sous-ensemble Lucide embarqué : masques monochromes, cache par densité d’écran.
use eframe::egui::{self, Color32, ColorImage, TextureHandle};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Icon {
    Paragraph,
    Heading,
    Heading1,
    Heading2,
    Heading3,
    Bold,
    Italic,
    Link,
    List,
    ListOrdered,
    ListDescription,
    Quote,
    Image,
    ChevronDown,
}

impl Icon {
    fn svg(self) -> &'static str {
        match self {
            Self::Paragraph => include_str!("../assets/icons/lucide/pilcrow.svg"),
            Self::Heading => include_str!("../assets/icons/lucide/heading.svg"),
            Self::Heading1 => include_str!("../assets/icons/lucide/heading-1.svg"),
            Self::Heading2 => include_str!("../assets/icons/lucide/heading-2.svg"),
            Self::Heading3 => include_str!("../assets/icons/lucide/heading-3.svg"),
            Self::Bold => include_str!("../assets/icons/lucide/bold.svg"),
            Self::Italic => include_str!("../assets/icons/lucide/italic.svg"),
            Self::Link => include_str!("../assets/icons/lucide/link.svg"),
            Self::List => include_str!("../assets/icons/lucide/list.svg"),
            Self::ListOrdered => include_str!("../assets/icons/lucide/list-ordered.svg"),
            Self::ListDescription => include_str!("../assets/icons/lucide/list-collapse.svg"),
            Self::Quote => include_str!("../assets/icons/lucide/quote.svg"),
            Self::Image => include_str!("../assets/icons/lucide/image-plus.svg"),
            Self::ChevronDown => include_str!("../assets/icons/lucide/chevron-down.svg"),
        }
    }

    pub fn for_block(kind: crate::document::BlockKind) -> Self {
        use crate::document::BlockKind;
        match kind {
            BlockKind::Paragraph => Self::Paragraph,
            BlockKind::Heading(1) => Self::Heading1,
            BlockKind::Heading(2) => Self::Heading2,
            BlockKind::Heading(3) => Self::Heading3,
            BlockKind::Heading(_) => Self::Heading,
            BlockKind::Bullet => Self::List,
            BlockKind::Numbered => Self::ListOrdered,
            BlockKind::Description => Self::ListDescription,
            BlockKind::Quote | BlockKind::QuoteAttribution => Self::Quote,
            BlockKind::Image(_) => Self::Image,
        }
    }

    fn raster(self, pixels: u32) -> ColorImage {
        let svg = self.svg().replace("currentColor", "#ffffff");
        let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &Default::default())
            .expect("icône SVG embarquée valide");
        let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels).expect("taille d’icône");
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(
                pixels as f32 / tree.size().width(),
                pixels as f32 / tree.size().height(),
            ),
            &mut pixmap.as_mut(),
        );
        ColorImage::from_rgba_premultiplied([pixels as usize, pixels as usize], pixmap.data())
    }

    pub fn texture(self, ctx: &egui::Context, size: f32) -> TextureHandle {
        let pixels = (size * ctx.pixels_per_point()).ceil().max(1.0) as u32;
        let id = egui::Id::new(("lucide", self, pixels));
        if let Some(texture) = ctx.data_mut(|data| data.get_temp::<TextureHandle>(id)) {
            return texture;
        }
        let texture = ctx.load_texture(
            format!("lucide-{self:?}-{pixels}"),
            self.raster(pixels),
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    }

    pub fn image(self, ctx: &egui::Context, size: f32) -> egui::Image<'static> {
        egui::Image::new((self.texture(ctx, size).id(), egui::Vec2::splat(size)))
            .tint(Color32::WHITE)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn bundled_icons_render_as_transparent_monochrome_masks_at_screen_density() {
        for icon in [
            Icon::Paragraph,
            Icon::Heading,
            Icon::Heading1,
            Icon::Heading2,
            Icon::Heading3,
            Icon::Bold,
            Icon::Italic,
            Icon::Link,
            Icon::List,
            Icon::ListOrdered,
            Icon::ListDescription,
            Icon::Quote,
            Icon::Image,
            Icon::ChevronDown,
        ] {
            for pixels in [18, 36, 54] {
                let image = icon.raster(pixels);
                assert_eq!(image.size, [pixels as usize; 2]);
                assert!(image.pixels.iter().any(|pixel| pixel.a() > 0), "{icon:?}");
                assert!(image.pixels.iter().any(|pixel| pixel.a() == 0));
                assert!(
                    image
                        .pixels
                        .iter()
                        .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b())
                );
            }
        }
    }
}
