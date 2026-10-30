//! Explicit inline objects and in-place public attributes. No editor positions or UI policy.
use super::{commands, projection, seed, structure, targets};
use crate::content::{
    Node,
    commands::{Command, TextTarget, TextZone, TreeTarget},
    contract::{Error, State, TargetReason, UnavailableReason},
};
use serde_json::{Map, Value};
use yrs::{Any, Doc, Text, Transact, Xml, XmlFragment, XmlOut};

fn parameter(name: &str) -> Error {
    Error::InvalidParameters {
        parameter: name.into(),
    }
}
fn attributes(
    doc: &Doc,
    state: &State,
    target: &TreeTarget,
    attrs: &Map<String, Value>,
) -> Result<(), Error> {
    let root = structure::root(doc, state, &target.root)?;
    let element = targets::element(root, &doc.transact(), &target.path)?;
    // Public parameters only. Content, private binding attributes and container
    // roles are edited through their own commands, never through an escape hatch.
    for (key, value) in attrs {
        let capability = crate::content::catalogue::ALL
            .iter()
            .find(|entry| entry.node == Some(element.tag().as_ref()))
            .ok_or_else(|| parameter("attrs"))?;
        let schema = crate::content::catalogue::parameters(capability.id);
        let property = schema["properties"]
            .as_object()
            .and_then(|properties| {
                properties.iter().find_map(|(name, field)| {
                    let hint = &field["x-editor"];
                    let attribute = hint["attribute"].as_str().unwrap_or(name);
                    let external = hint["storage"] == "structure"
                        || matches!(hint["placement"].as_str(), Some("structure" | "creation"))
                        || (hint["placement"] == "body"
                            && !matches!(element.tag().as_ref(), "footnote" | "citation"));
                    (attribute == key && !external).then_some(field)
                })
            })
            .ok_or_else(|| parameter("attrs"))?;
        let valid = match property["type"].as_str() {
            Some("string") => value.as_str().is_some_and(|s| s.len() <= 512_000),
            Some("integer") => value.as_u64().is_some(),
            Some("boolean") => value.is_boolean(),
            Some("array") => value.is_array(),
            _ => false,
        } && property["enum"]
            .as_array()
            .is_none_or(|values| values.contains(value))
            && property["minimum"]
                .as_f64()
                .is_none_or(|min| value.as_f64().is_some_and(|n| n >= min))
            && property["maximum"]
                .as_f64()
                .is_none_or(|max| value.as_f64().is_some_and(|n| n <= max));
        if !valid {
            return Err(parameter("attrs"));
        }
    }
    let mut tx = doc.transact_mut();
    for (key, value) in attrs {
        let value = Any::from_json(&value.to_string()).map_err(|_| parameter("attrs"))?;
        if element.get_attribute(&tx, key) != Some(yrs::Out::Any(value.clone())) {
            element.insert_attribute(&mut tx, key.as_str(), value);
        }
    }
    Ok(())
}
fn inline(doc: &Doc, state: &State, target: &TextTarget, node: &Node) -> Result<(), Error> {
    let line_break = matches!(node.kind.as_str(), "softBreak" | "hardBreak");
    if !line_break && !["footnote", "citation"].contains(&node.kind.as_str()) {
        return Err(parameter("node"));
    }
    if line_break
        && let TextZone::Record {
            collection,
            id,
            field,
            ..
        } = &target.zone
        && targets::record(doc, &doc.transact(), state, collection, id, field)?.1["kind"]
            == "rich_inline"
    {
        return Err(Error::CommandUnavailable {
            command: "inline.replace".into(),
            reason: UnavailableReason::UnsupportedTransformation,
        });
    }
    structure::new_subtree(node)?;
    let targets::Resolved::Rich(parent) = commands::range(doc, state, target)? else {
        return Err(Error::CommandUnavailable {
            command: "inline.replace".into(),
            reason: UnavailableReason::PlainTextOnly,
        });
    };
    let mut tx = doc.transact_mut();
    commands::replace_rich(&parent, &mut tx, target, "", Default::default())?;
    let children: Vec<_> = parent.children(&tx).collect();
    let mut offset = 0;
    let mut slot = children.len() as u32;
    let mut inserted = vec![node.clone()];
    for (index, child) in children.iter().enumerate() {
        let len = match child {
            XmlOut::Text(text) => text.len(&tx),
            _ => 1,
        };
        if target.from == offset {
            slot = index as u32;
            break;
        }
        if target.from < offset + len {
            let XmlOut::Text(text) = child else {
                return Err(targets::invalid(TargetReason::NotTextZone));
            };
            // Split the binding text once to create an XML slot. Keep the prefix
            // and all other elements; rebuild only the suffix of this text.
            let mut remaining = target.from - offset;
            for mut part in projection::xml(std::iter::once(child.clone()), &tx, 1, &mut 0)? {
                let text = part.text.as_mut().ok_or_else(|| parameter("node"))?;
                let length = text.encode_utf16().count() as u32;
                if remaining >= length {
                    remaining -= length;
                    continue;
                }
                *text = text[targets::byte_at(text, remaining)?..].to_owned();
                inserted.push(part);
                remaining = 0;
            }
            text.remove_range(&mut tx, target.from - offset, offset + len - target.from);
            slot = index as u32 + 1;
            break;
        }
        offset += len;
    }
    let root: &yrs::XmlFragmentRef = parent.as_ref();
    seed::children(root.clone(), &mut tx, &inserted, slot, 1, &mut 0)
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    match command {
        Command::SetNodeAttrs { target, attrs } => attributes(doc, state, target, attrs),
        Command::ReplaceInline { target, node } => inline(doc, state, target, node),
        _ => unreachable!(),
    }
}
