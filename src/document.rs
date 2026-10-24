//! Projection éditable du sous-ensemble Typst. Les indices publics sont des
//! indices de caractères Unicode (comme les curseurs egui), jamais des octets.
use std::{collections::BTreeMap, ops::Range};

#[path = "document/structure.rs"]
pub mod structure;
use structure::{LineId, Structure};
#[path = "document/edits.rs"]
pub mod edits;

pub const IMAGE_OBJECT: char = '\u{fffc}';
pub const LETTER_NOTES_IMPORT: &str = "#import \"modele.typ\": postscript, attachment\n\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageBlock {
    pub path: String,
    pub alt: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub link: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlockKind {
    #[default]
    Paragraph,
    Heading(u8),
    Bullet,
    Numbered,
    Description,
    Quote,
    QuoteAttribution,
    Postscript,
    Attachment,
    Image(u64),
}
impl BlockKind {
    pub fn is_letter_note(self) -> bool {
        matches!(self, Self::Postscript | Self::Attachment)
    }
    pub fn is_list(self) -> bool {
        matches!(self, Self::Bullet | Self::Numbered | Self::Description)
    }
    pub fn public_kind(self) -> Self {
        match self {
            Self::QuoteAttribution => Self::Quote,
            Self::Image(_) => Self::Paragraph,
            _ => self,
        }
    }
}

pub struct QuoteGroup {
    pub body: Range<usize>,
    pub attribution: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Bold,
    Italic,
}

/// Une seule chaîne visible : la sélection n'est jamais coupée aux frontières
/// des paragraphes. Les styles sont une projection du source, non un format disque.
#[derive(Clone, Debug)]
pub struct Projection {
    pub text: String,
    pub styles: Vec<InlineStyle>,
    pub blocks: Vec<BlockKind>,
    pub typing_style: InlineStyle,
    pub images: BTreeMap<u64, ImageBlock>,
    /// Ancres transitoires, conservées par les instantanés mais jamais exportées.
    line_ids: Vec<LineId>,
    edit_error: Option<edits::EditError>,
}

impl PartialEq for Projection {
    fn eq(&self, other: &Self) -> bool {
        self.same_content(other) && self.typing_style == other.typing_style
    }
}
impl Eq for Projection {}

impl Default for Projection {
    fn default() -> Self {
        Self {
            text: String::new(),
            styles: vec![],
            blocks: vec![BlockKind::Paragraph],
            typing_style: InlineStyle::default(),
            images: BTreeMap::new(),
            line_ids: vec![LineId::new()],
            edit_error: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    source: String,
    pub source_map: SourceMap,
    pub view: Projection,
    pub assets: crate::assets::Assets,
    pub structure: Structure,
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.same_content(other) && self.view.typing_style == other.view.typing_style
    }
}
impl Eq for Document {}

/// Chaque caractère visible pointe vers sa plage d'octets dans le source réel.
/// Les commandes et les délimiteurs ne sont jamais comptés comme du texte.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub characters: Vec<Range<usize>>,
    pub source_len: usize,
}
impl SourceMap {
    pub fn to_source(&self, character: usize) -> usize {
        self.characters
            .get(character)
            .map_or_else(|| self.characters.last().map_or(0, |r| r.end), |r| r.start)
    }
    pub fn to_visible(&self, byte: usize) -> usize {
        self.characters
            .iter()
            .enumerate()
            .min_by_key(|(_, r)| {
                if byte < r.start {
                    r.start - byte
                } else if byte >= r.end {
                    byte.saturating_sub(r.end) + 1
                } else {
                    0
                }
            })
            .map_or(0, |(i, r)| if byte >= r.end { i + 1 } else { i })
    }
}

impl Document {
    pub(crate) fn same_content(&self, other: &Self) -> bool {
        self.source == other.source
            && self.source_map == other.source_map
            && self.view.same_content(&other.view)
            && self.assets == other.assets
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn demo() -> Self {
        Self::from_source(include_str!("../examples/courrier.typ"))
            .expect("courrier d’exemple valide")
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Toute opération d'édition actualise le source qui sera compilé/exporté.
    /// Renvoie les séparateurs dans l’ordre des insertions afin que l’UI puisse
    /// conserver la sélection en créant les zones de description et d’attribution.
    pub fn sync_source(&mut self) -> Vec<usize> {
        let mut inserted = self.view.ensure_description_separators();
        inserted.extend(self.view.ensure_quote_attributions());
        inserted.extend(self.view.ensure_list_boundaries());
        self.structure.rebuild(&self.view);
        let committed = self.structure.to_projection();
        debug_assert_eq!(committed, self.view);
        (self.source, self.source_map) = committed.to_typst_mapped();
        self.view.edit_error = None;
        inserted
    }
}

#[path = "document/input.rs"]
mod input;
#[path = "document/options.rs"]
mod options;
#[path = "document/parsing.rs"]
mod parsing;
#[path = "document/projection.rs"]
mod projection;
#[path = "document/serialization.rs"]
mod serialization;
pub use options::*;
pub(crate) use serialization::typst_string;
