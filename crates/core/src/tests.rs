use crate::document::{BlockKind, Document, InlineStyle, Mark, Projection};
use crate::document_types::{DocumentType, Metadata, courrier, models::Model};

#[test]
fn japanese_text_uses_real_glyphs_in_typst_preview() {
    use typst::layout::{Frame, FrameItem};
    fn check(frame: &Frame, text: &mut String) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => check(&group.frame, text),
                FrameItem::Text(run) => {
                    text.push_str(&run.text);
                    assert!(
                        run.glyphs.iter().all(|glyph| glyph.id != 0),
                        "glyphe absent dans {:?}, police {}",
                        run.text,
                        run.font.info().family
                    );
                }
                _ => {}
            }
        }
    }
    let mut world = crate::rendering::TypstWorld::default();
    let rendered = world
        .compile_with_template(
            "にほん 日本 中国 繁體中文\n\n#strong[日本 中国]\n\n#smallcaps[Bonjour]\n\n$integral_0^1 x^2 dif x = 1/3$",
            &Metadata::demo(DocumentType::Courrier),
            "#let courrier(..args, body) = body",
        )
        .unwrap();
    let mut text = String::new();
    for page in rendered.pages() {
        check(&page.frame, &mut text);
    }
    assert!(text.contains("にほん") && text.contains("日本") && text.contains("繁體中文"));
}

#[test]
fn soft_newlines_and_paragraph_breaks_round_trip_with_unicode_styles_and_blocks() {
    for source in [
        "Été\nNoël\n\nSuite.",
        "Été\n",
        "Été\n\n",
        "Un *été\nà Noël*\n\nSuite.",
        "= Titre\nSuite\n\n- Un\n- Deux\n\nFin.",
        "#quote(block: true)[Un\nété\n\nUne seconde idée.]\n\nSuite.",
    ] {
        let mut document = Document::from_source(source).unwrap();
        let original = document.view.clone();
        document.sync_source();
        let restored = Document::from_source(document.source()).unwrap();
        assert_eq!(restored.view, original, "{source:?}");
        assert!(!document.source().contains('¶'));
        for (index, character) in document.view.text.chars().enumerate() {
            if !character.is_whitespace() {
                assert_eq!(
                    document
                        .source_map
                        .to_visible(document.source_map.to_source(index)),
                    index
                );
            }
        }
    }
    let mut document = Document::from_source("Été\n\n\n\nNoël").unwrap();
    assert_eq!(document.view.text, "Été\n\nNoël");
    document.sync_source();
    assert_eq!(document.source(), "Été\n\nNoël");
    let mut projection = Projection::default();
    projection.insert_text("Un\r\n\r\n\r\nDeux", 0);
    assert_eq!(projection.text, "Un\n\nDeux");
    assert_eq!(projection.insert_text("\n", 3), 0);
    assert_eq!(projection.text, "Un\n\nDeux");
}

#[test]
fn real_typst_keeps_a_soft_newline_on_the_same_line_and_starts_a_new_paragraph_after_two() {
    use typst::layout::{Frame, FrameItem};
    fn runs(frame: &Frame, y: f64, result: &mut Vec<(String, f64)>) {
        for (position, item) in frame.items() {
            let y = y + position.y.to_pt();
            match item {
                FrameItem::Group(group) => runs(&group.frame, y, result),
                FrameItem::Text(text) => result.push((text.text.to_string(), y)),
                _ => {}
            }
        }
    }
    let mut document = Document::from_source("Alpha\nBeta\n\nGamma").unwrap();
    document.sync_source();
    assert_eq!(document.source(), "Alpha\nBeta\n\nGamma");
    let mut world = crate::rendering::TypstWorld::default();
    let template = "#let courrier(..args, body) = { set page(width: 200mm, height: 200mm); set par(spacing: 20pt); body }";
    let rendered = world
        .compile_with_template(
            document.source(),
            &Metadata::demo(DocumentType::Courrier),
            template,
        )
        .unwrap();
    let mut text = Vec::new();
    runs(&rendered.pages()[0].frame, 0.0, &mut text);
    let at = |word: &str| {
        text.iter()
            .find(|(value, _)| value.contains(word))
            .unwrap()
            .1
    };
    assert!((at("Alpha") - at("Beta")).abs() < 0.01, "{text:?}");
    assert!(at("Gamma") - at("Beta") > 20.0, "{text:?}");
}

