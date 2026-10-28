//! Construction of new shared subtrees only; never rebuild an existing root.
use super::{
    super::{
        Node,
        contract::{Error, State},
    },
    projection::invalid,
};
use serde_json::{Value, json};
use yrs::{
    Any, Array, ArrayPrelim, Doc, Map, MapPrelim, Text, TextPrelim, Transact, TransactionMut, Xml,
    XmlElementPrelim, XmlFragment, XmlFragmentPrelim, XmlTextPrelim,
};

fn any(value: &Value) -> Result<Any, Error> {
    Any::from_json(&value.to_string()).map_err(|e| invalid(e.to_string()))
}
pub(super) fn children(
    parent: yrs::XmlFragmentRef,
    tx: &mut TransactionMut,
    nodes: &[Node],
    start: u32,
    depth: usize,
    count: &mut usize,
) -> Result<(), Error> {
    if depth > 16 && !nodes.is_empty() {
        return Err(Error::StateLimitExceeded);
    }
    for (index, node) in nodes.iter().enumerate() {
        *count += 1;
        if *count > 20_000 {
            return Err(Error::StateLimitExceeded);
        }
        if node.kind == "text" {
            let text = node.text.as_deref().ok_or_else(|| invalid("text_absent"))?;
            let value = parent.insert(tx, start + index as u32, XmlTextPrelim::new(text));
            if !node.marks.is_empty() && !text.is_empty() {
                let attrs = node
                    .marks
                    .iter()
                    .map(|mark| {
                        Ok((
                            mark.kind.as_str().into(),
                            any(mark.attrs.as_ref().unwrap_or(&json!({})))?,
                        ))
                    })
                    .collect::<Result<_, Error>>()?;
                value.format(tx, 0, text.encode_utf16().count() as u32, attrs);
            }
        } else {
            let value = parent.insert(
                tx,
                start + index as u32,
                XmlElementPrelim::empty(node.kind.as_str()),
            );
            if let Some(attrs) = node.attrs.as_ref().and_then(Value::as_object) {
                for (key, v) in attrs {
                    if !v.is_null() {
                        value.insert_attribute(tx, key.as_str(), any(v)?);
                    }
                }
            }
            if !node.marks.is_empty() {
                value.insert_attribute(
                    tx,
                    "__marks",
                    any(&serde_json::to_value(&node.marks).map_err(|e| invalid(e.to_string()))?)?,
                );
            }
            let fragment: &yrs::XmlFragmentRef = value.as_ref();
            children(fragment.clone(), tx, &node.content, 0, depth + 1, count)?;
        }
    }
    Ok(())
}
pub(super) fn write(doc: &Doc, state: &State) -> Result<(), Error> {
    let body = doc.get_or_insert_xml_fragment("default");
    let settings = doc.get_or_insert_map("settings");
    let mut tx = doc.transact_mut();
    children(body, &mut tx, &state.document.content, 0, 1, &mut 0)?;
    settings.insert(
        &mut tx,
        "document",
        any(&serde_json::to_value(&state.settings).map_err(|e| invalid(e.to_string()))?)?,
    );
    if let Some(schema) = state
        .settings
        .metadata
        .document_type()
        .structured_metadata()
    {
        let fields =
            serde_json::to_value(&state.settings.metadata).map_err(|e| invalid(e.to_string()))?;
        let collections = settings.insert(&mut tx, "collections", MapPrelim::default());
        for definition in schema["collections"].as_array().into_iter().flatten() {
            let name = definition["parameter"]
                .as_str()
                .ok_or_else(|| invalid("collection_parameter_absent"))?;
            let map = collections.insert(&mut tx, name, MapPrelim::default());
            let order = settings.insert(&mut tx, format!("order:{name}"), ArrayPrelim::default());
            for (index, value) in fields["fields"][name]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                let id = value["id"]
                    .as_str()
                    .ok_or_else(|| invalid("record_id_absent"))?;
                order.insert(&mut tx, index as u32, id);
                let record = map.insert(&mut tx, id, MapPrelim::default());
                fill_record(record, &mut tx, definition, value, id)?;
            }
        }
    }
    Ok(())
}

/// Descriptor-driven construction shared by bootstrap and record.add.
pub(super) fn fill_record(
    record: yrs::MapRef,
    tx: &mut TransactionMut,
    definition: &Value,
    value: &Value,
    id: &str,
) -> Result<(), Error> {
    record.insert(tx, "id", id);
    for field in definition["fields"].as_array().into_iter().flatten() {
        let key = field["key"]
            .as_str()
            .ok_or_else(|| invalid("record_field_absent"))?;
        let data = value.get(key).unwrap_or(&field["default"]);
        match field["kind"].as_str() {
            Some("rich" | "rich_inline") => {
                let rich = record.insert(tx, key, XmlFragmentPrelim::default());
                let node: Node =
                    serde_json::from_value(data.clone()).map_err(|e| invalid(e.to_string()))?;
                children(rich, tx, &node.content, 0, 1, &mut 0)?;
            }
            Some("text" | "multiline") => {
                record.insert(
                    tx,
                    key,
                    TextPrelim::new(
                        data.as_str()
                            .ok_or_else(|| invalid("record_text_expected"))?,
                    ),
                );
            }
            _ => {
                record.insert(tx, key, any(data)?);
            }
        }
    }
    Ok(())
}
