use super::super::{
    commands::{FieldTarget, TextZone, ZoneInfo},
    contract::{Error, State, TargetReason},
};
use serde_json::Value;
use yrs::{
    Doc, GetString, Map, MapRef, Out, ReadTxn, Transact, Xml, XmlElementRef, XmlFragment,
    XmlFragmentRef, XmlOut,
};

pub(in crate::content) fn invalid(reason: TargetReason) -> Error {
    Error::InvalidTarget { reason }
}
pub(in crate::content) fn element<T: ReadTxn>(
    root: XmlFragmentRef,
    tx: &T,
    path: &[usize],
) -> Result<XmlElementRef, Error> {
    if path.len() > 16 {
        return Err(Error::StateLimitExceeded);
    }
    let (index, rest) = path
        .split_first()
        .ok_or_else(|| invalid(TargetReason::NotTextZone))?;
    let mut count = 0;
    for child in root.children(tx) {
        // One Y.XmlText can project to several styled text nodes. Paths name the
        // public tree, not physical XML child indices or byte offsets.
        let width = super::projection::xml(std::iter::once(child.clone()), tx, 1, &mut 0)?.len();
        if *index < count + width {
            let XmlOut::Element(value) = child else {
                return Err(invalid(TargetReason::NotTextZone));
            };
            return if rest.is_empty() {
                Ok(value)
            } else {
                element(
                    <XmlElementRef as AsRef<XmlFragmentRef>>::as_ref(&value).clone(),
                    tx,
                    rest,
                )
            };
        }
        count += width;
    }
    Err(invalid(TargetReason::MissingNode))
}
pub(in crate::content) fn record<T: ReadTxn>(
    _doc: &Doc,
    tx: &T,
    state: &State,
    collection: &str,
    id: &str,
    field: &str,
) -> Result<(MapRef, Value), Error> {
    let schema = state
        .settings
        .metadata
        .document_type()
        .structured_metadata()
        .ok_or_else(|| invalid(TargetReason::MissingCollection))?;
    let definition = schema["collections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["parameter"] == collection)
        .ok_or_else(|| invalid(TargetReason::MissingCollection))?;
    let descriptor = definition["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == field)
        .ok_or_else(|| invalid(TargetReason::MissingField))?
        .clone();
    let settings = tx
        .get_map("settings")
        .ok_or_else(|| invalid(TargetReason::MissingCollection))?;
    let Some(Out::YMap(collections)) = settings.get(tx, "collections") else {
        return Err(invalid(TargetReason::MissingCollection));
    };
    let Some(Out::YMap(records)) = collections.get(tx, collection) else {
        return Err(invalid(TargetReason::MissingCollection));
    };
    // Tombstoned/unordered records are not public targets, even if a map entry survives.
    let visible = serde_json::to_value(&state.settings.metadata).expect("metadata");
    if !visible["fields"][collection]
        .as_array()
        .is_some_and(|items| items.iter().any(|v| v["id"] == id))
    {
        return Err(invalid(TargetReason::MissingRecord));
    }
    let Some(Out::YMap(record)) = records.get(tx, id) else {
        return Err(invalid(TargetReason::MissingRecord));
    };
    Ok((record, descriptor))
}
pub(in crate::content) enum Resolved {
    Rich(XmlElementRef),
    Text(yrs::TextRef),
    Scalar {
        map: MapRef,
        key: String,
        text: String,
    },
    Annotation(XmlElementRef),
}
pub(in crate::content) fn field<T: ReadTxn>(
    doc: &Doc,
    tx: &T,
    state: &State,
    target: &FieldTarget,
) -> Result<(MapRef, String, Value, String), Error> {
    match target {
        FieldTarget::Option { .. } => Err(invalid(TargetReason::MissingField)),
        FieldTarget::Metadata { field } => {
            let descriptor = state
                .settings
                .metadata
                .field_values()
                .into_iter()
                .find(|f| f.field == *field)
                .ok_or_else(|| invalid(TargetReason::MissingField))?;
            let key = serde_json::to_value(field)
                .expect("field")
                .as_str()
                .unwrap()
                .to_owned();
            let kind = if descriptor.value.is_string() {
                "text"
            } else if descriptor.value.is_boolean() {
                "boolean"
            } else {
                "records"
            };
            Ok((
                tx.get_map("settings")
                    .ok_or_else(|| invalid(TargetReason::MissingField))?,
                key,
                descriptor.value,
                kind.into(),
            ))
        }
        FieldTarget::Record {
            collection,
            id,
            field,
        } => {
            let (map, descriptor) = record(doc, tx, state, collection, id, field)?;
            let value = match map.get(tx, field) {
                Some(Out::YText(text)) => Value::String(text.get_string(tx)),
                Some(Out::Any(value)) => serde_json::to_value(value).expect("value"),
                _ => Value::Null,
            };
            Ok((
                map,
                field.clone(),
                value,
                descriptor["kind"].as_str().unwrap().into(),
            ))
        }
    }
}
pub(in crate::content) fn resolve(
    doc: &Doc,
    state: &State,
    zone: &TextZone,
) -> Result<Resolved, Error> {
    let tx = doc.transact();
    let body = tx
        .get_xml_fragment("default")
        .ok_or_else(|| invalid(TargetReason::MissingNode))?;
    match zone {
        TextZone::Body { path } => rich(element(body, &tx, path)?),
        TextZone::Annotation { path } => {
            let value = element(body, &tx, path)?;
            if value.tag().as_ref() != "footnote" {
                return Err(invalid(TargetReason::NotTextZone));
            }
            Ok(Resolved::Annotation(value))
        }
        TextZone::Metadata { field: key } => {
            let (map, key, value, _) =
                field(doc, &tx, state, &FieldTarget::Metadata { field: *key })?;
            Ok(Resolved::Scalar {
                map,
                key,
                text: value
                    .as_str()
                    .ok_or_else(|| invalid(TargetReason::NotTextZone))?
                    .into(),
            })
        }
        TextZone::Record {
            collection,
            id,
            field,
            path,
        } => {
            let (map, descriptor) = record(doc, &tx, state, collection, id, field)?;
            let value = map
                .get(&tx, field)
                .ok_or_else(|| invalid(TargetReason::MissingField))?;
            if matches!(descriptor["kind"].as_str(), Some("rich" | "rich_inline")) {
                let Out::YXmlFragment(root) = value else {
                    return Err(invalid(TargetReason::NotTextZone));
                };
                return rich(element(root, &tx, path)?);
            }
            if !path.is_empty() {
                return Err(invalid(TargetReason::UnexpectedPath));
            }
            if !matches!(
                descriptor["kind"].as_str(),
                Some("text" | "multiline" | "source")
            ) {
                return Err(invalid(TargetReason::NotTextZone));
            }
            match value {
                Out::YText(value) => Ok(Resolved::Text(value)),
                Out::Any(yrs::Any::String(value)) => Ok(Resolved::Scalar {
                    map,
                    key: field.clone(),
                    text: value.to_string(),
                }),
                _ => Err(invalid(TargetReason::NotTextZone)),
            }
        }
    }
}
fn rich(value: XmlElementRef) -> Result<Resolved, Error> {
    if !matches!(
        value.tag().as_ref(),
        "paragraph"
            | "heading"
            | "figure"
            | "tableCaption"
            | "descriptionTerm"
            | "descriptionDefinition"
            | "quoteAttribution"
            | "postscript"
            | "attachment"
    ) {
        return Err(invalid(TargetReason::NotTextZone));
    }
    Ok(Resolved::Rich(value))
}
impl Resolved {
    pub(in crate::content) fn info<T: ReadTxn>(&self, tx: &T) -> Result<ZoneInfo, Error> {
        let text = match self {
            Self::Rich(element) => super::projection::xml(element.children(tx), tx, 1, &mut 0)?
                .into_iter()
                .map(|n| {
                    if n.kind == "text" {
                        n.text.unwrap_or_default()
                    } else {
                        "\u{fffc}".into()
                    }
                })
                .collect(),
            Self::Text(value) => value.get_string(tx),
            Self::Scalar { text, .. } => text.clone(),
            Self::Annotation(value) => match value.get_attribute(tx, "text") {
                Some(Out::Any(yrs::Any::String(value))) => value.to_string(),
                _ => return Err(invalid(TargetReason::NotTextZone)),
            },
        };
        Ok(ZoneInfo {
            length: text.encode_utf16().count() as u32,
            text,
            supports_styles: matches!(self, Self::Rich(_)),
        })
    }
}
pub(in crate::content) fn byte_at(text: &str, offset: u32) -> Result<usize, Error> {
    let mut count = 0;
    for (index, ch) in text.char_indices() {
        if count == offset {
            return Ok(index);
        }
        count += ch.len_utf16() as u32;
        if count > offset {
            return Err(invalid(TargetReason::SplitSurrogate));
        }
    }
    if count == offset {
        Ok(text.len())
    } else {
        Err(invalid(TargetReason::OutOfBounds))
    }
}