#[test]
fn paragraph_commands_include_soft_lines_but_stop_at_separators_and_structures() {
    let mut document = Document::from_source("Été\nNoël\n\nAprès.").unwrap();
    assert_eq!(document.view.paragraph_at(5), 0..8);
    assert!(document.view.is_single_paragraph(0..8));
    assert!(!document.view.is_single_paragraph(0..10));
    document.view.set_block(5..5, BlockKind::Heading(2));
    document.sync_source();
    assert_eq!(document.view.text, "Été Noël\n\nAprès.");
    assert_eq!(
        document.view.blocks,
        [
            BlockKind::Heading(2),
            BlockKind::Paragraph,
            BlockKind::Paragraph
        ]
    );
    assert_eq!(
        Document::from_source(document.source()).unwrap().view,
        document.view
    );
    let list = Document::from_source("- Un\n- Deux").unwrap();
    assert!(!list.view.is_single_paragraph(0..7));
}

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
fn block_quotes_round_trip_multiple_paragraphs_styled_attributions_and_exact_positions() {
    let source = "Avant.\n\n#quote(block: true, attribution: [Élodie, _Rapport_ et #link(\"https://example.org\")[source]])[Un *été* à Noël.\n\nDeuxième paragraphe.]\n\nAprès.\n\n#quote(block: true)[Sans attribution.]\n\n#quote(block: true)[Autre citation.]";
    let mut document = Document::from_source(source).unwrap();
    assert_eq!(document.view.quote_groups().len(), 3);
    assert!(document.view.text.contains("Deuxième paragraphe.\nÉlodie"));
    assert!(!document.view.text.contains("#quote"));
    document.sync_source();
    let restored = Document::from_source(document.source()).unwrap();
    assert_eq!(restored.view, document.view);
    for (index, c) in document.view.text.chars().enumerate() {
        if c.is_whitespace() {
            continue;
        }
        let byte = document.source_map.to_source(index);
        assert_eq!(
            document.source_map.to_visible(byte),
            index,
            "{c} at {index}"
        );
        assert!(document.source()[document.source_map.characters[index].clone()].contains(c));
    }
    for invalid in [
        "#quote[Texte]",
        "#quote(block: false)[Texte]",
        "#quote(block: true, quotes: true)[Texte]",
        "#quote(block: true)[#quote(block: true)[Imbriquée]]",
        "#quote(block: true, attribution: [Un\n\nDeux])[Texte]",
    ] {
        assert!(Document::from_source(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn quote_models_control_rendering_and_navigation_also_covers_attribution() {
    use crate::navigation::RenderSync;
    use typst::layout::{Frame, FrameItem};
    fn collect(frame: &Frame, out: &mut String) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(g) => collect(&g.frame, out),
                FrameItem::Text(t) => out.push_str(&t.text),
                _ => {}
            }
        }
    }
    let mut document = Document::from_source("#quote(block: true, attribution: [Élodie et #link(\"https://example.org\")[son rapport]])[Un *été* à Noël.\n\nUne seconde pensée.]\n\nSuite.").unwrap();
    document.sync_source();
    let mut world = crate::rendering::TypstWorld::default();
    let metadata = Metadata::demo(DocumentType::Courrier);
    for model in Model::all() {
        let template = model.source();
        let output = world
            .compile_with_template(document.source(), &metadata, &template)
            .unwrap();
        let sync = RenderSync::new(1, world.clone(), output, document.source_map.clone());
        for needle in ["été", "seconde", "Élodie", "rapport", "Suite"] {
            let start = document.view.text[..document.view.text.find(needle).unwrap()]
                .chars()
                .count();
            let point = sync.forward(start).unwrap();
            assert!(
                sync.backward(point).unwrap().abs_diff(start) <= 1,
                "{needle}"
            );
            assert!(
                !sync
                    .selection_highlights(start..start + needle.chars().count())
                    .is_empty()
            );
        }
        let custom = template.replace("#emph(it.body)", "MODÈLE PERSONNALISÉ #it.body");
        let output = world
            .compile_with_template(document.source(), &metadata, &custom)
            .unwrap();
        let mut text = String::new();
        for page in output.pages() {
            collect(&page.frame, &mut text);
        }
        assert!(text.contains("MODÈLE PERSONNALISÉ"));
        assert!(text.contains("Élodie"));
    }
}

#[test]
fn description_lists_preserve_terms_styles_unicode_and_source_positions() {
    let mut document = Document::from_source(
        "/ Échéance: Le *12 novembre*.\n/ Contact\\: service: #link(\"https://example.org\")[Élodie].",
    ).unwrap();
    assert_eq!(
        document.view.text,
        "Échéance\tLe 12 novembre.\nContact: service\tÉlodie."
    );
    assert_eq!(document.view.blocks, [BlockKind::Description; 2]);
    document
        .view
        .toggle_mark(0..document.view.len(), Mark::Italic);
    document.sync_source();
    let restored = Document::from_source(document.source()).unwrap();
    assert_eq!(restored.view.text, document.view.text);
    assert_eq!(restored.view.styles, document.view.styles);
    assert_eq!(restored.view.blocks, document.view.blocks);
    for (index, c) in document.view.text.chars().enumerate() {
        if !matches!(c, '\n' | '\t') {
            assert_eq!(
                document
                    .source_map
                    .to_visible(document.source_map.to_source(index)),
                index
            );
        }
    }
    for source in ["/ : ", "/ Terme: ", "/ : Description"] {
        let mut document = Document::from_source(source).unwrap();
        document.sync_source();
        assert!(Document::from_source(document.source()).is_ok(), "{source}");
    }
}

#[test]
fn model_controls_list_markers_and_language_defaults_in_actual_typst_output() {
    use crate::document::DocumentLanguage;
    use crate::navigation::RenderSync;
    use typst::layout::{Frame, FrameItem};
    fn text(frame: &Frame, out: &mut String) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => text(&group.frame, out),
                FrameItem::Text(run) => out.push_str(&run.text),
                _ => {}
            }
        }
    }
    let mut world = crate::rendering::TypstWorld::default();
    let mut metadata = Metadata::demo(DocumentType::Courrier);
    let mut document =
        Document::from_source("- Élément unique\n\n/ Échéance: Le *12 novembre*.").unwrap();
    document.sync_source();
    for model in Model::all() {
        let template = model.source();
        assert_eq!(
            world.model_presentation(&template).unwrap(),
            Default::default()
        );
        for (language, marker) in [
            (DocumentLanguage::French, '—'),
            (DocumentLanguage::English, '•'),
        ] {
            metadata.options_mut().language = language;
            let output = world
                .compile_with_template(document.source(), &metadata, &template)
                .unwrap();
            let mut content = String::new();
            for page in output.pages() {
                text(&page.frame, &mut content);
            }
            assert!(content.contains(marker), "{content}");
            assert!(content.contains("Échéance"));
            let sync = RenderSync::new(1, world.clone(), output, document.source_map.clone());
            for needle in ["Échéance", "novembre"] {
                let index = document.view.text[..document.view.text.find(needle).unwrap()]
                    .chars()
                    .count();
                assert!(
                    !sync
                        .selection_highlights(index..index + needle.chars().count())
                        .is_empty()
                );
                let position = sync.forward(index).unwrap();
                let actual = sync.backward(position).unwrap();
                assert!(actual.abs_diff(index) <= 1, "{actual} vs {index}");
            }
        }
        metadata.options_mut().bullet_marker = Some("▪".into());
        assert!(
            world
                .compile_with_template(document.source(), &metadata, &template)
                .is_err()
        );
        let configurable = format!(
            "{template}\n#let editor-options = (bullet-markers: (\"▪\", \"◆\"), custom-bullet-marker: false)"
        );
        let policy = world.model_presentation(&configurable).unwrap();
        assert!(policy.allows_bullet_marker("▪"));
        assert!(!policy.allows_bullet_marker("★"));
        let output = world
            .compile_with_template(document.source(), &metadata, &configurable)
            .unwrap();
        let mut content = String::new();
        for page in output.pages() {
            text(&page.frame, &mut content);
        }
        assert!(content.contains('▪'));
        metadata.options_mut().bullet_marker = Some("★".into());
        assert!(
            world
                .compile_with_template(document.source(), &metadata, &configurable)
                .is_err()
        );
        let custom = format!("{template}\n#let editor-options = (custom-bullet-marker: true,)");
        assert!(
            world
                .compile_with_template(document.source(), &metadata, &custom)
                .is_ok()
        );
        for invalid in ["", "ab", "\n", " "] {
            metadata.options_mut().bullet_marker = Some(invalid.into());
            assert!(
                world
                    .compile_with_template(document.source(), &metadata, &custom)
                    .is_err()
            );
        }
        metadata.options_mut().bullet_marker = None;
        let malformed =
            format!("{template}\n#let editor-options = (bullet-markers: (\"wrong\",),)");
        assert!(world.model_presentation(&malformed).is_err());
        // Le créateur du modèle peut remplacer les conventions automatiques.
        let overridden = template.replace(
            r#"if options.language == "fr" { ([—], [•], [◦]) } else { ([•], [◦], [▪]) }"#,
            "[◆]",
        );
        let output = world
            .compile_with_template(document.source(), &metadata, &overridden)
            .unwrap();
        let mut content = String::new();
        for page in output.pages() {
            text(&page.frame, &mut content);
        }
        assert!(content.contains('◆'));
    }
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
            BlockKind::Paragraph,
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
    let restored =
        Document::from_source(&source).unwrap_or_else(|error| panic!("{error}: {source:?}"));
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
fn real_typst_composes_registered_models_and_metadata_escapes() {
    let mut world = crate::rendering::TypstWorld::default();
    let mut document = Document::demo();
    document.view.toggle_mark(0..50, Mark::Italic);
    document.sync_source();
    let metadata = Metadata::Courrier(courrier::Metadata {
        objet: "Une demande \"citée\" avec \\ et #panic(\"NON\")".into(),
        ..Default::default()
    });
    for model in Model::all() {
        let result = world
            .compile_courrier(document.source(), &metadata, model)
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
fn document_options_control_real_page_dimensions_and_text_language() {
    use crate::document::{DocumentLanguage, PaperFormat};
    use typst::layout::{Frame, FrameItem};
    fn collect(frame: &Frame, language: &str, text: &mut String) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => collect(&group.frame, language, text),
                FrameItem::Text(run) => {
                    assert_eq!(run.lang.as_str(), language);
                    text.push_str(&run.text);
                }
                _ => {}
            }
        }
    }
    let mut world = crate::rendering::TypstWorld::default();
    let mut metadata = Metadata::demo(DocumentType::Courrier);
    for (paper, width, height) in [
        (PaperFormat::A3, 297.0, 420.0),
        (PaperFormat::A4, 210.0, 297.0),
        (PaperFormat::A5, 148.0, 210.0),
        (PaperFormat::Letter, 215.9, 279.4),
        (PaperFormat::Legal, 215.9, 355.6),
    ] {
        for landscape in [false, true] {
            metadata.options_mut().paper = paper;
            metadata.options_mut().landscape = landscape;
            {
                let document = world.compile_with_template("A short letter.", &metadata,
                    "#let courrier(..args, options: (:), body) = { set page(paper: options.paper, flipped: options.landscape); set text(lang: options.language); body }").unwrap();
                for page in document.pages() {
                    let size = page.frame.size();
                    let (expected_width, expected_height) = if landscape {
                        (height, width)
                    } else {
                        (width, height)
                    };
                    assert!((size.x.to_mm() - expected_width).abs() < 0.05);
                    assert!((size.y.to_mm() - expected_height).abs() < 0.05);
                }
            }
        }
    }
    metadata.options_mut().paper = PaperFormat::A4;
    metadata.options_mut().landscape = false;
    for language in DocumentLanguage::ALL {
        metadata.options_mut().language = language;
        let document = world
            .compile_courrier("A short letter.", &metadata, Model::Personal)
            .unwrap();
        let mut text = String::new();
        for page in document.pages() {
            collect(&page.frame, language.code(), &mut text);
        }
        assert!(text.contains("A short letter."));
        if language == DocumentLanguage::English {
            assert!(text.contains("Subject"));
        }
        if language == DocumentLanguage::French {
            assert!(text.contains("Objet"));
        }
    }
}

