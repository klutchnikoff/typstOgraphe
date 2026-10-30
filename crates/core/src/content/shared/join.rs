//! Adjacent prose joins preserve the survivor, never cross a semantic boundary.
use super::{projection, seed, structure, targets};
use crate::content::{
    commands::{TreeRoot, TreeTarget},
    contract::{Error, State, TargetReason, UnavailableReason},
};
use yrs::{Doc, Transact, XmlFragment};

pub(super) fn apply(
    doc: &Doc,
    state: &State,
    left: &TreeTarget,
    right: &TreeTarget,
) -> Result<(), Error> {
    let (Some((&a, parent)), Some((&b, other))) = (left.path.split_last(), right.path.split_last())
    else {
        return Err(targets::invalid(TargetReason::NotBlock));
    };
    if left.root != right.root || parent != other || a.checked_add(1) != Some(b) {
        return Err(Error::InvalidParameters {
            parameter: "right".into(),
        });
    }
    let container = structure::container(
        doc,
        state,
        &TreeTarget {
            root: left.root.clone(),
            path: parent.to_vec(),
        },
    )?;
    let tx = doc.transact();
    let nodes = projection::xml(container.children(&tx), &tx, 1, &mut 0)?;
    let l = nodes
        .get(a)
        .ok_or_else(|| targets::invalid(TargetReason::MissingNode))?;
    let r = nodes
        .get(b)
        .ok_or_else(|| targets::invalid(TargetReason::MissingNode))?;
    if [l, r]
        .iter()
        .any(|n| !matches!(n.kind.as_str(), "paragraph" | "heading"))
    {
        return Err(Error::CommandUnavailable {
            command: "block.join".into(),
            reason: UnavailableReason::UnsupportedTransformation,
        });
    }
    // Removing the mandatory trailing slot would just recreate it with new IDs.
    if left.root == TreeRoot::Body
        && parent.is_empty()
        && b + 1 == nodes.len()
        && r.kind == "paragraph"
        && r.content.is_empty()
    {
        return Ok(());
    }
    let empty_left = l.content.is_empty();
    let suffix = r.content.clone();
    let left_element = targets::element(structure::root(doc, state, &left.root)?, &tx, &left.path)?;
    let index = left_element.len(&tx);
    drop(tx);
    let mut tx = doc.transact_mut();
    if empty_left {
        container.remove(&mut tx, a as u32);
    } else {
        if !suffix.is_empty() {
            let fragment: &yrs::XmlFragmentRef = left_element.as_ref();
            seed::children(fragment.clone(), &mut tx, &suffix, index, 1, &mut 0)?;
        }
        container.remove(&mut tx, b as u32);
    }
    Ok(())
}
