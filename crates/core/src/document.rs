//! Document options and source mapping shared by the structured editor and Typst.
mod markup;
mod options;
mod source_map;
pub use markup::{escape_markup, typst_string};
pub use options::*;
pub use source_map::SourceMap;
