use super::{targets::*, *};
use crate::document::Document;

#[test]
fn every_unicode_caret_round_trips_in_both_directions_across_all_roles() {
    let document = Document::from_source("= É\n\nUn é 👩‍🔬\nNoël\n\n/ Mot: Définition\n\n#quote(block: true, attribution: [Autrice])[Une idée.]\n\n#figure(image(\"images/parc.svg\"), caption: [Un été])").unwrap();
    let len = document.view.len();
    for anchor in 0..=len {
        let head = len - anchor;
        let target = document.structure.selection_at(anchor, head).unwrap();
        assert_eq!(
            document.structure.resolve_selection(target),
            Ok((anchor, head))
        );
    }
    let document = Document::from_source("é 👩‍🔬 été").unwrap();
    let selection = document.structure.selection_at(2, 6).unwrap();
    let Position::Text(anchor) = selection.anchor else {
        panic!("text")
    };
    let Position::Text(head) = selection.head else {
        panic!("text")
    };
    assert_eq!((anchor.offset, head.offset), (2, 6));
    assert_ne!(document.view.byte_index(6), 6);
}

#[test]
fn empty_annotations_and_image_boundaries_have_distinct_logical_targets() {
    let document =
        Document::from_source("#quote(block: true)[Texte]\n\n#figure(image(\"images/parc.svg\"))")
            .unwrap();
    let quote = &document.structure.blocks[0];
    let image = document.structure.blocks.last().unwrap();
    let BlockContent::Quote { attribution, .. } = &quote.content else {
        panic!("quote")
    };
    let BlockContent::Image { caption, .. } = &image.content else {
        panic!("image")
    };
    let last = document.view.len();
    let caption_target = document.structure.selection_at(last, last).unwrap();
    assert_eq!(
        caption_target.anchor,
        Position::Text(TextPosition {
            zone_id: caption.id,
            offset: 0
        })
    );
    assert_eq!(
        document
            .structure
            .selection_at(last - 1, last - 1)
            .unwrap()
            .anchor,
        Position::Before(image.id)
    );
    let whole_image = LogicalSelection {
        anchor: Position::Before(image.id),
        head: Position::After(image.id),
    };
    assert_eq!(
        document.structure.resolve_selection(whole_image),
        Ok((last - 1, last))
    );
    let author = LogicalSelection {
        anchor: Position::Text(TextPosition {
            zone_id: attribution.id,
            offset: 0,
        }),
        head: caption_target.head,
    };
    assert!(document.structure.resolve_selection(author).unwrap().0 < last - 1);
}

#[test]
fn offsets_and_unknown_identities_are_rejected_without_clamping() {
    let document = Document::from_source("Été").unwrap();
    let valid = document.structure.selection_at(0, 3).unwrap();
    assert_eq!(
        document.structure.selection_at(4, 0),
        Err(TargetError::InvalidPosition)
    );
    assert_eq!(
        document.structure.selection_at(0, usize::MAX),
        Err(TargetError::InvalidPosition)
    );
    let Position::Text(mut text) = valid.head else {
        panic!("text")
    };
    text.offset = usize::MAX;
    assert_eq!(
        document.structure.resolve_selection(LogicalSelection {
            head: Position::Text(text),
            ..valid
        }),
        Err(TargetError::InvalidPosition)
    );
    text.offset = 0;
    text.zone_id = ZoneId(fresh_id());
    assert_eq!(
        document.structure.resolve_selection(LogicalSelection {
            head: Position::Text(text),
            ..valid
        }),
        Err(TargetError::UnknownZone)
    );
    assert_eq!(
        document.structure.resolve_selection(LogicalSelection {
            head: Position::After(BlockId(fresh_id())),
            ..valid
        }),
        Err(TargetError::UnknownBlock)
    );
}
