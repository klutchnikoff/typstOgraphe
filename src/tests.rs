use crate::document::{BlockKind, Document, InlineStyle, Mark, Metadata, Projection};
use eframe::egui::TextBuffer;

#[test]
fn demo_projects_native_typst_without_exposing_source() {
    let document = Document::demo();
    assert!(document.view.text.contains("jeudi 12 novembre 2026"));
    assert!(!document.view.text.contains("#link"));
    assert!(!document.view.text.contains('*'));
    assert_eq!(
        document
            .view
            .blocks
            .iter()
            .filter(|kind| **kind == BlockKind::Bullet)
            .count(),
        3
    );
    assert!(document.view.styles.iter().any(|s| s.bold));
    assert!(document.view.styles.iter().any(|s| s.link.is_some()));
}

#[test]
fn round_trip_preserves_unicode_and_overlapping_inline_styles() {
    let mut document =
        Document::from_source("Élodie écrit à Noël.\n\nDeuxième paragraphe.").unwrap();
    document.view.toggle_mark(0..12, Mark::Bold);
    document.view.toggle_mark(6..23, Mark::Italic);
    document.sync_source();
    let restored = Document::from_source(document.source()).unwrap();
    assert_eq!(restored.view.text, document.view.text);
    assert_eq!(restored.view.styles, document.view.styles);
    assert_eq!(restored.view.blocks, document.view.blocks);
}

#[test]
fn editing_uses_characters_and_preserves_surrounding_marks() {
    let mut document = Document::from_source("*Élodie* écrit.").unwrap();
    document.view.typing_style = document.view.style_at(2);
    document.view.delete_char_range(2..4);
    document.view.insert_text("👩🏽‍💻é", 2);
    assert_eq!(document.view.text, "Él👩🏽‍💻éie écrit.");
    assert_eq!(
        document.view.text.chars().count(),
        document.view.styles.len()
    );
    assert!(document.view.styles[2..7].iter().all(|s| s.bold));
    document.sync_source();
    let restored = Document::from_source(document.source()).unwrap();
    assert_eq!(restored.view.text, document.view.text);
    assert_eq!(restored.view.styles, document.view.styles);
}

#[test]
fn selecting_to_the_start_of_next_paragraph_excludes_it() {
    let mut document = Document::from_source("Un.\n\nDeux.\n\nTrois.").unwrap();
    document.view.set_block(0..4, BlockKind::Heading(1));
    assert_eq!(
        document.view.blocks,
        [
            BlockKind::Heading(1),
            BlockKind::Paragraph,
            BlockKind::Paragraph
        ]
    );
    document
        .view
        .set_block(0..document.view.len(), BlockKind::Numbered);
    document.sync_source();
    let restored = Document::from_source(document.source()).unwrap();
    assert_eq!(restored.view.blocks, document.view.blocks);
}

#[test]
fn inserting_and_deleting_paragraphs_keeps_structure_aligned() {
    let mut document = Document::from_source("= Titre\n\nTexte.").unwrap();
    document.view.insert_text("\nSuite", 5);
    assert_eq!(
        document.view.blocks,
        [
            BlockKind::Heading(1),
            BlockKind::Paragraph,
            BlockKind::Paragraph
        ]
    );
    document.view.delete_char_range(2..12);
    assert_eq!(
        document.view.blocks.len(),
        document.view.text.matches('\n').count() + 1
    );
    assert_eq!(
        document.view.styles.len(),
        document.view.text.chars().count()
    );
}

#[test]
fn literal_typst_characters_are_not_executed() {
    let literal = "#panic(\"NON\") [x] * _ $ @id <id> \\ = - + / ~ { } \" '";
    let mut view = Projection::default();
    view.insert_text(literal, 0);
    let (source, _) = view.to_typst_mapped();
    let restored = Document::from_source(&source).unwrap();
    assert_eq!(restored.view.text, literal);
}

#[test]
fn unknown_functions_and_calculated_arguments_are_rejected() {
    assert!(Document::from_source("#let x = 1").is_err());
    assert!(Document::from_source("#link(\"https://example.org\" + \"/x\")[Texte]").is_err());
    assert!(Document::from_source("#warning[Texte]").is_err());
    assert!(Document::from_source("- Élément\n  - Sous-élément").is_err());
}

#[test]
fn repeated_edits_keep_projection_invariants() {
    let mut view = Projection::default();
    for i in 0..200 {
        let index = if view.is_empty() { 0 } else { i % view.len() };
        view.typing_style = InlineStyle {
            bold: i % 2 == 0,
            ..Default::default()
        };
        view.insert_text(if i % 4 == 0 { "é\n" } else { "漢🙂" }, index);
        if view.len() > 9 {
            view.delete_char_range(index..index + 1);
        }
        assert_eq!(view.len(), view.text.chars().count());
        assert_eq!(view.blocks.len(), view.text.matches('\n').count() + 1);
    }
}

