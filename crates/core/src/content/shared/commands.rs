use super::{
    super::{
        commands::{Command, FieldTarget, Style, Switch, TextTarget},
        contract::{Error, State, TargetReason, UnavailableReason},
    },
    targets::{self, Resolved},
};
use serde_json::Value;
use yrs::types::Attrs;
use yrs::{
    Any, Doc, Map, Out, ReadTxn, Text, Transact, TransactionMut, Xml, XmlElementRef, XmlFragment,
    XmlOut, XmlTextPrelim,
};

fn parameter(name: &str) -> Error {
    Error::InvalidParameters {
        parameter: name.into(),
    }
}
fn unavailable(command: &Command, reason: UnavailableReason) -> Error {
    Error::CommandUnavailable {
        command: command.id().into(),
        reason,
    }
}
fn any(value: &Value) -> Any {
    Any::from_json(&value.to_string()).expect("JSON value")
}
fn style(style: &Style) -> Result<(&'static str, Any), Error> {
    match style {
        Style::Bold => Ok(("bold", any(&serde_json::json!({})))),
        Style::Italic => Ok(("italic", any(&serde_json::json!({})))),
        Style::Link { url } => {
            if !["https://", "http://", "mailto:"]
                .iter()
                .any(|p| url.starts_with(p) && url.len() > p.len())
            {
                return Err(parameter("url"));
            }
            Ok(("link", any(&serde_json::json!({"href":url}))))
        }
    }
}
fn attributes(marks: &[Style]) -> Result<Attrs, Error> {
    let mut result = Attrs::new();
    for mark in marks {
        let (key, value) = style(mark)?;
        if result.insert(key.into(), value).is_some() {
            return Err(parameter("marks"));
        }
    }
    Ok(result)
}
pub(in crate::content) fn validate_styles(marks: &[Style]) -> Result<(), Error> {
    attributes(marks).map(|_| ())
}
fn range(doc: &Doc, state: &State, target: &TextTarget) -> Result<Resolved, Error> {
    if target.from > target.to {
        return Err(targets::invalid(TargetReason::ReversedRange));
    }
    let zone = targets::resolve(doc, state, &target.zone)?;
    let info = zone.info(&doc.transact())?;
    targets::byte_at(&info.text, target.from)?;
    targets::byte_at(&info.text, target.to)?;
    Ok(zone)
}
fn write_text(text: &yrs::TextRef, tx: &mut TransactionMut, before: &str, after: &str) {
    if before == after {
        return;
    }
    let old: Vec<_> = before.chars().collect();
    let new: Vec<_> = after.chars().collect();
    let mut prefix = 0;
    let mut suffix = 0;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    while suffix < old.len() - prefix
        && suffix < new.len() - prefix
        && old[old.len() - suffix - 1] == new[new.len() - suffix - 1]
    {
        suffix += 1;
    }
    let offset = old[..prefix].iter().map(|c| c.len_utf16() as u32).sum();
    let removed = old[prefix..old.len() - suffix]
        .iter()
        .map(|c| c.len_utf16() as u32)
        .sum();
    if removed > 0 {
        text.remove_range(tx, offset, removed);
    }
    let inserted: String = new[prefix..new.len() - suffix].iter().collect();
    if !inserted.is_empty() {
        text.insert(tx, offset, &inserted);
    }
}
fn set_field(
    doc: &Doc,
    state: &State,
    command: &Command,
    target: &FieldTarget,
    value: &Value,
) -> Result<(), Error> {
    let (map, key, before, kind) = targets::field(doc, &doc.transact(), state, target)?;
    let valid = match kind.as_str() {
        "text" | "multiline" | "source" | "language" => value.is_string(),
        "boolean" => value.is_boolean(),
        "tokens" | "references" => value
            .as_array()
            .is_some_and(|v| v.iter().all(Value::is_string)),
        _ => {
            return Err(unavailable(
                command,
                UnavailableReason::UnsupportedFieldKind,
            ));
        }
    };
    if !valid {
        return Err(parameter("value"));
    }
    if kind == "language"
        && serde_json::from_value::<crate::document::DocumentLanguage>(value.clone()).is_err()
    {
        return Err(parameter("value"));
    }
    if *value == before {
        return Ok(());
    }
    let mut tx = doc.transact_mut();
    if let Some(Out::YText(text)) = map.get(&tx, &key) {
        write_text(
            &text,
            &mut tx,
            before.as_str().ok_or_else(|| parameter("value"))?,
            value.as_str().ok_or_else(|| parameter("value"))?,
        );
    } else {
        map.insert(&mut tx, key, any(value));
    }
    Ok(())
}
fn length<T: ReadTxn>(value: &XmlOut, tx: &T) -> u32 {
    match value {
        XmlOut::Text(value) => value.len(tx),
        _ => 1,
    }
}
pub(super) fn replace_rich(
    parent: &XmlElementRef,
    tx: &mut TransactionMut,
    target: &TextTarget,
    inserted: &str,
    attrs: Attrs,
) -> Result<(), Error> {
    let mut offset = 0;
    let children: Vec<_> = parent
        .children(tx)
        .enumerate()
        .map(|(index, child)| {
            let start = offset;
            offset += length(&child, tx);
            (index as u32, child, start, offset)
        })
        .collect();
    // Delete backwards so removing atoms never shifts physical child indices.
    for (index, child, start, end) in children.into_iter().rev() {
        let from = target.from.max(start);
        let to = target.to.min(end);
        if from >= to {
            continue;
        }
        match child {
            XmlOut::Text(value) => value.remove_range(tx, from - start, to - from),
            _ => parent.remove(tx, index),
        }
    }
    if inserted.is_empty() {
        return Ok(());
    }
    let mut offset = 0;
    let mut slot = 0;
    for (index, child) in parent.children(tx).enumerate() {
        let len = length(&child, tx);
        if let XmlOut::Text(value) = &child {
            if target.from <= offset + len {
                value.insert_with_attributes(tx, target.from - offset, inserted, attrs);
                return Ok(());
            }
        } else if target.from == offset {
            slot = index as u32;
            break;
        }
        offset += len;
        slot = index as u32 + 1;
    }
    let value = parent.insert(tx, slot, XmlTextPrelim::new(""));
    value.insert_with_attributes(tx, 0, inserted, attrs);
    Ok(())
}
fn replace(
    doc: &Doc,
    state: &State,
    command: &Command,
    target: &TextTarget,
    inserted: &str,
    marks: &[Style],
) -> Result<(), Error> {
    if inserted.len() > 512_000 {
        return Err(Error::StateLimitExceeded);
    }
    let attrs = attributes(marks)?;
    let zone = range(doc, state, target)?;
    let info = zone.info(&doc.transact())?;
    if info.supports_styles && inserted.contains(['\n', '\r']) {
        return Err(unavailable(
            command,
            UnavailableReason::StructuralEditRequired,
        ));
    }
    if !info.supports_styles && !marks.is_empty() {
        return Err(unavailable(command, UnavailableReason::PlainTextOnly));
    }
    let from = targets::byte_at(&info.text, target.from)?;
    let to = targets::byte_at(&info.text, target.to)?;
    if !info.supports_styles && info.text[from..to] == *inserted {
        return Ok(());
    }
    if let Resolved::Rich(parent) = &zone {
        let tx = doc.transact();
        let mut offset = 0;
        let mut selected = String::new();
        let mut same = true;
        for node in super::projection::xml(parent.children(&tx), &tx, 1, &mut 0)? {
            let len = super::super::operations::inline_len(&node) as u32;
            let a = target.from.max(offset);
            let b = target.to.min(offset + len);
            if a < b {
                if let Some(text) = node.text {
                    selected.push_str(
                        &text[targets::byte_at(&text, a - offset)?
                            ..targets::byte_at(&text, b - offset)?],
                    );
                    let current: Attrs = node
                        .marks
                        .into_iter()
                        .map(|m| {
                            (
                                m.kind.into(),
                                any(m.attrs.as_ref().unwrap_or(&serde_json::json!({}))),
                            )
                        })
                        .collect();
                    same &= current == attrs;
                } else {
                    same = false;
                }
            }
            offset += len;
        }
        if same && selected == inserted {
            return Ok(());
        }
    }
    let mut tx = doc.transact_mut();
    match zone {
        Resolved::Rich(parent) => replace_rich(&parent, &mut tx, target, inserted, attrs)?,
        other => {
            let next = format!("{}{}{}", &info.text[..from], inserted, &info.text[to..]);
            match other {
                Resolved::Text(text) => write_text(&text, &mut tx, &info.text, &next),
                Resolved::Scalar { map, key, .. } => {
                    map.insert(&mut tx, key, next);
                }
                Resolved::Annotation(element) => {
                    element.insert_attribute(&mut tx, "text", next);
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}
fn format(
    doc: &Doc,
    state: &State,
    command: &Command,
    target: &TextTarget,
    change: Option<(&str, Any)>,
) -> Result<(), Error> {
    let zone = range(doc, state, target)?;
    let Resolved::Rich(parent) = zone else {
        return Err(unavailable(command, UnavailableReason::PlainTextOnly));
    };
    if target.from == target.to {
        return Err(unavailable(command, UnavailableReason::EmptySelection));
    }
    let mut tx = doc.transact_mut();
    let mut offset = 0;
    let children: Vec<_> = parent.children(&tx).collect();
    for child in children {
        let start = offset;
        offset += length(&child, &tx);
        let from = target.from.max(start);
        let to = target.to.min(offset);
        if from >= to {
            continue;
        }
        match child {
            XmlOut::Text(text) => {
                let mut attrs = Attrs::new();
                for part in text.diff(&tx, |_| ()) {
                    for key in part
                        .attributes
                        .as_deref()
                        .into_iter()
                        .flat_map(|attrs| attrs.keys())
                    {
                        if change
                            .as_ref()
                            .is_none_or(|(name, _)| *name == super::projection::mark_name(key))
                        {
                            attrs.insert(key.clone(), Any::Null);
                        }
                    }
                }
                if let Some((key, value)) = &change {
                    attrs.insert((*key).into(), value.clone());
                }
                if !attrs.is_empty() {
                    text.format(&mut tx, from - start, to - from, attrs);
                }
            }
            XmlOut::Element(element) => {
                let mut marks: Vec<super::super::Mark> = match element.get_attribute(&tx, "__marks")
                {
                    Some(Out::Any(value)) if value != Any::Null => {
                        serde_json::from_value(serde_json::to_value(value).expect("marks"))
                            .map_err(|_| parameter("marks"))?
                    }
                    _ => vec![],
                };
                let mut before = marks.clone();
                before.sort_by(|a, b| a.kind.cmp(&b.kind));
                marks.retain(|m| change.as_ref().is_some_and(|(name, _)| *name != m.kind));
                if let Some((name, value)) = &change
                    && *value != Any::Null
                {
                    marks.push(super::super::Mark {
                        kind: (*name).into(),
                        attrs: Some(serde_json::to_value(value).expect("mark")),
                    });
                }
                marks.sort_by(|a, b| a.kind.cmp(&b.kind));
                if marks != before {
                    if marks.is_empty() {
                        element.remove_attribute(&mut tx, &"__marks");
                    } else {
                        element.insert_attribute(
                            &mut tx,
                            "__marks",
                            any(&serde_json::to_value(marks).expect("marks")),
                        );
                    }
                }
            }
            _ => return Err(targets::invalid(TargetReason::NotTextZone)),
        }
    }
    Ok(())
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    match command {
        Command::InsertTable { .. }
        | Command::InsertTableRow { .. }
        | Command::RemoveTableRow { .. }
        | Command::InsertTableColumn { .. }
        | Command::RemoveTableColumn { .. }
        | Command::TableHeader { .. } => super::tables::apply(doc, state, command),
        Command::InsertBlock { .. }
        | Command::ReplaceBlock { .. }
        | Command::RemoveBlock { .. }
        | Command::SplitBlock { .. }
        | Command::Paragraph { .. }
        | Command::Heading1 { .. }
        | Command::Heading2 { .. }
        | Command::Heading3 { .. } => super::structure::apply(doc, state, command),
        Command::AddRecord { .. } | Command::RemoveRecord { .. } | Command::MoveRecord { .. } => {
            super::records::apply(doc, state, command)
        }
        Command::SetField { target, value } => set_field(doc, state, command, target, value),
        Command::Insert {
            target,
            text,
            marks,
        } => replace(doc, state, command, target, text, marks),
        Command::Delete { target } => replace(doc, state, command, target, "", &[]),
        Command::Bold { target, value } => format(
            doc,
            state,
            command,
            target,
            Some((
                "bold",
                if *value == Switch::On {
                    any(&serde_json::json!({}))
                } else {
                    Any::Null
                },
            )),
        ),
        Command::Italic { target, value } => format(
            doc,
            state,
            command,
            target,
            Some((
                "italic",
                if *value == Switch::On {
                    any(&serde_json::json!({}))
                } else {
                    Any::Null
                },
            )),
        ),
        Command::Link { target, url } => format(
            doc,
            state,
            command,
            target,
            Some(style(&Style::Link { url: url.clone() })?),
        ),
        Command::RemoveLink { target } => {
            format(doc, state, command, target, Some(("link", Any::Null)))
        }
        Command::Clear { target } => format(doc, state, command, target, None),
    }
}
