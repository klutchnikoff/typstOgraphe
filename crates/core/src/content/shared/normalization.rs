//! Representation equivalences only. Preserve every non-default attribute and value.
use super::super::{Node, contract::Settings};
use serde_json::{Value, json};

pub(super) fn node(value: &Node) -> Node {
    let mut n = value.clone();
    n.content = value.content.iter().map(node).collect();
    if let Some(attrs) = n.attrs.as_mut().and_then(Value::as_object_mut) {
        if let Some(value) = attrs.remove("__marks")
            && n.marks.is_empty()
            && !value.is_null()
            && let Ok(marks) = serde_json::from_value(value)
        {
            n.marks = marks;
        }
        // The Yjs binding omits null attributes; the editor materializes them.
        attrs.retain(|_, v| !v.is_null());
        for (key, default) in match n.kind.as_str() {
            "tableCell" | "tableHeader" => vec![("colspan", json!(1)), ("rowspan", json!(1))],
            "orderedList" => vec![("start", json!(1))],
            "heading" => vec![("level", json!(1))],
            _ => vec![],
        } {
            if attrs.get(key) == Some(&default) {
                attrs.remove(key);
            }
        }
        if attrs.is_empty() {
            n.attrs = None;
        }
    }
    for mark in &mut n.marks {
        if let Some(attrs) = mark.attrs.as_mut().and_then(Value::as_object_mut) {
            attrs.retain(|_, v| !v.is_null());
            if attrs.is_empty() {
                mark.attrs = None;
            }
        }
    }
    n.marks.sort_by(|a, b| {
        a.kind.cmp(&b.kind).then_with(|| {
            a.attrs
                .as_ref()
                .map(Value::to_string)
                .cmp(&b.attrs.as_ref().map(Value::to_string))
        })
    });
    n.marks.dedup();
    let mut merged: Vec<Node> = vec![];
    for child in n.content {
        if let Some(last) = merged.last_mut()
            && child.kind == "text"
            && last.kind == "text"
            && child.marks == last.marks
            && child.attrs == last.attrs
        {
            last.text
                .get_or_insert_default()
                .push_str(child.text.as_deref().unwrap_or_default());
        } else {
            merged.push(child);
        }
    }
    n.content = merged;
    n
}
fn rich_values(value: &mut Value) {
    if value.get("type").and_then(Value::as_str) == Some("doc")
        && let Ok(parsed) = serde_json::from_value::<Node>(value.clone())
    {
        *value = serde_json::to_value(node(&parsed)).expect("node serialization");
        return;
    }
    match value {
        Value::Object(map) => map.values_mut().for_each(rich_values),
        Value::Array(list) => list.iter_mut().for_each(rich_values),
        _ => (),
    }
}
pub(super) fn settings(settings: &Settings) -> Value {
    let mut result = serde_json::to_value(settings).expect("settings serialization");
    rich_values(&mut result);
    result
}
