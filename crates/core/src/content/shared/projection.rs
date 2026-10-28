//! Interpret shared types using the same document-type descriptors as the editor.
use super::super::{
    Mark, Node,
    contract::{Error, Settings, State},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use yrs::{Array, Doc, GetString, Map, Out, ReadTxn, Text, Transact, Xml, XmlFragment, XmlOut};

pub(super) fn invalid(reason: impl Into<String>) -> Error {
    Error::InvalidSharedState {
        reason: reason.into(),
    }
}
fn value<T: ReadTxn>(out: Out, tx: &T) -> Result<Value, Error> {
    match out {
        Out::Any(v) => serde_json::to_value(v).map_err(|e| invalid(e.to_string())),
        Out::YText(v) => Ok(json!(v.get_string(tx))),
        _ => Err(invalid("expected_scalar_value")),
    }
}
pub(super) fn xml<T: ReadTxn>(
    items: impl Iterator<Item = XmlOut>,
    tx: &T,
    depth: usize,
    count: &mut usize,
) -> Result<Vec<Node>, Error> {
    let mut nodes = vec![];
    for item in items {
        if depth > 16 {
            return Err(Error::StateLimitExceeded);
        }
        match item {
            XmlOut::Element(element) => {
                let mut attrs = serde_json::Map::new();
                for (key, out) in element.attributes(tx) {
                    attrs.insert(key.into(), value(out, tx)?);
                }
                let marks = match attrs.remove("__marks") {
                    Some(Value::Null) | None => vec![],
                    Some(value) => serde_json::from_value(value)
                        .map_err(|e| invalid(format!("invalid_inline_marks: {e}")))?,
                };
                nodes.push(Node {
                    kind: element.tag().to_string(),
                    text: None,
                    attrs: (!attrs.is_empty()).then_some(Value::Object(attrs)),
                    marks,
                    content: xml(element.children(tx), tx, depth + 1, count)?,
                });
                *count += 1;
            }
            XmlOut::Text(text) => {
                for delta in text.diff(tx, |_| ()) {
                    let Out::Any(yrs::Any::String(text)) = delta.insert else {
                        return Err(invalid("non_text_xml_embed"));
                    };
                    if text.is_empty() {
                        continue;
                    }
                    let mut marks = vec![];
                    for (key, attrs) in delta
                        .attributes
                        .as_deref()
                        .into_iter()
                        .flat_map(|attrs| attrs.iter())
                    {
                        // y-tiptap permits attributed mark keys with an eight-character hash.
                        let name = key
                            .rsplit_once("--")
                            .filter(|(_, suffix)| {
                                suffix.len() == 8
                                    && suffix
                                        .bytes()
                                        .all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c))
                            })
                            .map(|(name, _)| name)
                            .unwrap_or(key);
                        marks.push(Mark {
                            kind: name.into(),
                            attrs: Some(
                                serde_json::to_value(attrs).map_err(|e| invalid(e.to_string()))?,
                            ),
                        });
                    }
                    marks.sort_by(|a, b| a.kind.cmp(&b.kind));
                    nodes.push(Node {
                        kind: "text".into(),
                        text: Some(text.to_string()),
                        attrs: None,
                        content: vec![],
                        marks,
                    });
                    *count += 1;
                }
            }
            _ => return Err(invalid("unsupported_xml_child")),
        }
        if *count > 20_000 {
            return Err(Error::StateLimitExceeded);
        }
    }
    Ok(nodes)
}
fn rich<T: ReadTxn>(fragment: yrs::XmlFragmentRef, tx: &T) -> Result<Value, Error> {
    Ok(json!({"type":"doc", "content":xml(fragment.children(tx), tx, 1, &mut 0)?}))
}
pub(super) fn read(doc: &Doc) -> Result<State, Error> {
    let tx = doc.transact();
    let root = tx
        .get_map("settings")
        .ok_or_else(|| invalid("settings_absent"))?;
    let base: Settings = serde_json::from_value(value(
        root.get(&tx, "document")
            .ok_or_else(|| invalid("document_settings_absent"))?,
        &tx,
    )?)
    .map_err(|e| invalid(e.to_string()))?;
    let mut settings = serde_json::to_value(&base).map_err(|e| invalid(e.to_string()))?;
    let fields = settings["metadata"]["fields"]
        .as_object_mut()
        .ok_or_else(|| invalid("metadata_fields_absent"))?;
    for field in base.metadata.field_values() {
        let key = serde_json::to_value(field.field).map_err(|e| invalid(e.to_string()))?;
        if let Some(out) = root.get(
            &tx,
            key.as_str().ok_or_else(|| invalid("field_key_absent"))?,
        ) {
            fields.insert(field.parameter.into(), value(out, &tx)?);
        }
    }
    for key in [
        "paper",
        "language",
        "landscape",
        "bullet_marker",
        "spelling_ignored",
    ] {
        if let Some(out) = root.get(&tx, &format!("options.{key}")) {
            fields["options"][key] = value(out, &tx)?;
        }
    }
    if let Some(out) = root.get(&tx, "signature") {
        fields.insert("signature".into(), value(out, &tx)?);
    }
    if let Some(out) = root.get(&tx, "collections") {
        let Out::YMap(collections) = out else {
            return Err(invalid("collections_must_be_shared_map"));
        };
        if let Some(schema) = base.metadata.document_type().structured_metadata() {
            for definition in schema["collections"].as_array().into_iter().flatten() {
                let name = definition["parameter"]
                    .as_str()
                    .ok_or_else(|| invalid("collection_parameter_absent"))?;
                let Some(out) = collections.get(&tx, name) else {
                    continue;
                };
                let Out::YMap(records) = out else {
                    return Err(invalid("records_must_be_shared_map"));
                };
                let Some(Out::YArray(order)) = root.get(&tx, &format!("order:{name}")) else {
                    return Err(invalid("record_order_absent"));
                };
                let mut values = vec![];
                let mut ids = BTreeSet::new();
                for id in order.iter(&tx) {
                    let Out::Any(yrs::Any::String(id)) = id else {
                        return Err(invalid("record_id_must_be_text"));
                    };
                    // Concurrent reordering can leave duplicate or deleted IDs in the order.
                    if !ids.insert(id.to_string()) {
                        continue;
                    }
                    let Some(out) = records.get(&tx, &id) else {
                        continue;
                    };
                    let Out::YMap(record) = out else {
                        return Err(invalid("record_must_be_shared_map"));
                    };
                    let mut item = json!({"id":id.as_ref()});
                    if record.get(&tx, "id").map(|v| value(v, &tx)).transpose()?
                        != Some(item["id"].clone())
                    {
                        return Err(invalid("record_identity_mismatch"));
                    }
                    for field in definition["fields"].as_array().into_iter().flatten() {
                        let key = field["key"]
                            .as_str()
                            .ok_or_else(|| invalid("record_field_absent"))?;
                        item[key] = match record.get(&tx, key) {
                            Some(Out::YXmlFragment(fragment))
                                if matches!(
                                    field["kind"].as_str(),
                                    Some("rich" | "rich_inline")
                                ) =>
                            {
                                rich(fragment, &tx)?
                            }
                            Some(out) => value(out, &tx)?,
                            None => field["default"].clone(),
                        };
                    }
                    values.push(item);
                }
                fields.insert(name.into(), json!(values));
            }
        }
    }
    let document = Node {
        kind: "doc".into(),
        text: None,
        attrs: None,
        marks: vec![],
        content: xml(
            tx.get_xml_fragment("default")
                .ok_or_else(|| invalid("body_absent"))?
                .children(&tx),
            &tx,
            1,
            &mut 0,
        )?,
    };
    Ok(State::new(
        document,
        serde_json::from_value(settings).map_err(|e| invalid(e.to_string()))?,
    ))
}
