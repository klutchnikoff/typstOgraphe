use crate::{
    assets::Assets,
    content,
    document::DocumentLanguage,
    document_types::{DocumentType, Metadata},
    rendering::TypstWorld,
};
use typst::layout::{Frame, FrameItem};
fn text(frame: &Frame) -> String {
    frame
        .items()
        .map(|(_, item)| match item {
            FrameItem::Group(group) => text(&group.frame),
            FrameItem::Text(run) => run.text.to_string(),
            _ => String::new(),
        })
        .collect()
}
#[test]
fn literal_editor_quotes_follow_the_document_language_in_both_models() {
    let document = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"L'apostrophe et \"citation\"."}]}]})).unwrap();
    let (source, map) = content::source_mapped(&document, &Assets::default()).unwrap();
    for kind in [DocumentType::Courrier, DocumentType::Article] {
        let mut metadata = Metadata::blank(kind);
        let template = kind.default_model();
        let mut world = TypstWorld::default();
        world.set_dependencies(&crate::packages::for_template(&template));
        for (language, opening, closing) in [
            (DocumentLanguage::French, '«', '»'),
            (DocumentLanguage::English, '“', '”'),
        ] {
            metadata.options_mut().language = language;
            let rendered = world
                .compile_with_template(&source, &metadata, &template)
                .unwrap();
            let output = rendered
                .pages()
                .iter()
                .map(|page| text(&page.frame))
                .collect::<String>();
            assert!(
                output.contains("L’apostrophe"),
                "{kind:?}/{language:?}: {output}"
            );
            assert!(
                output.contains(opening) && output.contains(closing),
                "{kind:?}/{language:?}: {output}"
            );
            assert!(!output.contains('"') && !output.contains('\''), "{output}");
            let sync = crate::navigation::RenderSync::new(1, world, rendered, map.clone());
            let input = "L'apostrophe et \"citation\".";
            for (byte, character) in input
                .char_indices()
                .filter(|(_, c)| matches!(c, '\'' | '"'))
            {
                let at = 1 + input[..byte].encode_utf16().count();
                assert_eq!(&source[map.characters[at].clone()], &character.to_string());
                assert!(
                    !sync.selection_highlights_exact(at..at + 1).is_empty(),
                    "quote selection lost: {character}"
                );
                let point = sync.forward(at).expect("forward quote synchronization");
                assert!(
                    sync.backward(point).is_some(),
                    "backward quote synchronization lost"
                );
            }
            world = TypstWorld::default();
            world.set_dependencies(&crate::packages::for_template(&template));
        }
    }
}

#[test]
fn typst_models_can_disable_smart_quotes_without_turning_editor_text_into_code() {
    let input = "L'apostrophe et \"citation\" : #panic(\"literal\")";
    let document = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":input}]}]})).unwrap();
    let source = content::source(&document, &Assets::default()).unwrap();
    let mut world = TypstWorld::default();
    let rendered = world
        .compile_with_template(
            &source,
            &Metadata::blank(DocumentType::Courrier),
            "#let courrier(..args, body) = { set smartquote(enabled: false); body }",
        )
        .unwrap();
    let output = rendered
        .pages()
        .iter()
        .map(|page| text(&page.frame))
        .collect::<String>();
    assert!(output.contains(input), "{output}");
    assert_eq!(document.content[0].content[0].text.as_deref(), Some(input));
    assert_eq!(crate::document::typst_string("a\"b'c"), "\"a\\\"b'c\"");
}