#[test]
fn compiler_reports_error_and_recovers_on_next_request() {
    let mut world = crate::rendering::TypstWorld::default();
    assert!(
        world
            .compile_courrier(
                "#panic(\"essai\")",
                &Metadata::demo(DocumentType::Courrier),
                Model::Personal
            )
            .unwrap_err()
            .to_string()
            .contains("essai")
    );
    assert!(
        world
            .compile_courrier(
                "Bonjour.",
                &Metadata::demo(DocumentType::Courrier),
                Model::Personal
            )
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

fn compiled_sync(document: &Document, wide_margin: bool) -> crate::navigation::RenderSync {
    let mut world = crate::rendering::TypstWorld::default();
    let template = if wide_margin {
        Model::Personal.source().replace("left: 25mm", "left: 32mm")
    } else {
        Model::Personal.source()
    };
    let rendered = world
        .compile_with_template(
            document.source(),
            &Metadata::demo(DocumentType::Courrier),
            &template,
        )
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

#[test]
fn image_figures_round_trip_optional_styled_captions_alt_and_source_positions() {
    let source = "Avant.\n\n#figure(image(\"images/parc.svg\", alt: \"Parc à Noël — \\\"ouvert\\\"\"), caption: [Un *parc* et #link(\"https://example.org\")[sa source]])\n\n#figure(image(\"images/autre.png\"))\n\nAprès.";
    let mut document = Document::from_source(source).unwrap();
    assert_eq!(document.view.images.len(), 2);
    assert_eq!(document.view.images[&1].alt, "Parc à Noël — \"ouvert\"");
    assert!(document.view.text.contains("Un parc et sa source"));
    assert!(!document.view.text.contains("#figure"));
    document.sync_source();
    assert_eq!(
        Document::from_source(document.source()).unwrap().view,
        document.view
    );
    for (index, c) in document.view.text.chars().enumerate() {
        assert_eq!(
            document
                .source_map
                .to_visible(document.source_map.to_source(index)),
            index,
            "{index}: {c}"
        );
        if !c.is_whitespace() && c != crate::document::IMAGE_OBJECT {
            assert!(document.source()[document.source_map.characters[index].clone()].contains(c));
        }
    }
    for invalid in [
        "#figure(image(\"../secret.png\"))",
        "#figure(image(\"/tmp/secret.png\"))",
        "#figure(image(\"images/parc.svg\", width: 20%))",
        "#figure(rect())",
        "#figure(image(\"images/parc.svg\"), placement: top)",
        "#figure(image(\"images/parc.svg\"), caption: [Un\n\nDeux])",
        "#quote(block: true)[#figure(image(\"images/parc.svg\"))]",
    ] {
        assert!(Document::from_source(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn image_models_control_size_caption_and_navigation_and_embed_alt_in_pdf() {
    use crate::navigation::RenderSync;
    use typst::layout::{Frame, FrameItem};
    fn images(frame: &Frame, out: &mut Vec<(f64, f64, Option<String>)>) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Image(image, size, _) => out.push((
                    size.x.to_pt(),
                    size.y.to_pt(),
                    image.alt().map(str::to_owned),
                )),
                FrameItem::Group(group) => images(&group.frame, out),
                _ => {}
            }
        }
    }
    let mut document = Document::from_source("Avant.\n\n#figure(image(\"images/parc.svg\", alt: \"Un chemin et trois arbres\"), caption: [Le *parc* municipal.])\n\nAprès.").unwrap();
    document.sync_source();
    document.assets.insert(
        "images/parc.svg".into(),
        include_bytes!("../../../fixtures/parc.svg")
            .as_slice()
            .into(),
    );
    let mut world = crate::rendering::TypstWorld::default();
    world.set_assets(&document.assets);
    let metadata = Metadata::demo(DocumentType::Courrier);
    for model in Model::all() {
        let template = model.source();
        let page = world
            .compile_with_template(document.source(), &metadata, &template)
            .unwrap();
        let mut found = Vec::new();
        for p in page.pages() {
            images(&p.frame, &mut found);
        }
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].2.as_deref(), Some("Un chemin et trois arbres"));
        assert!((found[0].0 / found[0].1 - 2.4).abs() < 0.01);
        let sync = RenderSync::new(1, world.clone(), page.clone(), document.source_map.clone());
        let at = document.view.image_range(1).unwrap().start;
        let location = sync.forward(at).unwrap();
        assert_eq!(sync.backward(location), Some(at));
        assert_eq!(sync.selection_highlights(at..at + 1).len(), 1);
        assert!(
            !typst_pdf::pdf(&page, &typst_pdf::PdfOptions::default())
                .unwrap()
                .is_empty()
        );
        let custom = template.replace(
            "set image(width: 100%, height: 65mm, fit: \"contain\")",
            "set image(width: 60pt)",
        );
        let custom_page = world
            .compile_with_template(document.source(), &metadata, &custom)
            .unwrap();
        let mut custom_images = Vec::new();
        for p in custom_page.pages() {
            images(&p.frame, &mut custom_images);
        }
        assert!((custom_images[0].0 - 60.0).abs() < 0.01);
    }
}

#[test]
fn image_insertions_and_cross_block_deletions_keep_the_projection_valid() {
    use crate::document::ImageBlock;
    let original = Document::from_source("A.\n\n#figure(image(\"images/parc.svg\"), caption: [Un *parc*.])\n\n#figure(image(\"images/second.png\"), caption: [B.])\n\nFin.").unwrap();
    let check = |view: &Projection| {
        assert_eq!(view.len(), view.text.chars().count());
        assert_eq!(view.blocks.len(), view.text.matches('\n').count() + 1);
        for (index, line) in view.line_ranges().iter().enumerate() {
            if let BlockKind::Image(id) = view.blocks[index] {
                assert!(view.images.contains_key(&id));
                assert_eq!(
                    view.text.chars().nth(line.start),
                    Some(crate::document::IMAGE_OBJECT)
                );
            }
        }
        let source = view.to_typst_mapped().0;
        assert!(
            !source.contains(crate::document::IMAGE_OBJECT),
            "{source:?}\n{view:?}"
        );
        Document::from_source(&source).unwrap_or_else(|error| panic!("{error}: {source:?}"));
    };
    for start in 0..original.view.len() {
        let mut edited = original.view.clone();
        edited.insert_text("é\n漢", start);
        check(&edited);
        let mut inserted = original.view.clone();
        inserted.insert_image(
            start,
            ImageBlock {
                path: "images/new.svg".into(),
                alt: String::new(),
            },
        );
        check(&inserted);
        if original
            .view
            .validate_object_insertion(&(start..start))
            .is_ok()
        {
            assert_eq!(inserted.images.len(), 3);
        } else {
            assert_eq!(inserted, original.view);
            assert_eq!(
                inserted.validate_edit(),
                Err(crate::document::edits::EditError::ProtectedBoundary)
            );
        }
        for end in start + 1..=original.view.len() {
            let mut deleted = original.view.clone();
            deleted.delete_char_range(start..end);
            check(&deleted);
            if original.view.validate_deletion(&(start..end)).is_err() {
                assert_eq!(deleted, original.view);
            }
        }
    }
    let mut adjacent = original.view.clone();
    let first = adjacent.image_range(1).unwrap();
    adjacent.delete_char_range(first.start..first.end + 1);
    assert_eq!(adjacent.images.len(), 1);
    assert_eq!(adjacent.images[&2].path, "images/second.png");
}

#[test]
fn letter_notes_round_trip_rich_text_and_keep_exact_source_positions() {
    for source in [
        "Avant.\n\n#postscript[Un *été* utile.\nSuite _précise_.\n\nSecond paragraphe.]\n\n#attachment[Formulaire *signé*]\n#attachment[#link(\"https://example.org\")[Rapport annuel]]",
        "#postscript[]",
        "#postscript[]\n#attachment[]",
        "#attachment[]",
        "#postscript[Texte]\n#attachment[]",
    ] {
        let mut document = Document::from_source(source).unwrap();
        let original = document.view.clone();
        document.sync_source();
        assert_eq!(
            Document::from_source(document.source()).unwrap().view,
            original
        );
        assert!(!document.view.text.contains("#postscript"));
        for (index, character) in document.view.text.chars().enumerate() {
            if !character.is_whitespace() {
                let byte = document.source_map.to_source(index);
                assert_eq!(document.source_map.to_visible(byte), index);
                assert!(
                    document.source()[document.source_map.characters[index].clone()]
                        .contains(character)
                );
            }
        }
    }
    for invalid in [
        "#postscript(\"texte\")",
        "#attachment[Un\n\nDeux]",
        "#postscript[#attachment[Imbriquée]]",
        "#quote(block: true)[#postscript[Imbriqué]]",
    ] {
        assert!(Document::from_source(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn letter_notes_render_after_signature_and_keep_bidirectional_selection_mapping() {
    use crate::{navigation::RenderSync, rendering::TypstWorld};
    fn text(frame: &typst::layout::Frame) -> String {
        frame
            .items()
            .map(|(_, item)| match item {
                typst::layout::FrameItem::Text(run) => run.text.to_string(),
                typst::layout::FrameItem::Group(group) => text(&group.frame),
                _ => String::new(),
            })
            .collect()
    }
    let source = "#postscript[POSTSCRIPTUM été *utile*.]\n\nCORPS PRINCIPAL.\n\n#attachment[FORMULAIRE signé]\n#attachment[RAPPORT annuel]";
    let mut document = Document::from_source(source).unwrap();
    document.sync_source();
    let source = document.source();
    let mut metadata = Metadata::blank(DocumentType::Courrier);
    metadata.courrier_mut().expediteur = "SIGNATAIRE".into();
    let template = Model::Personal.source();
    let mut world = TypstWorld::default();
    let pages = world
        .compile_with_template(source, &metadata, &template)
        .unwrap();
    let content: String = pages.pages().iter().map(|page| text(&page.frame)).collect();
    assert!(
        content.find("CORPS").unwrap() < content.rfind("SIGNATAIRE").unwrap(),
        "{content}"
    );
    assert!(
        content.rfind("SIGNATAIRE").unwrap() < content.find("POSTSCRIPTUM").unwrap(),
        "{content}"
    );
    assert!(
        content.find("POSTSCRIPTUM").unwrap() < content.find("FORMULAIRE").unwrap(),
        "{content}"
    );
    assert!(content.find("FORMULAIRE").unwrap() < content.find("RAPPORT").unwrap());
    assert_eq!(content.matches("POSTSCRIPTUM").count(), 1);
    let sync = RenderSync::new(1, world, pages, document.source_map.clone());
    for word in ["POSTSCRIPTUM", "FORMULAIRE", "RAPPORT"] {
        let at = document.view.text[..document.view.text.find(word).unwrap()]
            .chars()
            .count();
        assert!(
            !sync.selection_highlights(at..at + word.len()).is_empty(),
            "{word}"
        );
        let point = sync.forward(at + 3).unwrap();
        let back = sync.backward(point).unwrap();
        assert!((at..=at + word.len()).contains(&back), "{word}: {back}");
    }
}

#[test]
fn postscript_insertion_precedes_existing_enclosures_and_reuses_its_existing_block() {
    let mut document = Document::from_source("Corps.\n\n#attachment[Formulaire]").unwrap();
    let at = document.view.insert_letter_note(BlockKind::Postscript);
    document.view.insert_text("Complément.", at);
    document.sync_source();
    assert!(
        document.view.text.find("Complément.").unwrap()
            < document.view.text.find("Formulaire").unwrap()
    );
    let before = document.view.clone();
    assert_eq!(document.view.insert_letter_note(BlockKind::Postscript), at);
    assert_eq!(document.view, before);
    assert_eq!(
        Document::from_source(document.source()).unwrap().view,
        document.view
    );
}

#[test]
fn navigation_rebuilds_source_ranges_for_each_compilation_of_a_reused_world() {
    let mut world = crate::rendering::TypstWorld::default();
    let metadata = Metadata::demo(DocumentType::Courrier);
    let mut snapshots = Vec::new();
    for (revision, prefix) in ["", "Préambule allongé. ", "Court. "]
        .into_iter()
        .enumerate()
    {
        let document = Document::from_source(&format!(
            "{prefix}*Noël* consulte le #link(\"https://example.org\")[dossier] avec _Élodie_."
        ))
        .unwrap();
        let rendered = world
            .compile_with_template(document.source(), &metadata, &Model::Personal.source())
            .unwrap();
        let sync = crate::navigation::RenderSync::new(
            revision as u64,
            world.clone(),
            rendered,
            document.source_map.clone(),
        );
        let at = document.view.text[..document.view.text.find("dossier").unwrap()]
            .chars()
            .count()
            + 2;
        let point = sync.forward(at).unwrap();
        assert_eq!(sync.backward(point), Some(at));
        assert!(!sync.selection_highlights(at..at + 1).is_empty());
        snapshots.push((sync, at, point));
    }
    for (sync, at, point) in snapshots {
        assert_eq!(sync.forward(at), Some(point));
        assert_eq!(sync.backward(point), Some(at));
    }
}