#[test]
fn real_typst_composes_both_models_and_metadata_escapes() {
    let mut world = crate::rendering::TypstWorld::default();
    let mut document = Document::demo();
    document.view.toggle_mark(0..50, Mark::Italic);
    document.sync_source();
    let metadata = Metadata {
        objet: "Une demande \"citée\" avec \\ et #panic(\"NON\")".into(),
        ..Default::default()
    };
    for compact in [false, true] {
        let result = world
            .compile(document.source(), &metadata, compact)
            .unwrap();
        assert!(!result.pages().is_empty());
        let pixmap =
            typst_render::render(&result.pages()[0], &typst_render::RenderOptions::default());
        assert!(pixmap.width() > 1000);
        assert!(
            pixmap
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|rgba| rgba[0] < 150 && rgba[1] < 150 && rgba[2] < 150)
        );
        let pdf = typst_pdf::pdf(&result, &typst_pdf::PdfOptions::default()).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }
}

#[test]
fn compiler_reports_error_and_recovers_on_next_request() {
    let mut world = crate::rendering::TypstWorld::default();
    assert!(
        world
            .compile("#panic(\"essai\")", &Metadata::default(), false)
            .unwrap_err()
            .contains("essai")
    );
    assert!(
        world
            .compile("Bonjour.", &Metadata::default(), false)
            .is_ok()
    );
}

#[test]
fn source_positions_survive_unicode_escapes_styles_and_empty_paragraphs() {
    let mut document = Document::from_source("= Échéance\n\nUn *été* avec _Noël_ et \\#.").unwrap();
    for canonical in [false, true] {
        if canonical {
            document
                .view
                .insert_text("\n\nEncore é.", document.view.len());
            document.sync_source();
        }
        assert_eq!(document.source_map.characters.len(), document.view.len());
        assert_eq!(document.source_map.source_len, document.source().len());
        for (index, character) in document.view.text.chars().enumerate() {
            if character.is_whitespace() {
                continue;
            }
            let byte = document.source_map.to_source(index);
            assert_eq!(
                document.source_map.to_visible(byte),
                index,
                "{character} at {index}"
            );
            let range = document.source_map.characters[index].clone();
            let fragment = &document.source()[range];
            assert!(
                fragment.contains(character),
                "{character:?} not in {fragment:?}"
            );
        }
    }
}

fn compiled_sync(document: &Document, compact: bool) -> crate::navigation::RenderSync {
    let mut world = crate::rendering::TypstWorld::default();
    let rendered = world
        .compile(document.source(), &Metadata::default(), compact)
        .unwrap();
    crate::navigation::RenderSync::new(1, world, rendered, document.source_map.clone())
}

#[test]
fn bidirectional_navigation_handles_unicode_styles_links_and_repeated_passages() {
    let mut document = Document::from_source("= Échéance\n\nNoël écrit un *courrier* avec _Élodie_ et \\# ici.\n\n#link(\"https://example.org\")[Consulter le dossier]\n\n- Élément A\n- Élément B\n\nPassage identique.\n\nPassage identique.").unwrap();
    for canonical in [false, true] {
        if canonical {
            document.sync_source();
        }
        for compact in [false, true] {
            let sync = compiled_sync(&document, compact);
            for word in [
                "Échéance",
                "Noël",
                "courrier",
                "Élodie",
                "#",
                "Consulter",
                "dossier",
                "Élément B",
            ] {
                let byte = document.view.text.find(word).unwrap();
                let index = document.view.text[..byte].chars().count();
                let point = sync.forward(index).expect(word);
                assert_eq!(
                    sync.backward(point),
                    Some(index),
                    "{word}, canonical {canonical}, compact {compact}"
                );
            }
            let first = document.view.text.find("Passage identique").unwrap();
            let last = document.view.text.rfind("Passage identique").unwrap();
            let first = document.view.text[..first].chars().count();
            let last = document.view.text[..last].chars().count();
            let first_point = sync.forward(first).unwrap();
            let last_point = sync.forward(last).unwrap();
            assert!(last_point.point[1] > first_point.point[1]);
            assert_eq!(sync.backward(last_point), Some(last));
            assert!(
                sync.backward(crate::navigation::PagePoint {
                    page: 0,
                    point: [0.0, 0.0]
                })
                .is_none()
            );
        }
    }
}

