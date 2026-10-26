//! Carte structure → octets Typst. Les balises temporaires ne sont jamais conservées.
use super::{Node, schema};
use crate::{assets::Assets, document::SourceMap};
use std::ops::Range;
struct Entry {
    position: usize,
    value: String,
    atom: bool,
}
pub(super) fn literal(value: &str) -> String {
    crate::document::escape_markup(value)
        .replace('.', "\\.")
        .replace('\n', " ")
        .replace('\r', " ")
}
pub fn source_mapped(document: &Node, assets: &Assets) -> Result<(String, SourceMap), String> {
    let mut marked = document.clone();
    let mut entries = vec![];
    fn visit(node: &mut Node, position: usize, entries: &mut Vec<Entry>) -> usize {
        if node.kind == "text" || node.kind == "footnote" {
            let value = if node.kind == "text" {
                node.text.clone().unwrap_or_default()
            } else {
                node.attrs
                    .as_ref()
                    .and_then(|v| v.get("text"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .into()
            };
            let atom = node.kind == "footnote";
            let size = if atom {
                1
            } else {
                value.encode_utf16().count()
            };
            let id = entries.len();
            entries.push(Entry {
                position,
                value,
                atom,
            });
            let attrs = node.attrs.get_or_insert_with(|| serde_json::json!({}));
            attrs["_mapping"] = serde_json::json!(id);
            return size;
        }
        if matches!(node.kind.as_str(), "softBreak" | "hardBreak") {
            return 1;
        }
        let mut offset = position + usize::from(node.kind != "doc");
        for child in &mut node.content {
            offset += visit(child, offset, entries)
        }
        offset - position + usize::from(node.kind != "doc")
    }
    let size = visit(&mut marked, 0, &mut entries);
    let raw = schema::generate(&marked, assets, true)?;
    let mut output = String::with_capacity(raw.len());
    let mut characters: Vec<Range<usize>> = vec![0..0; size + 1];
    let mut remaining = raw.as_str();
    while let Some(index) = remaining.find("/*@typstographe:") {
        output.push_str(&remaining[..index]);
        let after = &remaining[index + 16..];
        let end = after.find("*/").ok_or("Balise interne invalide.")?;
        let id: usize = after[..end]
            .parse()
            .map_err(|_| "Identité interne invalide.")?;
        let entry = entries.get(id).ok_or("Carte interne invalide.")?;
        let start = output.len();
        let mut byte = start;
        let mut position = entry.position;
        for ch in entry.value.chars() {
            let width = literal(&ch.to_string()).len();
            if !entry.atom {
                for _ in 0..ch.len_utf16() {
                    characters[position] = byte..byte + width;
                    position += 1
                }
            }
            byte += width
        }
        if entry.atom {
            characters[position] = start..byte
        }
        remaining = &after[end + 2..];
    }
    output.push_str(remaining);
    let first = characters
        .iter()
        .find(|range| !range.is_empty())
        .cloned()
        .unwrap_or(0..0);
    let mut previous = first;
    for range in &mut characters {
        if range.start == range.end {
            *range = previous.clone()
        } else {
            previous = range.clone()
        }
    }
    let source_len = output.len();
    Ok((
        output,
        SourceMap {
            characters,
            source_len,
        },
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_render_resolves_structured_selection_in_both_directions() {
        let node=serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Bonjour collègue."}]}]})).unwrap();
        let (source, map) = source_mapped(&node, &Assets::default()).unwrap();
        let mut world = crate::rendering::TypstWorld::default();
        let metadata =
            crate::document_types::Metadata::demo(crate::document_types::DocumentType::Courrier);
        let template = crate::document_types::DocumentType::Courrier.default_model();
        world.set_dependencies(&crate::packages::for_template(&template));
        let document = world
            .compile_with_template(&source, &metadata, &template)
            .unwrap();
        let sync = crate::navigation::RenderSync::new(7, world, document, map);
        let point = sync.forward(3).expect("forward");
        let back = sync.backward(point).expect("backward");
        assert!((1..=8).contains(&back), "{back}");
        assert!(!sync.selection_highlights_exact(1..8).is_empty());
    }
    #[test]
    fn mapping_keeps_the_exact_source_and_utf16_positions() {
        let node=serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"é😀 #x.","marks":[{"type":"bold"}]}]}]})).unwrap();
        let (source, map) = source_mapped(&node, &Assets::default()).unwrap();
        assert_eq!(
            source,
            super::super::source(&node, &Assets::default()).unwrap()
        );
        assert_eq!(&source[map.characters[1].clone()], "é");
        assert_eq!(map.characters[2], map.characters[3]);
        assert_eq!(&source[map.characters[2].clone()], "😀");
        assert_eq!(&source[map.characters[5].clone()], "\\#");
    }
}
