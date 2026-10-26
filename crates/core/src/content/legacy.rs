//! Import explicite de la projection historique vers le contrat structuré.
use super::{Mark, Node};
use crate::document::{BlockKind, Document, Projection, structure::BlockContent};
fn node(kind: &str, content: Vec<Node>) -> Node {
    Node {
        kind: kind.into(),
        text: None,
        attrs: None,
        content,
        marks: vec![],
    }
}
fn inline(view: &Projection) -> Vec<Node> {
    let mut result: Vec<Node> = vec![];
    for (index, ch) in view.text.chars().enumerate() {
        if ch == '\n' {
            result.push(node("softBreak", vec![]));
            continue;
        }
        let style = view.styles.get(index).cloned().unwrap_or_default();
        let mut marks = vec![];
        if style.bold {
            marks.push(Mark {
                kind: "bold".into(),
                attrs: None,
            })
        }
        if style.italic {
            marks.push(Mark {
                kind: "italic".into(),
                attrs: None,
            })
        }
        if let Some(url) = style.link {
            marks.push(Mark {
                kind: "link".into(),
                attrs: Some(serde_json::json!({"href":url})),
            })
        }
        if let Some(previous) = result
            .last_mut()
            .filter(|n| n.kind == "text" && n.marks == marks)
        {
            previous.text.as_mut().unwrap().push(ch)
        } else {
            result.push(Node {
                kind: "text".into(),
                text: Some(ch.to_string()),
                attrs: None,
                content: vec![],
                marks,
            })
        }
    }
    result
}
pub fn import(source: &str) -> Result<Node, String> {
    let document = Document::from_source(source).map_err(|e| e.to_string())?;
    let mut content = vec![];
    for block in &document.structure.blocks {
        let item = match &block.content {
            BlockContent::Text { kind, body } => match kind {
                BlockKind::Heading(level) => {
                    let mut heading = node("heading", inline(&body.view));
                    heading.attrs = Some(serde_json::json!({"level":level}));
                    heading
                }
                BlockKind::Bullet | BlockKind::Numbered => node(
                    if *kind == BlockKind::Bullet {
                        "bulletList"
                    } else {
                        "orderedList"
                    },
                    vec![node(
                        "listItem",
                        vec![node("paragraph", inline(&body.view))],
                    )],
                ),
                BlockKind::Postscript => node("postscript", inline(&body.view)),
                BlockKind::Attachment => node("attachment", inline(&body.view)),
                _ => node("paragraph", inline(&body.view)),
            },
            BlockContent::Description { term, definition } => node(
                "descriptionList",
                vec![node(
                    "descriptionItem",
                    vec![
                        node("descriptionTerm", inline(&term.view)),
                        node("descriptionDefinition", inline(&definition.view)),
                    ],
                )],
            ),
            BlockContent::Quote { body, attribution } => node(
                "quote",
                vec![
                    node("paragraph", inline(&body.view)),
                    node("quoteAttribution", inline(&attribution.view)),
                ],
            ),
            BlockContent::Image { image, caption, .. } => {
                let mut figure = node("figure", inline(&caption.view));
                figure.attrs = Some(serde_json::json!({"src":image.path,"alt":image.alt}));
                figure
            }
        };
        if matches!(
            item.kind.as_str(),
            "bulletList" | "orderedList" | "descriptionList"
        ) && content
            .last()
            .is_some_and(|last: &Node| last.kind == item.kind)
        {
            content.last_mut().unwrap().content.extend(item.content)
        } else {
            content.push(item)
        }
    }
    if content.is_empty() {
        content.push(node("paragraph", vec![]))
    }
    Ok(node("doc", content))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_styles_and_blocks_retain_their_semantics() {
        let original = "= Titre\n\nUn *texte gras* et _italique_.\n\n- Premier\n- Second\n\n/ Terme: Définition\n\n#quote(block: true, attribution: [Source])[Une citation.]";
        let doc = import(original).unwrap();
        let encoded = serde_json::to_string(&doc).unwrap();
        for expected in [
            "heading",
            "bold",
            "italic",
            "bulletList",
            "descriptionList",
            "quoteAttribution",
        ] {
            assert!(encoded.contains(expected), "{expected}")
        }
        let source = super::super::source(&doc, &Default::default()).unwrap();
        let kind = crate::document_types::DocumentType::Courrier;
        let template = kind.default_model();
        let mut world = crate::rendering::TypstWorld::default();
        world.set_dependencies(&crate::packages::for_template(&template));
        world
            .compile_with_template(
                &source,
                &crate::document_types::Metadata::blank(kind),
                &template,
            )
            .unwrap();
    }
}