#[test]
fn navigation_tracks_a_long_paragraph_across_pages_and_retains_its_snapshot() {
    let mut document =
        Document::from_source(&"Cette phrase remplit plusieurs pages. ".repeat(700)).unwrap();
    let sync = compiled_sync(&document, false);
    let index = document.view.len() - 20;
    let location = sync.forward(index).unwrap();
    assert!(location.page >= 2);
    let backward = sync.backward(location).unwrap();
    assert!(backward.abs_diff(index) <= 1); // Une ligature peut couvrir deux caractères.
    document.view.insert_text("Un nouveau début. ", 0);
    document.sync_source();
    // L'instantané reste cohérent même après la modification du document actif.
    assert_eq!(sync.backward(location), Some(backward));
}

fn highlighted(
    highlights: &[crate::navigation::SelectionHighlight],
    point: crate::navigation::PagePoint,
) -> bool {
    highlights.iter().any(|highlight| {
        if highlight.page != point.page {
            return false;
        }
        let corners = &highlight.corners;
        let cross: Vec<_> = (0..4)
            .map(|i| {
                let a = corners[i];
                let b = corners[(i + 1) % 4];
                (b[0] - a[0]) * (point.point[1] - a[1]) - (b[1] - a[1]) * (point.point[0] - a[0])
            })
            .collect();
        cross.iter().all(|&value| value >= -0.01) || cross.iter().all(|&value| value <= 0.01)
    })
}

#[test]
fn selection_highlights_follow_visible_unicode_text_across_styles_and_links() {
    let mut document = Document::from_source("= Échéance\n\nBonjour *Noël*, _Élodie_ écrit \\# ici.\n\n#link(\"https://example.org\")[Consulter le dossier]\n\nTexte identique.\n\nTexte identique.").unwrap();
    for canonical in [false, true] {
        if canonical {
            document.sync_source();
        }
        for compact in [false, true] {
            let sync = compiled_sync(&document, compact);
            let start = document.view.text.find("jour").unwrap();
            let start = document.view.text[..start].chars().count();
            let end = document.view.text.find("dossier").unwrap() + "dossier".len();
            let end = document.view.text[..end].chars().count();
            let highlights = sync.selection_highlights(start..end);
            assert!(!highlights.is_empty());
            for index in start..end {
                if document
                    .view
                    .text
                    .chars()
                    .nth(index)
                    .unwrap()
                    .is_whitespace()
                {
                    continue;
                }
                assert!(
                    highlighted(&highlights, sync.forward(index).unwrap()),
                    "character {index}, canonical {canonical}, compact {compact}"
                );
            }
            for word in ["Échéance", "Bon", "Texte identique"] {
                let byte = document.view.text.find(word).unwrap();
                let index = document.view.text[..byte].chars().count();
                assert!(
                    !highlighted(&highlights, sync.forward(index).unwrap()),
                    "{word}"
                );
            }
            assert!(highlights.len() < end - start); // Les glyphes voisins sont réunis en bandes.
            assert!(sync.selection_highlights(start..start).is_empty());
            assert!(
                sync.selection_highlights(document.view.len()..usize::MAX)
                    .is_empty()
            );
            let last = document.view.text.rfind("Texte identique").unwrap();
            let last = document.view.text[..last].chars().count();
            let repeated = sync.selection_highlights(last..document.view.len());
            assert!(highlighted(&repeated, sync.forward(last).unwrap()));
            let first = document.view.text.find("Texte identique").unwrap();
            let first = document.view.text[..first].chars().count();
            assert!(!highlighted(&repeated, sync.forward(first).unwrap()));
        }
    }
}

#[test]
fn selection_highlights_span_pages_and_include_partial_ligatures() {
    let document =
        Document::from_source(&"Une lettre à Élodie avec une formule officielle. ".repeat(250))
            .unwrap();
    let sync = compiled_sync(&document, false);
    let highlights = sync.selection_highlights(10..document.view.len() - 10);
    assert!(highlights.iter().any(|highlight| highlight.page >= 1));
    assert!(highlighted(&highlights, sync.forward(15).unwrap()));
    assert!(highlighted(
        &highlights,
        sync.forward(document.view.len() - 15).unwrap()
    ));
    assert!(!highlighted(&highlights, sync.forward(0).unwrap()));
    assert!(!highlighted(
        &highlights,
        sync.forward(document.view.len() - 1).unwrap()
    ));

    let document = Document::from_source("Office efficace.").unwrap();
    let sync = compiled_sync(&document, false);
    // « ffi » peut former un seul glyphe : un seul caractère sélectionné
    // suffit à surligner le glyphe, sans englober le reste du mot.
    let partial = sync.selection_highlights(2..3);
    assert!(highlighted(&partial, sync.forward(2).unwrap()));
    assert!(!highlighted(&partial, sync.forward(0).unwrap()));
    assert!(!highlighted(&partial, sync.forward(5).unwrap()));
}
