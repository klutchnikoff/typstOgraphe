use super::*;
use crate::{
    document::{Document, IMAGE_OBJECT, Mark},
    document_types::DocumentType,
    session::{Selection, Session},
    storage::StoredDocument,
};

fn identities(document: &Document) -> Vec<(BlockId, Vec<ZoneId>)> {
    document
        .structure
        .blocks
        .iter()
        .map(|block| {
            (
                block.id,
                block.content.zones().iter().map(|zone| zone.id).collect(),
            )
        })
        .collect()
}

#[test]
fn all_current_blocks_round_trip_without_changing_source_maps_styles_or_assets() {
    for source in [
        "",
        "Été\nNoël\n\nSuite.\n",
        "= Un *titre*\n\n== Deux\n\n=== Trois",
        "- *Un*\n- Deux\n\n+ Trois\n+ Quatre",
        "/ *Terme*: Une _définition_\n/ Autre: ",
        "#quote(block: true, attribution: [Une _autrice_])[Un *été*\n\nÀ Noël.]",
        "#quote(block: true)[]",
        "#figure(image(\"images/parc.svg\", alt: \"Un parc\"), caption: [Un *été*.])",
        "#figure(image(\"images/parc.svg\"))",
        "#import \"modele.typ\": postscript, attachment\n\nUn courrier.\n\n#postscript[Une *note*.\n\nSa suite.]\n\n#attachment[Un formulaire]\n#attachment[Une pièce]",
        "#link(\"https://example.org\")[Un *lien*] et du _texte_.",
    ] {
        let mut document = Document::from_source(source).unwrap();
        document.assets.insert(
            "images/parc.svg".into(),
            include_bytes!("../../../../../fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        let assets = document.assets.clone();
        assert_eq!(
            document.structure.to_projection(),
            document.view,
            "{source:?}"
        );
        let expected = document.view.to_typst_mapped();
        let before = identities(&document);
        document.sync_source();
        assert_eq!(
            (document.source().to_owned(), document.source_map.clone()),
            expected,
            "{source:?}"
        );
        assert_eq!(identities(&document), before, "{source:?}");
        assert_eq!(document.structure.to_projection(), document.view);
        assert_eq!(document.assets, assets);
        assert_eq!(
            Document::from_source(document.source()).unwrap().view,
            document.view
        );
    }
}

#[test]
fn roles_are_separate_projections_without_sentinels_separators_or_placeholder_text() {
    let document = Document::from_source(
        "/ Mot: Texte\n\n#quote(block: true)[Citation]\n\n#figure(image(\"images/parc.svg\"))",
    )
    .unwrap();
    let mut ids = BTreeSet::new();
    let mut roles = Vec::new();
    for block in &document.structure.blocks {
        for zone in block.content.zones() {
            assert!(ids.insert(zone.id));
            assert!(!zone.view.text.contains(['\t', IMAGE_OBJECT]));
            assert!(
                zone.view
                    .blocks
                    .iter()
                    .all(|kind| *kind == BlockKind::Paragraph)
            );
            roles.push((zone.role, zone.view.text.as_str()));
        }
    }
    assert!(roles.contains(&(ZoneRole::Term, "Mot")));
    assert!(roles.contains(&(ZoneRole::Definition, "Texte")));
    assert!(roles.contains(&(ZoneRole::Body, "Citation")));
    assert!(roles.contains(&(ZoneRole::Attribution, "")));
    assert!(roles.contains(&(ZoneRole::Caption, "")));
}

#[test]
fn unicode_typing_styles_and_newlines_keep_block_and_zone_identities() {
    let mut document = Document::from_source("Été é 👩‍🔬\nNoël\n\nSuite.").unwrap();
    let before = identities(&document);
    document.view.insert_text("œ", 2);
    document.view.toggle_mark(0..3, Mark::Bold);
    document.sync_source();
    assert_eq!(identities(&document), before);
    document.view.insert_text("\n", 4);
    document.sync_source();
    assert_eq!(identities(&document), before);
    document.view.delete_char_range(0..5);
    document.sync_source();
    assert_eq!(identities(&document), before);
}

#[test]
fn insertion_before_identical_paragraphs_preserves_original_objects() {
    let mut document = Document::from_source("Même\n\nMême").unwrap();
    let original = identities(&document);
    document.view.insert_text("Même\n\n", 0);
    document.sync_source();
    let now = identities(&document);
    assert_eq!(&now[2..], original);
    assert!(
        !original
            .iter()
            .any(|(id, _)| now[..2].iter().any(|(new, _)| new == id))
    );
}

#[test]
fn removing_the_first_quote_line_preserves_body_and_attribution_identities() {
    let mut document =
        Document::from_source("#quote(block: true, attribution: [Auteur])[Une\n\nAutre ligne]")
            .unwrap();
    let original = identities(&document);
    let first_newline = document.view.text.chars().position(|c| c == '\n').unwrap();
    document.view.delete_char_range(0..first_newline + 1);
    document.sync_source();
    assert_eq!(identities(&document), original);
    let BlockContent::Quote { body, attribution } = &document.structure.blocks[0].content else {
        panic!("quote")
    };
    assert_eq!(attribution.view.text, "Auteur");
    assert!(body.view.text.contains("Autre ligne"));
}

#[test]
fn replacing_an_image_keeps_identities_and_only_whole_block_deletion_removes_them() {
    let mut document =
        Document::from_source("#figure(image(\"images/parc.svg\"), caption: [Légende])").unwrap();
    let original = identities(&document);
    document.view.images.values_mut().next().unwrap().path = "images/autre.svg".into();
    document.view.insert_text(" !", document.view.len());
    document.sync_source();
    assert_eq!(identities(&document), original);
    document.view.delete_char_range(0..1);
    assert_eq!(
        document.view.validate_edit(),
        Err(crate::document::edits::EditError::ProtectedBoundary)
    );
    document.sync_source();
    assert_eq!(identities(&document), original);
    assert!(!document.view.images.is_empty());
    document.view.delete_char_range(0..document.view.len());
    document.sync_source();
    assert_ne!(identities(&document)[0].0, original[0].0);
    assert!(document.view.images.is_empty());
    assert_eq!(document.view.text, "");
}

#[test]
fn undo_redo_restores_identities_and_a_new_branch_never_reuses_deleted_ones() {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "Début".into();
    let document = Document::from_source(&stored.source).unwrap();
    let mut session = Session::new(stored, document, None, 1);
    let initial = identities(&session.document);
    let selection = Selection::default();
    let before = session.snapshot(selection);
    session
        .document
        .view
        .insert_text("\n\nSuite", session.document.view.len());
    session.remember(before, selection, false).unwrap();
    let extended = identities(&session.document);
    assert_eq!(extended[0], initial[0]);
    session.history(true, selection).unwrap();
    assert_eq!(identities(&session.document), initial);
    assert!(!session.is_dirty());
    session.history(false, selection).unwrap();
    assert_eq!(identities(&session.document), extended);
    session.history(true, selection).unwrap();
    let before = session.snapshot(selection);
    session
        .document
        .view
        .insert_text("\n\nSuite", session.document.view.len());
    session.remember(before, selection, false).unwrap();
    let branched = identities(&session.document);
    for (id, zones) in &branched[1..] {
        assert!(!extended.iter().any(|(old, _)| old == id));
        assert!(
            !extended
                .iter()
                .any(|(_, old)| zones.iter().any(|zone| old.contains(zone)))
        );
    }
    assert!(session.redo.is_empty());
}

#[test]
fn reopening_rebuilds_new_identities_without_changing_document_equality() {
    let mut first = Document::from_source("Un *texte*.").unwrap();
    let second = Document::from_source(first.source()).unwrap();
    assert_eq!(first, second);
    assert_ne!(identities(&first), identities(&second));
    let previous = identities(&first);
    first.sync_source();
    first.sync_source();
    assert_eq!(identities(&first), previous);
    assert!(!first.source().contains("id"));
}

#[test]
fn paragraph_splits_and_joins_follow_the_surviving_line_not_its_index() {
    let mut document = Document::from_source("Début fin").unwrap();
    let initial = identities(&document)[0].clone();
    document.view.insert_text("\n\n", 6);
    document.sync_source();
    let split = identities(&document);
    assert_eq!(split[0], initial);
    assert_ne!(split[2].0, initial.0);
    document.view.delete_char_range(6..8);
    document.sync_source();
    assert_eq!(identities(&document), [initial]);
    assert_eq!(document.view.text, "Début fin");

    let mut document = Document::from_source("= Titre\n\nSuite").unwrap();
    let last = identities(&document).last().unwrap().clone();
    let end = document.view.line_ranges().last().unwrap().start;
    document.view.delete_char_range(0..end);
    document.sync_source();
    assert_eq!(identities(&document), [last]);
    assert_eq!(document.view.blocks, [BlockKind::Paragraph]);
}

#[test]
fn editing_either_description_zone_preserves_both_identities_and_inline_styles() {
    let mut document = Document::from_source("/ *Été*: Une _définition_").unwrap();
    let original = identities(&document);
    document.view.delete_char_range(0..3);
    document.sync_source();
    assert_eq!(identities(&document), original);
    document.view.insert_text("Noël", 0);
    document.view.insert_text(" !", document.view.len());
    document.sync_source();
    assert_eq!(identities(&document), original);
    let BlockContent::Description { term, definition } = &document.structure.blocks[0].content
    else {
        panic!("description")
    };
    assert_eq!(term.view.text, "Noël");
    assert_eq!(definition.view.text, "Une définition !");
    assert!(definition.view.styles.iter().any(|style| style.italic));
}

#[test]
fn inserting_an_image_before_an_existing_block_does_not_reassign_its_identity() {
    let mut document = Document::from_source("Un paragraphe.").unwrap();
    let original = identities(&document)[0].clone();
    document.view.insert_image(
        0,
        ImageBlock {
            path: "images/parc.svg".into(),
            alt: String::new(),
        },
    );
    document.sync_source();
    assert_eq!(identities(&document)[1], original);
    assert_ne!(identities(&document)[0].0, original.0);
    let image = document.view.images.keys().next().copied().unwrap();
    let range = document.view.image_range(image).unwrap();
    document.view.delete_char_range(range.start..range.end + 1);
    document.sync_source();
    assert_eq!(identities(&document), [original]);
}

#[test]
fn splitting_a_quote_keeps_attribution_identity_with_its_text_not_with_the_first_block() {
    let mut document =
        Document::from_source("#quote(block: true, attribution: [Autrice])[Une\n\nAutre]").unwrap();
    let BlockContent::Quote { attribution, .. } = &document.structure.blocks[0].content else {
        panic!("quote")
    };
    let original_attribution = attribution.id;
    let original_block = document.structure.blocks[0].id;
    document.view.blocks[1] = BlockKind::Paragraph;
    document.sync_source();
    let quotes: Vec<_> = document
        .structure
        .blocks
        .iter()
        .filter_map(|block| {
            if let BlockContent::Quote { attribution, .. } = &block.content {
                Some((block.id, attribution))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(quotes.len(), 2);
    assert_eq!(quotes[0].0, original_block);
    assert_ne!(quotes[0].1.id, original_attribution);
    assert_eq!(quotes[0].1.view.text, "");
    assert_ne!(quotes[1].0, original_block);
    assert_eq!(quotes[1].1.id, original_attribution);
    assert_eq!(quotes[1].1.view.text, "Autrice");
}

#[test]
fn a_no_op_native_replacement_keeps_committed_identities_and_revision() {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "Un texte".into();
    let document = Document::from_source(&stored.source).unwrap();
    let mut session = Session::new(stored, document, None, 1);
    let original = identities(&session.document);
    let selection = Selection::default();
    let before = session.snapshot(selection);
    session.document.view.replace_with("Un texte");
    session.remember(before, selection, false).unwrap();
    assert_eq!(session.revision, 1);
    assert!(session.undo.is_empty());
    assert_eq!(identities(&session.document), original);
    let before = session.snapshot(selection);
    session
        .document
        .view
        .insert_text(" !", session.document.view.len());
    session.remember(before, selection, false).unwrap();
    assert_eq!(identities(&session.document), original);
}

#[test]
fn inserting_images_at_compound_boundaries_preserves_all_existing_zone_identities() {
    for source in [
        "#quote(block: true, attribution: [Autrice])[Texte]",
        "#quote(block: true)[]",
        "/ Mot: Définition",
        "= Titre",
        "",
    ] {
        let mut document = Document::from_source(source).unwrap();
        let before = identities(&document);
        document.view.insert_image(
            0,
            ImageBlock {
                path: "images/parc.svg".into(),
                alt: String::new(),
            },
        );
        document.sync_source();
        assert_eq!(&identities(&document)[1..], before);
        assert_eq!(
            Document::from_source(document.source()).unwrap().view,
            document.view
        );
    }
    let mut document = Document::from_source("#quote(block: true)[Texte]").unwrap();
    let before = identities(&document);
    document.view.insert_image(
        document.view.len(),
        ImageBlock {
            path: "images/parc.svg".into(),
            alt: String::new(),
        },
    );
    document.sync_source();
    assert_eq!(identities(&document)[0], before[0]);
    let BlockContent::Quote { body, attribution } = &document.structure.blocks[0].content else {
        panic!("quote")
    };
    assert_eq!(body.view.text, "Texte");
    assert_eq!(attribution.view.text, "");
}
