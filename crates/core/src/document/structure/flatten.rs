use super::*;
use crate::document::IMAGE_OBJECT;

impl Structure {
    /// Adaptateur vers le tampon et le sérialiseur existants. Les frontières
    /// internes sont reconstruites depuis les rôles, pas conservées dans les zones.
    pub fn to_projection(&self) -> Projection {
        let mut view = Projection {
            typing_style: self.typing_style.clone(),
            ..Projection::default()
        };
        for (index, block) in self.blocks.iter().enumerate() {
            if index > 0 {
                view.push("\n", &InlineStyle::default());
            }
            match &block.content {
                BlockContent::Text { kind, body } => append(&mut view, body, *kind),
                BlockContent::Description { term, definition } => {
                    append(&mut view, term, BlockKind::Description);
                    view.push("\t", &InlineStyle::default());
                    append(&mut view, definition, BlockKind::Description);
                }
                BlockContent::Quote { body, attribution } => {
                    append(&mut view, body, BlockKind::Quote);
                    view.push("\n", &InlineStyle::default());
                    append(&mut view, attribution, BlockKind::QuoteAttribution);
                }
                BlockContent::Image {
                    resource,
                    image,
                    caption,
                } => {
                    view.push(&IMAGE_OBJECT.to_string(), &InlineStyle::default());
                    append(&mut view, caption, BlockKind::Image(*resource));
                    view.images.insert(*resource, image.clone());
                }
            }
        }
        view.line_ids = self
            .blocks
            .iter()
            .flat_map(|block| block.lines.iter().copied())
            .collect();
        debug_assert_eq!(view.line_ids.len(), view.blocks.len());
        view
    }
}

fn append(view: &mut Projection, zone: &TextZone, kind: BlockKind) {
    let line = view.blocks.len() - 1;
    view.push(&zone.view.text, &InlineStyle::default());
    let start = view.styles.len() - zone.view.styles.len();
    view.styles[start..].clone_from_slice(&zone.view.styles);
    view.blocks[line..].fill(kind);
}
