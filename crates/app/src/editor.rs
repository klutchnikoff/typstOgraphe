use crate::preferences::{EditorFont, EditorStyle};
use crate::{
    annotations::AnnotationKind,
    document::{BlockKind, InlineStyle, Projection},
};
use eframe::egui::{
    self, Color32, FontFamily, Stroke, TextBuffer, TextFormat,
    text::{CCursor, CCursorRange, Galley, LayoutJob},
};
use std::{ops::Range, sync::Arc};

pub const EDITOR_ID: &str = "corps-courrier";
const DESCRIPTION_LABEL_SCALE: f32 = 0.9;
const IMAGE_HEIGHT: f32 = 220.0;
const IMAGE_PADDING: f32 = 10.0;
const IMAGE_CAPTION_GAP: f32 = 8.0;
const ANNOTATION_SCALE: f32 = 0.9;

#[path = "editor/buffer.rs"]
mod buffer;
#[path = "editor/decorations.rs"]
mod decorations;
#[path = "editor/descriptions.rs"]
mod descriptions;
#[path = "editor/fonts.rs"]
mod fonts;
#[path = "editor/geometry.rs"]
mod geometry;
#[path = "editor/images.rs"]
mod images;
#[path = "editor/layout.rs"]
mod layout;
pub use buffer::{InputBuffer, ProjectionBuffer, to_core, to_editor};
#[cfg(test)]
use decorations::link_underlines;
pub use decorations::paint_decorations;
use descriptions::*;
pub use fonts::install_fonts;
pub use geometry::{cursor, paint_spelling, range_rects, restore_cursor, store_cursor};
use images::image_frame;
pub use images::{ImageAction, paint_images};
use layout::*;
pub use layout::{layout, layout_with_search};
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "editor/tests.rs"]
mod tests;
