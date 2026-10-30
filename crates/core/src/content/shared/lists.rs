//! Flat list items: preserve the left item/prefix, recreate only the moved suffix.
use super::{commands, projection, seed, structure, targets};
use crate::content::{
    Node,
    commands::{Command, TextTarget, TextZone, TreeRoot, TreeTarget},
    contract::{Error, State, TargetReason, UnavailableReason},
};
use yrs::{Doc, Transact, XmlElementRef, XmlFragment, XmlFragmentRef, XmlOut};

fn fragment(element: &XmlElementRef) -> XmlFragmentRef {
    <XmlElementRef as AsRef<XmlFragmentRef>>::as_ref(element).clone()
}
fn unsupported(id: &str) -> Error {
    Error::CommandUnavailable {
        command: id.into(),
        reason: UnavailableReason::UnsupportedTransformation,
    }
}
struct Item {
    parent: XmlFragmentRef,
    list: XmlElementRef,
    list_index: u32,
    index: u32,
    node: Node,
}
fn item(doc: &Doc, state: &State, target: &TreeTarget) -> Result<Item, Error> {
    let path = &target.path;
    if path.len() < 2 {
        return Err(targets::invalid(TargetReason::NotBlock));
    }
    let (&index, list_path) = path.split_last().unwrap();
    let (&list_index, parent_path) = list_path.split_last().unwrap();
    let parent = structure::container(
        doc,
        state,
        &TreeTarget {
            root: target.root.clone(),
            path: parent_path.to_vec(),
        },
    )?;
    let tx = doc.transact();
    let list = targets::element(structure::root(doc, state, &target.root)?, &tx, list_path)?;
    if !matches!(
        list.tag().as_ref(),
        "bulletList" | "orderedList" | "descriptionList"
    ) {
        return Err(targets::invalid(TargetReason::NotBlockContainer));
    }
    let node = projection::xml(
        std::iter::once(XmlOut::Element(list.clone())),
        &tx,
        1,
        &mut 0,
    )?
    .remove(0);
    if index >= node.content.len() {
        return Err(targets::invalid(TargetReason::MissingNode));
    }
    Ok(Item {
        parent,
        list,
        list_index: list_index as u32,
        index: index as u32,
        node,
    })
}
fn suffix(nodes: &[Node], offset: u32) -> Result<Vec<Node>, Error> {
    let mut remaining = offset;
    let mut result = Vec::new();
    for node in nodes {
        let length = node.inline_len() as u32;
        if remaining >= length {
            remaining -= length;
            continue;
        }
        let mut node = node.clone();
        if let Some(text) = &mut node.text {
            *text = text[targets::byte_at(text, remaining)?..].to_owned();
        }
        result.push(node);
        remaining = 0;
    }
    Ok(result)
}
fn split(doc: &Doc, state: &State, target: &TextTarget) -> Result<(), Error> {
    let (root, path) = match &target.zone {
        TextZone::Body { path } => (TreeRoot::Body, path),
        _ => return Err(unsupported("list.item.split")),
    };
    let Some((&part, item_path)) = path.split_last() else {
        return Err(unsupported("list.item.split"));
    };
    let ctx = item(
        doc,
        state,
        &TreeTarget {
            root,
            path: item_path.to_vec(),
        },
    )?;
    let before = &ctx.node.content[ctx.index as usize];
    let description = before.kind == "descriptionItem";
    if (description && part != 1) || part >= before.content.len() {
        return Err(unsupported("list.item.split"));
    }
    let zone = commands::range(doc, state, target)?;
    let info = zone.info(&doc.transact())?;
    let targets::Resolved::Rich(element) = zone else {
        return Err(unsupported("list.item.split"));
    };
    let mut next = before.clone();
    let mut right = before.content[part].clone();
    right.content = suffix(&right.content, target.to)?;
    if description {
        next.content[0].content.clear();
        next.content[1] = right;
    } else {
        next.content = vec![right];
        next.content.extend_from_slice(&before.content[part + 1..]);
    }
    let mut tx = doc.transact_mut();
    commands::replace_rich(
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
    if !description {
        let current = ctx
            .list
            .get(&tx, ctx.index)
            .unwrap()
            .into_xml_element()
            .unwrap();
        if current.len(&tx) > part as u32 + 1 {
            let count = current.len(&tx) - part as u32 - 1;
            current.remove_range(&mut tx, part as u32 + 1, count);
        }
    }
    seed::children(
        fragment(&ctx.list),
        &mut tx,
        &[next],
        ctx.index + 1,
        1,
        &mut 0,
    )
}
fn join(doc: &Doc, state: &State, left: &TreeTarget, right: &TreeTarget) -> Result<(), Error> {
    let Some((&index, parent)) = left.path.split_last() else {
        return Err(unsupported("list.item.join"));
    };
    if left.root != right.root
        || right
            .path
            .split_last()
            .is_none_or(|(b, p)| index.checked_add(1) != Some(*b) || p != parent)
    {
        return Err(unsupported("list.item.join"));
    }
    let ctx = item(doc, state, left)?;
    if ctx.node.kind == "descriptionList" {
        return Err(unsupported("list.item.join"));
    }
    let r = ctx
        .node
        .content
        .get(index + 1)
        .ok_or_else(|| targets::invalid(TargetReason::MissingNode))?;
    let tx = doc.transact();
    let element = ctx
        .list
        .get(&tx, ctx.index)
        .unwrap()
        .into_xml_element()
        .unwrap();
    let last = element
        .get(&tx, element.len(&tx) - 1)
        .unwrap()
        .into_xml_element()
        .unwrap();
    let at = last.len(&tx);
    let count = element.len(&tx);
    drop(tx);
    let mut tx = doc.transact_mut();
    if r.content.iter().all(|n| n.content.is_empty()) {
        ctx.list.remove(&mut tx, ctx.index + 1);
        return Ok(());
    }
    seed::children(
        fragment(&last),
        &mut tx,
        &r.content[0].content,
        at,
        1,
        &mut 0,
    )?;
    seed::children(
        fragment(&element),
        &mut tx,
        &r.content[1..],
        count,
        1,
        &mut 0,
    )?;
    ctx.list.remove(&mut tx, ctx.index + 1);
    Ok(())
}
fn exit(doc: &Doc, state: &State, target: &TreeTarget) -> Result<(), Error> {
    let ctx = item(doc, state, target)?;
    if ctx.node.content[ctx.index as usize]
        .content
        .iter()
        .any(|n| !n.content.is_empty())
    {
        return Err(unsupported("list.item.exit"));
    }
    let mut tx = doc.transact_mut();
    let after = if ctx.node.content.len() == 1 {
        ctx.parent.remove(&mut tx, ctx.list_index);
        ctx.list_index
    } else {
        ctx.list.remove(&mut tx, ctx.index);
        ctx.list_index + 1
    };
    let next = ctx.parent.get(&tx, after);
    let nodes = projection::xml(next.into_iter(), &tx, 1, &mut 0)?;
    if !nodes
        .first()
        .is_some_and(|n| n.kind == "paragraph" && n.content.is_empty())
    {
        let paragraph = Node {
            kind: "paragraph".into(),
            attrs: None,
            text: None,
            marks: vec![],
            content: vec![],
        };
        seed::children(ctx.parent, &mut tx, &[paragraph], after, 1, &mut 0)?;
    }
    Ok(())
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    match command {
        Command::SplitListItem { target } => split(doc, state, target),
        Command::JoinListItems { left, right } => join(doc, state, left, right),
        Command::ExitListItem { target } => exit(doc, state, target),
        _ => unreachable!(),
    }
}
