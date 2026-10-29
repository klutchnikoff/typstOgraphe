use crate::document_types::{DocumentType, Metadata, models::Model};

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
    let source = "Alpha\nBeta\n\nGamma";
    let mut world = crate::rendering::TypstWorld::default();
    let template = "#let courrier(..args, body) = { set page(width: 200mm, height: 200mm); set par(spacing: 20pt); body }";
    let rendered = world
        .compile_with_template(source, &Metadata::demo(DocumentType::Courrier), template)
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
fn literal_metadata_and_structured_body_cannot_execute_typst_markup() {
    let metadata = Metadata::Courrier(crate::document_types::courrier::Metadata {
        objet: "Une demande \"citée\" avec \\ et #panic(\"NON\")".into(),
        ..Default::default()
    });
    let node = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Bonjour. #panic(\"NON\") [texte] *littéral*"}]}]})).unwrap();
    let source = crate::content::source(&node, &Default::default()).unwrap();
    let mut world = crate::rendering::TypstWorld::default();
    let template = DocumentType::Courrier.default_model();
    world.set_dependencies(&crate::packages::for_template(&template));
    let document = world
        .compile_with_template(&source, &metadata, &template)
        .unwrap();
    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(!document.pages().is_empty());
}
