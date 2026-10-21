//! Annotations facultatives de la projection éditable, indépendantes de leur
//! disposition et du modèle Typst. Tous les indices désignent des caractères.
use crate::document::{BlockKind, Projection};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    ImageCaption,
    QuoteAttribution,
}
impl AnnotationKind {
    pub fn for_block(block: BlockKind) -> Option<Self> {
        match block {
            BlockKind::Image(_) => Some(Self::ImageCaption),
            BlockKind::QuoteAttribution => Some(Self::QuoteAttribution),
            _ => None,
        }
    }
    pub fn placeholder(self) -> String {
        match self {
            Self::ImageCaption => crate::tr!("image-caption-placeholder"),
            Self::QuoteAttribution => crate::tr!("quote-attribution-placeholder"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotationBody {
    Text(usize),
    Object(Range<usize>),
}

/// Décrit les cibles de navigation, sans embarquer de règle de géométrie ou
/// d’événement clavier. Un objet se sélectionne ; un corps textuel reçoit le curseur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionalAnnotation {
    pub kind: AnnotationKind,
    pub structure: Range<usize>,
    pub body: AnnotationBody,
    pub content: Range<usize>,
}
impl OptionalAnnotation {
    pub fn main_target(&self) -> Range<usize> {
        match &self.body {
            AnnotationBody::Text(at) => *at..*at,
            AnnotationBody::Object(range) => range.clone(),
        }
    }
    pub fn contains(&self, selection: &Range<usize>) -> bool {
        self.structure.start <= selection.start && selection.end <= self.structure.end
    }
    pub fn enter_leaves(&self, selection: &Range<usize>) -> bool {
        self.contains(selection)
            && (matches!(self.body, AnnotationBody::Object(_))
                || selection.start >= self.content.start)
    }
}

impl Projection {
    pub fn annotations(&self) -> Vec<OptionalAnnotation> {
        let ranges = self.line_ranges();
        let mut annotations = Vec::new();
        for (line, range) in ranges.iter().enumerate() {
            if matches!(self.blocks[line], BlockKind::Image(_)) {
                annotations.push(OptionalAnnotation {
                    kind: AnnotationKind::ImageCaption,
                    structure: range.clone(),
                    body: AnnotationBody::Object(range.start..range.start + 1),
                    content: range.start + 1..range.end,
                });
            }
        }
        for group in self.quote_groups() {
            if let Some(line) = group.attribution {
                annotations.push(OptionalAnnotation {
                    kind: AnnotationKind::QuoteAttribution,
                    structure: ranges[group.body.start].start..ranges[line].end,
                    body: AnnotationBody::Text(ranges[group.body.start].start),
                    content: ranges[line].clone(),
                });
            }
        }
        annotations.sort_by_key(|annotation| annotation.structure.start);
        annotations
    }
    pub fn annotation_at(&self, selection: Range<usize>) -> Option<OptionalAnnotation> {
        self.annotations()
            .into_iter()
            .find(|annotation| annotation.contains(&selection))
    }
    /// Quitter une annotation crée un paragraphe normal après toute la structure.
    /// La légende ou l’attribution reste intacte, même avec une sélection active.
    pub fn leave_annotation(&mut self, annotation: &OptionalAnnotation) -> usize {
        let at = annotation.structure.end;
        self.insert_text("\n", at);
        let next = at + 1;
        let line = self.line_at(next);
        self.blocks[line] = BlockKind::Paragraph;
        next
    }
}
