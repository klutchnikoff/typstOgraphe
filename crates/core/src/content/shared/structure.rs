//! Explicit subtree edits. XML tags are immutable: replacement renews only that subtree.
use super::{projection, seed, targets};
use crate::content::{
    Node,
    commands::{Command, TextTarget, TextZone, TreeRoot, TreeTarget},
    contract::{Error, State, TargetReason, UnavailableReason},
};
use yrs::{Doc, Map, Out, ReadTxn, Transact, Xml, XmlElementRef, XmlFragment, XmlFragmentRef};

fn fragment(value: &XmlElementRef) -> XmlFragmentRef {
    <XmlElementRef as AsRef<XmlFragmentRef>>::as_ref(value).clone()
}
fn invalid(reason: &str) -> Error {
    Error::InvalidDocument {
        reason: reason.into(),
    }
}
fn paragraph() -> Node {
    serde_json::from_value(serde_json::json!({"type":"paragraph"})).unwrap()
}
fn block(kind: &str) -> bool {
    matches!(
        kind,
        "paragraph"
            | "heading"
            | "bulletList"
            | "orderedList"
            | "descriptionList"
            | "quote"
            | "figure"
            | "table"
            | "postscript"
            | "attachment"
    )
}
/// Grammar checks complement source generation: it alone permits some empty containers
/// that ProseMirror would silently repair on import.
fn shape(node: &Node, depth: usize, count: &mut usize) -> Result<(), Error> {
    *count += 1;
    if depth > 16 || *count > 20_000 || node.text.as_ref().is_some_and(|s| s.len() > 512_000) {
        return Err(Error::StateLimitExceeded);
    }
    let kinds: Vec<_> = node.content.iter().map(|n| n.kind.as_str()).collect();
    let valid = match node.kind.as_str() {
        "doc" => !kinds.is_empty() && kinds.iter().all(|k| block(k)),
        "paragraph"
        | "heading"
        | "figure"
        | "descriptionTerm"
        | "descriptionDefinition"
        | "quoteAttribution"
        | "postscript"
        | "attachment" => kinds.iter().all(|k| {
            matches!(
                *k,
                "text" | "softBreak" | "hardBreak" | "footnote" | "citation"
            )
        }),
        "text" => kinds.is_empty() && node.text.as_ref().is_some_and(|t| !t.is_empty()),
        "softBreak" | "hardBreak" | "footnote" | "citation" => kinds.is_empty(),
        "bulletList" | "orderedList" => !kinds.is_empty() && kinds.iter().all(|k| *k == "listItem"),
        "listItem" | "tableCell" | "tableHeader" => {
            !kinds.is_empty() && kinds.iter().all(|k| *k == "paragraph")
        }
        "descriptionList" => !kinds.is_empty() && kinds.iter().all(|k| *k == "descriptionItem"),
        "descriptionItem" => kinds == ["descriptionTerm", "descriptionDefinition"],
        "quote" => {
            kinds.len() >= 2
                && kinds.last() == Some(&"quoteAttribution")
                && kinds[..kinds.len() - 1].iter().all(|k| *k == "paragraph")
        }
        "table" => !kinds.is_empty() && kinds.iter().all(|k| *k == "tableRow"),
        "tableRow" => {
            !kinds.is_empty()
                && kinds
                    .iter()
                    .all(|k| matches!(*k, "tableCell" | "tableHeader"))
        }
        _ => false,
    };
    if !valid
        || (node.kind != "text" && node.text.is_some())
        || (!matches!(
            node.kind.as_str(),
            "text" | "softBreak" | "hardBreak" | "footnote" | "citation"
        ) && !node.marks.is_empty())
    {
        return Err(invalid("invalid_block_structure"));
    }
    for child in &node.content {
        shape(child, depth + 1, count)?;
    }
    Ok(())
}
pub(super) fn validate(state: &State) -> Result<(), Error> {
    shape(&state.document, 0, &mut 0)?;
    if let Some(schema) = state
        .settings
        .metadata
        .document_type()
        .structured_metadata()
    {
        let data = serde_json::to_value(&state.settings.metadata).expect("metadata");
        for definition in schema["collections"].as_array().into_iter().flatten() {
            let name = definition["parameter"].as_str().unwrap();
            for field in definition["fields"].as_array().into_iter().flatten() {
                if matches!(field["kind"].as_str(), Some("rich" | "rich_inline")) {
                    let key = field["key"].as_str().unwrap();
                    for record in data["fields"][name].as_array().into_iter().flatten() {
                        let node: Node = serde_json::from_value(record[key].clone())
                            .map_err(|_| invalid("invalid_rich_record"))?;
                        shape(&node, 0, &mut 0)?;
                    }
                }
            }
        }
    }
    Ok(())
}
fn root(doc: &Doc, state: &State, target: &TreeRoot) -> Result<XmlFragmentRef, Error> {
    let tx = doc.transact();
    match target {
        TreeRoot::Body => tx
            .get_xml_fragment("default")
            .ok_or_else(|| targets::invalid(TargetReason::MissingNode)),
        TreeRoot::Record {
            collection,
            id,
            field,
        } => {
            let (map, descriptor) = targets::record(doc, &tx, state, collection, id, field)?;
            if !matches!(descriptor["kind"].as_str(), Some("rich" | "rich_inline")) {
                return Err(targets::invalid(TargetReason::NotBlockContainer));
            }
            match map.get(&tx, field) {
                Some(Out::YXmlFragment(value)) => Ok(value),
                _ => Err(targets::invalid(TargetReason::NotBlockContainer)),
            }
        }
    }
}
fn container(doc: &Doc, state: &State, target: &TreeTarget) -> Result<XmlFragmentRef, Error> {
    let root = root(doc, state, &target.root)?;
    if target.path.is_empty() {
        return Ok(root);
    }
    let tx = doc.transact();
    let value = targets::element(root, &tx, &target.path)?;
    if !matches!(
        value.tag().as_ref(),
        "listItem"
            | "bulletList"
            | "orderedList"
            | "descriptionList"
            | "quote"
            | "tableCell"
            | "tableHeader"
    ) {
        return Err(targets::invalid(TargetReason::NotBlockContainer));
    }
    Ok(fragment(&value))
}
fn location(
    doc: &Doc,
    state: &State,
    target: &TreeTarget,
) -> Result<(XmlFragmentRef, u32, Node), Error> {
    let (index, parent) = target
        .path
        .split_last()
        .ok_or_else(|| targets::invalid(TargetReason::NotBlock))?;
    let root = container(
        doc,
        state,
        &TreeTarget {
            root: target.root.clone(),
            path: parent.to_vec(),
        },
    )?;
    let tx = doc.transact();
    let nodes = projection::xml(root.children(&tx), &tx, 1, &mut 0)?;
    let node = nodes
        .get(*index)
        .ok_or_else(|| targets::invalid(TargetReason::MissingNode))?
        .clone();
    // Cells, rows and inline zones are boundaries, never removable as ordinary blocks.
    if !block(&node.kind) && !matches!(node.kind.as_str(), "listItem" | "descriptionItem") {
        return Err(targets::invalid(TargetReason::NotBlock));
    }
    Ok((root, *index as u32, node))
}
fn repair(doc: &Doc, parent: &XmlFragmentRef, top: bool) -> Result<(), Error> {
    let mut tx = doc.transact_mut();
    let nodes = projection::xml(parent.children(&tx), &tx, 1, &mut 0)?;
    if (top
        && nodes
            .last()
            .is_none_or(|n| !matches!(n.kind.as_str(), "paragraph" | "heading")))
        || nodes.is_empty()
    {
        // Empty paragraphs keep editable containers and a final insertion point available.
        seed::children(
            parent.clone(),
            &mut tx,
            &[paragraph()],
            nodes.len() as u32,
            1,
            &mut 0,
        )?;
    }
    Ok(())
}
fn top(target: &TreeTarget) -> bool {
    target.path.is_empty()
}
fn new_subtree(node: &Node) -> Result<(), Error> {
    fn public(node: &Node) -> bool {
        !node
            .attrs
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .is_some_and(|a| a.keys().any(|key| key.starts_with('_')))
            && node.content.iter().all(public)
    }
    shape(node, 0, &mut 0)?;
    if !public(node) {
        return Err(Error::InvalidParameters {
            parameter: "node".into(),
        });
    }
    Ok(())
}
fn insert(
    doc: &Doc,
    state: &State,
    parent: &TreeTarget,
    index: u32,
    node: &Node,
) -> Result<(), Error> {
    new_subtree(node)?;
    let root = container(doc, state, parent)?;
    let len = root.len(&doc.transact());
    if index > len {
        return Err(targets::invalid(TargetReason::OutOfBounds));
    }
    seed::children(
        root.clone(),
        &mut doc.transact_mut(),
        std::slice::from_ref(node),
        index,
        1,
        &mut 0,
    )?;
    repair(doc, &root, top(parent))
}
fn replace(
    doc: &Doc,
    state: &State,
    target: &TreeTarget,
    replacement: Option<&Node>,
) -> Result<(), Error> {
    if let Some(node) = replacement {
        new_subtree(node)?;
    }
    let (parent, index, before) = location(doc, state, target)?;
    if replacement
        .is_some_and(|n| super::normalization::node(n) == super::normalization::node(&before))
    {
        return Ok(());
    }
    {
        let mut tx = doc.transact_mut();
        parent.remove(&mut tx, index);
        if let Some(node) = replacement {
            seed::children(
                parent.clone(),
                &mut tx,
                std::slice::from_ref(node),
                index,
                1,
                &mut 0,
            )?;
        }
    }
    // Repair only paragraph containers, not list containers (empty lists are rejected).
    let parent_target = TreeTarget {
        root: target.root.clone(),
        path: target.path[..target.path.len() - 1].to_vec(),
    };
    let is_text_container = if parent_target.path.is_empty() {
        true
    } else {
        let root = root(doc, state, &target.root)?;
        let tx = doc.transact();
        matches!(
            targets::element(root, &tx, &parent_target.path)?
                .tag()
                .as_ref(),
            "listItem" | "tableCell" | "tableHeader"
        )
    };
    if is_text_container {
        repair(doc, &parent, top(&parent_target))?;
    }
    Ok(())
}
fn convert(
    doc: &Doc,
    state: &State,
    command: &Command,
    target: &TreeTarget,
    level: Option<u8>,
) -> Result<(), Error> {
    let (parent, index, mut node) = location(doc, state, target)?;
    if !matches!(node.kind.as_str(), "paragraph" | "heading") {
        return Err(Error::CommandUnavailable {
            command: command.id().into(),
            reason: UnavailableReason::UnsupportedTransformation,
        });
    }
    // A level change does not change the immutable XML tag or its content.
    if node.kind == "heading"
        && let Some(level) = level
    {
        let current = node
            .attrs
            .as_ref()
            .and_then(|a| a.get("level"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        if current != u64::from(level) {
            let mut tx = doc.transact_mut();
            let element = parent
                .get(&tx, index)
                .and_then(|v| v.into_xml_element())
                .expect("heading element");
            element.insert_attribute(&mut tx, "level", u32::from(level));
        }
        return Ok(());
    }
    node.kind = if level.is_some() {
        "heading"
    } else {
        "paragraph"
    }
    .into();
    node.attrs = level.map(|n| serde_json::json!({"level":n}));
    replace(doc, state, target, Some(&node))
}
fn split(doc: &Doc, state: &State, target: &TextTarget) -> Result<(), Error> {
    if target.from != target.to {
        return Err(Error::InvalidParameters {
            parameter: "target".into(),
        });
    }
    let tree = match &target.zone {
        TextZone::Body { path } => TreeTarget {
            root: TreeRoot::Body,
            path: path.clone(),
        },
        TextZone::Record {
            collection,
            id,
            field,
            path,
        } => TreeTarget {
            root: TreeRoot::Record {
                collection: collection.clone(),
                id: id.clone(),
                field: field.clone(),
            },
            path: path.clone(),
        },
        _ => return Err(targets::invalid(TargetReason::NotBlock)),
    };
    let (parent, index, node) = location(doc, state, &tree)?;
    if !matches!(node.kind.as_str(), "paragraph" | "heading") {
        return Err(targets::invalid(TargetReason::NotTextZone));
    }
    let resolved = targets::resolve(doc, state, &target.zone)?;
    let info = resolved.info(&doc.transact())?;
    targets::byte_at(&info.text, target.from)?;
    let targets::Resolved::Rich(element) = resolved else {
        unreachable!()
    };
    let mut remaining = target.from;
    let mut suffix = Vec::new();
    for mut inline in node.content {
        let len = super::super::operations::inline_len(&inline) as u32;
        if remaining >= len {
            remaining -= len;
            continue;
        }
        if let Some(text) = &mut inline.text {
            *text = text[targets::byte_at(text, remaining)?..].to_owned();
        }
        suffix.push(inline);
        remaining = 0;
    }
    let mut after = paragraph();
    after.content = suffix;
    let mut tx = doc.transact_mut();
    super::commands::replace_rich(
        &element,
        &mut tx,
        &TextTarget {
            zone: target.zone.clone(),
            from: target.from,
            to: info.length,
        },
        "",
        Default::default(),
    )?;
    seed::children(parent, &mut tx, &[after], index + 1, 1, &mut 0)
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    match command {
        Command::InsertBlock {
            parent,
            index,
            node,
        } => insert(doc, state, parent, *index, node),
        Command::ReplaceBlock { target, node } => replace(doc, state, target, Some(node)),
        Command::RemoveBlock { target } => replace(doc, state, target, None),
        Command::SplitBlock { target } => split(doc, state, target),
        Command::Paragraph { target } => convert(doc, state, command, target, None),
        Command::Heading1 { target } => convert(doc, state, command, target, Some(1)),
        Command::Heading2 { target } => convert(doc, state, command, target, Some(2)),
        Command::Heading3 { target } => convert(doc, state, command, target, Some(3)),
        _ => unreachable!(),
    }
}
