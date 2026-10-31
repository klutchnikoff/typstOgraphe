//! Documentary whitespace policy, applied to a private typed-command candidate.
//! Mirrors the immediate editor repair carried by validated typing deltas.
use super::{seed, targets};
use crate::content::{
    Node,
    contract::{Error, State},
};
use yrs::{
    Any, Doc, Map, Out, ReadTxn, Text, Transact, XmlElementRef, XmlFragment, XmlFragmentRef, XmlOut,
};
fn fragment(value: &XmlElementRef) -> XmlFragmentRef {
    <XmlElementRef as AsRef<XmlFragmentRef>>::as_ref(value).clone()
}
fn is_break(value: &XmlOut) -> bool {
    matches!(value, XmlOut::Element(e) if matches!(e.tag().as_ref(), "softBreak" | "hardBreak"))
}
fn text_empty<T: ReadTxn>(value: &XmlOut, tx: &T) -> bool {
    matches!(value, XmlOut::Text(t) if t.len(tx) == 0)
}
fn empty<T: ReadTxn>(value: &XmlOut, tx: &T, heading: bool) -> bool {
    matches!(value, XmlOut::Element(e) if (e.tag().as_ref() == "paragraph" || (heading && e.tag().as_ref() == "heading")) && e.children(tx).all(|c| text_empty(&c, tx) || is_break(&c)))
}
fn ending_break<T: ReadTxn>(value: &XmlOut, tx: &T) -> bool {
    matches!(value, XmlOut::Element(e) if e.tag().as_ref() == "paragraph" && e.children(tx).filter(|c| !text_empty(c, tx)).last().is_some_and(|c| is_break(&c)))
}
/// Preserve the first ordinary/NBSP space in each run, deleting only excess
/// characters. A mark boundary is not a word boundary; inline atoms are.
fn spaces(parent: &XmlFragmentRef, tx: &mut yrs::TransactionMut) {
    let mut previous = false;
    let children: Vec<_> = parent.children(tx).collect();
    for child in children {
        if let XmlOut::Text(text) = child {
            let mut offset = 0;
            let mut remove = Vec::new();
            let plain = text
                .diff(tx, |_| ())
                .into_iter()
                .map(|part| match part.insert {
                    Out::Any(Any::String(value)) => value.to_string(),
                    _ => "\u{fffc}".into(),
                })
                .collect::<String>();
            for ch in plain.chars() {
                let current = matches!(ch, ' ' | '\u{a0}');
                if current && previous {
                    remove.push(offset);
                }
                previous = current;
                offset += ch.len_utf16() as u32;
            }
            for offset in remove.into_iter().rev() {
                text.remove_range(tx, offset, 1);
            }
        } else {
            previous = false;
            if let XmlOut::Element(element) = child {
                spaces(&fragment(&element), tx);
            }
        }
    }
}
fn visit(parent: &XmlFragmentRef, tx: &mut yrs::TransactionMut, flow: bool) {
    let children: Vec<_> = parent.children(tx).collect();
    for (index, child) in children.iter().enumerate().rev() {
        let XmlOut::Element(element) = child else {
            continue;
        };
        if flow
            && empty(child, tx, false)
            && children
                .get(index + 1)
                .is_some_and(|next| empty(next, tx, false))
        {
            parent.remove(tx, index as u32);
            continue;
        }
        if element.tag().as_ref() == "paragraph" {
            let parts: Vec<_> = element.children(tx).collect();
            let all_empty = empty(child, tx, false);
            let mut previous = false;
            let mut remove = Vec::new();
            for (index, part) in parts.iter().enumerate() {
                if text_empty(part, tx) {
                    continue;
                }
                let current = is_break(part);
                if current && (all_empty || previous) {
                    remove.push(index as u32);
                }
                previous = current;
            }
            for index in remove.into_iter().rev() {
                element.remove(tx, index);
            }
        } else {
            visit(
                &fragment(element),
                tx,
                matches!(
                    element.tag().as_ref(),
                    "quote" | "tableCell" | "tableHeader"
                ),
            );
        }
    }
}
fn normalize(doc: &Doc, root: XmlFragmentRef, trailing: bool) -> Result<(), Error> {
    let mut tx = doc.transact_mut();
    spaces(&root, &mut tx);
    if root.children(&tx).any(|c| !empty(&c, &tx, true)) {
        while root.get(&tx, 0).is_some_and(|c| empty(&c, &tx, true)) {
            root.remove(&mut tx, 0);
        }
    }
    if let Some(XmlOut::Element(first)) = root.get(&tx, 0)
        && matches!(first.tag().as_ref(), "paragraph" | "heading")
    {
        let mut remove = Vec::new();
        for (index, child) in first.children(&tx).enumerate() {
            if text_empty(&child, &tx) {
                continue;
            }
            if !is_break(&child) {
                break;
            }
            remove.push(index as u32);
        }
        for index in remove.into_iter().rev() {
            first.remove(&mut tx, index);
        }
    }
    visit(&root, &mut tx, true);
    let len = root.len(&tx);
    if len >= 2
        && root
            .get(&tx, len - 1)
            .is_some_and(|c| empty(&c, &tx, false))
        && root
            .get(&tx, len - 2)
            .is_some_and(|c| ending_break(&c, &tx))
    {
        root.remove(&mut tx, len - 1);
    }
    if trailing
        && root
            .get(&tx, root.len(&tx).saturating_sub(1))
            .is_none_or(|c| !empty(&c, &tx, false) && !ending_break(&c, &tx))
    {
        let paragraph: Node =
            serde_json::from_value(serde_json::json!({"type":"paragraph"})).unwrap();
        let index = root.len(&tx);
        seed::children(root, &mut tx, &[paragraph], index, 1, &mut 0)?;
    }
    Ok(())
}
pub(super) fn apply(doc: &Doc, state: &State) -> Result<(), Error> {
    let root = doc.transact().get_xml_fragment("default").expect("body");
    normalize(doc, root, true)?;
    if let Some(schema) = state
        .settings
        .metadata
        .document_type()
        .structured_metadata()
    {
        let data = serde_json::to_value(&state.settings.metadata).expect("metadata");
        for definition in schema["collections"].as_array().into_iter().flatten() {
            let name = definition["parameter"].as_str().unwrap();
            for field in definition["fields"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|f| f["kind"] == "rich" || f["kind"] == "rich_inline")
            {
                let key = field["key"].as_str().unwrap();
                for record in data["fields"][name].as_array().into_iter().flatten() {
                    let (map, _) = targets::record(
                        doc,
                        &doc.transact(),
                        state,
                        name,
                        record["id"].as_str().unwrap(),
                        key,
                    )?;
                    let value = map.get(&doc.transact(), key);
                    if let Some(Out::YXmlFragment(root)) = value {
                        if field["kind"] == "rich" {
                            normalize(doc, root, false)?;
                        } else {
                            spaces(&root, &mut doc.transact_mut());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
