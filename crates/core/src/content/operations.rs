//! Éditions structurées partagées : cibles dans l'arbre, jamais positions de l'UI.
use super::{Mark, Node, catalogue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub id: String,
    #[serde(default)]
    pub params: Value,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub path: Vec<usize>,
    #[serde(default)]
    pub from: usize,
    #[serde(default)]
    pub to: usize,
}
fn node(kind: &str, content: Vec<Node>) -> Node {
    Node {
        kind: kind.into(),
        text: None,
        attrs: None,
        content,
        marks: vec![],
    }
}
fn text(value: &str, marks: Vec<Mark>) -> Node {
    Node {
        kind: "text".into(),
        text: Some(value.into()),
        attrs: None,
        content: vec![],
        marks,
    }
}
pub fn at<'a>(root: &'a Node, path: &[usize]) -> Result<&'a Node, String> {
    let mut n = root;
    for index in path {
        n = n.content.get(*index).ok_or("Cible structurée absente.")?;
    }
    Ok(n)
}
fn at_mut<'a>(root: &'a mut Node, path: &[usize]) -> Result<&'a mut Node, String> {
    let mut n = root;
    for index in path {
        n = n
            .content
            .get_mut(*index)
            .ok_or("Cible structurée absente.")?;
    }
    Ok(n)
}
pub fn inline_len(node: &Node) -> usize {
    if node.kind == "text" {
        node.text.as_deref().unwrap_or("").encode_utf16().count()
    } else if matches!(node.kind.as_str(), "softBreak" | "hardBreak" | "footnote") {
        1
    } else {
        node.content.iter().map(inline_len).sum()
    }
}
pub fn end_target(root: &Node) -> Target {
    fn visit(node: &Node, path: &mut Vec<usize>) -> Option<Target> {
        if matches!(
            node.kind.as_str(),
            "paragraph"
                | "heading"
                | "figure"
                | "descriptionTerm"
                | "descriptionDefinition"
                | "quoteAttribution"
                | "postscript"
                | "attachment"
        ) {
            let end = node.content.iter().map(inline_len).sum();
            return Some(Target {
                path: path.clone(),
                from: end,
                to: end,
            });
        }
        for index in (0..node.content.len()).rev() {
            path.push(index);
            if let Some(found) = visit(&node.content[index], path) {
                return Some(found);
            }
            path.pop();
        }
        None
    }
    visit(root, &mut vec![]).unwrap_or_default()
}
fn byte_at(text: &str, utf16: usize) -> Result<usize, String> {
    let mut count = 0;
    for (byte, ch) in text.char_indices() {
        if count == utf16 {
            return Ok(byte);
        }
        count += ch.len_utf16();
    }
    if count == utf16 {
        Ok(text.len())
    } else {
        Err("Position au milieu d'un caractère Unicode.".into())
    }
}
fn split(children: &[Node], offset: usize) -> Result<(Vec<Node>, Vec<Node>), String> {
    let (mut left, mut right, mut pos) = (vec![], vec![], 0);
    for child in children {
        let len = inline_len(child);
        if pos + len <= offset {
            left.push(child.clone())
        } else if pos >= offset {
            right.push(child.clone())
        } else if child.kind == "text" {
            let value = child.text.as_deref().unwrap_or("");
            let byte = byte_at(value, offset - pos)?;
            if byte > 0 {
                left.push(text(&value[..byte], child.marks.clone()))
            }
            if byte < value.len() {
                right.push(text(&value[byte..], child.marks.clone()))
            }
        } else {
            return Err("Une annotation ne peut pas être coupée.".into());
        }
        pos += len;
    }
    if offset > pos {
        return Err("Position hors de la zone.".into());
    }
    Ok((left, right))
}
fn target(params: &Value, context: &Target) -> Result<Target, String> {
    params
        .get("target")
        .map(|value| serde_json::from_value(value.clone()).map_err(|e| e.to_string()))
        .unwrap_or_else(|| Ok(context.clone()))
}
fn string<'a>(params: &'a Value, name: &str) -> Result<&'a str, String> {
    params
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Paramètre texte absent : {name}"))
}
pub fn apply(
    root: &mut Node,
    call: &Call,
    context: &mut Target,
    typing_marks: &mut Vec<Mark>,
    kind: crate::document_types::DocumentType,
) -> Result<(), String> {
    if !call.params.is_object() {
        return Err("Paramètres objet attendus.".into());
    }
    let capability = catalogue::find(&call.id);
    if let Some(c) = capability
        && c.letter_only
        && kind != crate::document_types::DocumentType::Courrier
    {
        return Err("Capacité absente de ce type.".into());
    }
    let t = target(&call.params, context)?;
    if matches!(
        call.id.as_str(),
        "text.insert"
            | "text.delete"
            | "text.bold"
            | "text.italic"
            | "text.link"
            | "text.remove_link"
            | "text.clear"
            | "insert.footnote"
    ) {
        let n = at_mut(root, &t.path)?;
        if !matches!(
            n.kind.as_str(),
            "paragraph"
                | "heading"
                | "figure"
                | "descriptionTerm"
                | "descriptionDefinition"
                | "quoteAttribution"
                | "postscript"
                | "attachment"
        ) {
            return Err("Une zone de texte est attendue.".into());
        }
        if t.from > t.to {
            return Err("Sélection inversée.".into());
        }
        let (left, rest) = split(&n.content, t.from)?;
        let (mut selected, right) = split(&rest, t.to - t.from)?;
        match call.id.as_str() {
            "text.insert" => {
                let value = string(&call.params, "text")?;
                let marks = call
                    .params
                    .get("marks")
                    .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
                    .transpose()?
                    .unwrap_or_else(|| typing_marks.clone());
                selected = if value.is_empty() {
                    vec![]
                } else {
                    vec![text(value, marks)]
                };
                *context = Target {
                    path: t.path.clone(),
                    from: t.from + value.encode_utf16().count(),
                    to: t.from + value.encode_utf16().count(),
                }
            }
            "text.delete" => {
                selected.clear();
                *context = Target {
                    path: t.path.clone(),
                    from: t.from,
                    to: t.from,
                }
            }
            "insert.footnote" => {
                let mut annotation = node("footnote", vec![]);
                annotation.attrs = Some(
                    json!({"text":call.params.get("text").and_then(Value::as_str).unwrap_or("")}),
                );
                selected = vec![annotation];
                *context = Target {
                    path: t.path.clone(),
                    from: t.from + 1,
                    to: t.from + 1,
                }
            }
            id => {
                let mark = match id {
                    "text.bold" => "bold",
                    "text.italic" => "italic",
                    _ => "link",
                };
                if t.from == t.to {
                    typing_marks.retain(|m| id != "text.clear" && m.kind != mark);
                    if !matches!(id, "text.clear" | "text.remove_link")
                        && call.params.get("value").and_then(Value::as_str) != Some("off")
                    {
                        typing_marks.push(Mark {
                            kind: mark.into(),
                            attrs: if mark == "link" {
                                Some(json!({"href":string(&call.params,"url")?}))
                            } else {
                                None
                            },
                        });
                    }
                }
                for child in &mut selected {
                    child.marks.retain(|m| {
                        if id == "text.clear" {
                            false
                        } else {
                            m.kind != mark
                        }
                    });
                    if !matches!(id, "text.clear" | "text.remove_link")
                        && call.params.get("value").and_then(Value::as_str) != Some("off")
                    {
                        child.marks.push(Mark {
                            kind: mark.into(),
                            attrs: if mark == "link" {
                                Some(json!({"href":string(&call.params,"url")?}))
                            } else {
                                None
                            },
                        })
                    }
                }
            }
        }
        n.content = left.into_iter().chain(selected).chain(right).collect();
        return Ok(());
    }
    if call.id.starts_with("block.heading_") || call.id == "block.paragraph" {
        let n = at_mut(root, &t.path)?;
        if !matches!(n.kind.as_str(), "paragraph" | "heading") {
            return Err("Paragraphe attendu.".into());
        }
        if let Some(level) = call.id.strip_prefix("block.heading_") {
            let level = level.parse::<u8>().map_err(|e| e.to_string())?;
            if !(1..=3).contains(&level) {
                return Err("Niveau de titre invalide.".into());
            }
            n.kind = "heading".into();
            n.attrs = Some(json!({"level":level}))
        } else {
            n.kind = "paragraph".into();
            n.attrs = None
        }
        return Ok(());
    }
    if call.id == "block.remove" {
        let (index, parent) = t.path.split_last().ok_or("Bloc attendu.")?;
        let n = at_mut(root, parent)?;
        if *index >= n.content.len() {
            return Err("Bloc absent.".into());
        }
        n.content.remove(*index);
        if root.content.is_empty() {
            root.content.push(node("paragraph", vec![]))
        }
        *context = end_target(root);
        return Ok(());
    }
    if call.id == "annotation.set" {
        let n = at_mut(root, &t.path)?;
        if n.kind != "footnote" {
            return Err("Note attendue.".into());
        }
        n.attrs = Some(json!({"text":string(&call.params,"text")?}));
        return Ok(());
    }
    let replacement = match call.id.as_str() {
        "block.bullet" | "block.numbered" => {
            let n = at(root, &t.path)?;
            node(
                if call.id == "block.bullet" {
                    "bulletList"
                } else {
                    "orderedList"
                },
                vec![node("listItem", vec![node("paragraph", n.content.clone())])],
            )
        }
        "block.description" => {
            let n = at(root, &t.path)?;
            node(
                "descriptionList",
                vec![node(
                    "descriptionItem",
                    vec![
                        node("descriptionTerm", vec![]),
                        node("descriptionDefinition", n.content.clone()),
                    ],
                )],
            )
        }
        "block.quote" => {
            let n = at(root, &t.path)?;
            node(
                "quote",
                vec![
                    node("paragraph", n.content.clone()),
                    node("quoteAttribution", vec![]),
                ],
            )
        }
        "letter.insert_postscript" | "letter.insert_attachment" => node(
            if call.id.ends_with("postscript") {
                "postscript"
            } else {
                "attachment"
            },
            vec![],
        ),
        "insert.image" => {
            let mut image = node("figure", vec![]);
            image.attrs = Some(
                json!({"src":string(&call.params,"src")?,"alt":call.params.get("alt").and_then(Value::as_str).unwrap_or("")}),
            );
            image
        }
        "insert.table" => {
            let rows = call.params.get("rows").and_then(Value::as_u64).unwrap_or(3);
            let columns = call
                .params
                .get("columns")
                .and_then(Value::as_u64)
                .unwrap_or(2);
            if !(1..=100).contains(&rows) || !(1..=12).contains(&columns) {
                return Err("Dimensions de tableau invalides.".into());
            }
            node(
                "table",
                (0..rows)
                    .map(|row| {
                        node(
                            "tableRow",
                            (0..columns)
                                .map(|_| {
                                    node(
                                        if row == 0
                                            && call
                                                .params
                                                .get("header")
                                                .and_then(Value::as_bool)
                                                .unwrap_or(true)
                                        {
                                            "tableHeader"
                                        } else {
                                            "tableCell"
                                        },
                                        vec![node("paragraph", vec![])],
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            )
        }
        _ => return Err(format!("Commande structurée inconnue : {}", call.id)),
    };
    if call.id.starts_with("block.") {
        *at_mut(root, &t.path)? = replacement;
    } else {
        let index = call
            .params
            .get("index")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(root.content.len());
        if index > root.content.len() {
            return Err("Position de bloc invalide.".into());
        }
        root.content.insert(index, replacement)
    }
    *context = end_target(root);
    Ok(())
}
/// Les contrats d'édition sont partagés avec la découverte des adaptateurs.
pub fn specifications() -> Vec<Value> {
    let target = json!({"type":"object","properties":{"path":{"type":"array","items":{"type":"integer","minimum":0}},"from":{"type":"integer","minimum":0,"default":0},"to":{"type":"integer","minimum":0,"default":0}},"required":["path"],"additionalProperties":false});
    let mut entries = vec![];
    for capability in catalogue::ALL {
        let mut params = catalogue::parameters(capability.id);
        params["properties"]
            .as_object_mut()
            .unwrap()
            .insert("target".into(), target.clone());
        if matches!(capability.id, "text.bold" | "text.italic") {
            params["properties"]["value"] =
                json!({"type":"string","enum":["on","off"],"default":"on"})
        }
        if capability.id == "text.link" {
            params["properties"]["url"] = json!({"type":"string"});
            params["required"] = json!(["url"])
        }
        entries.push(
            json!({"id":capability.id,"description":capability.description,"params_schema":params}),
        )
    }
    for (id, description, properties) in [
        (
            "text.insert",
            "Insert literal text; styles persist within the agent context until turned off.",
            json!({"text":{"type":"string"},"marks":{"type":"array","items":{"type":"object"}}}),
        ),
        (
            "text.delete",
            "Delete a range inside one editable zone.",
            json!({}),
        ),
        (
            "text.clear",
            "Remove styles from a selection or subsequent agent typing.",
            json!({}),
        ),
        ("text.remove_link", "Remove only link style.", json!({})),
        (
            "block.remove",
            "Remove one whole block addressed by a path.",
            json!({}),
        ),
        (
            "annotation.set",
            "Replace a footnote's plain text.",
            json!({"text":{"type":"string"}}),
        ),
    ] {
        let mut properties = properties;
        properties["target"] = target.clone();
        entries.push(json!({"id":id,"description":description,"params_schema":{"type":"object","properties":properties,"additionalProperties":false}}))
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_unicode_edits_and_styles_use_structured_targets() {
        let mut root = node("doc", vec![node("paragraph", vec![text("Aé😀B", vec![])])]);
        let mut context = Target {
            path: vec![0],
            from: 1,
            to: 4,
        };
        apply(
            &mut root,
            &Call {
                id: "text.bold".into(),
                params: json!({}),
            },
            &mut context,
            &mut vec![],
            crate::document_types::DocumentType::Courrier,
        )
        .unwrap();
        assert_eq!(root.content[0].content[1].text.as_deref(), Some("é😀"));
        context.from = 3;
        context.to = 3;
        assert!(
            apply(
                &mut root,
                &Call {
                    id: "text.insert".into(),
                    params: json!({"text":"x"})
                },
                &mut context,
                &mut vec![],
                crate::document_types::DocumentType::Courrier
            )
            .is_err()
        );
    }
}
