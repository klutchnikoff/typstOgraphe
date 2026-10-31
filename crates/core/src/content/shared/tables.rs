//! Fine edits preserve the grid, caption, rows and cells outside the edited boundary.
use super::{seed, targets};
use crate::content::{
    commands::{Command, HeaderAxis, TreeRoot, TreeTarget},
    contract::{Error, State, TargetReason},
};
use yrs::{Doc, ReadTxn, Transact, Xml, XmlElementRef, XmlFragment, XmlFragmentRef, XmlOut};
fn invalid() -> Error {
    targets::invalid(TargetReason::OutOfBounds)
}
fn fragment(value: &XmlElementRef) -> XmlFragmentRef {
    <XmlElementRef as AsRef<XmlFragmentRef>>::as_ref(value).clone()
}
fn grid(doc: &Doc, target: &TreeTarget) -> Result<XmlElementRef, Error> {
    if target.root != TreeRoot::Body {
        return Err(targets::invalid(TargetReason::NotBlock));
    }
    let tx = doc.transact();
    let root = tx.get_xml_fragment("default").expect("body");
    let figure = targets::element(root, &tx, &target.path)?;
    if figure.tag().as_ref() != "tableFigure" {
        return Err(targets::invalid(TargetReason::NotBlock));
    }
    figure
        .get(&tx, 0)
        .and_then(|v| v.into_xml_element())
        .filter(|v| v.tag().as_ref() == "table")
        .ok_or_else(invalid)
}
/// Change only cells whose semantic role actually changes. Other Yrs objects,
/// rich content, caption and row/column identities remain untouched.
fn roles(
    grid: &XmlElementRef,
    tx: &mut yrs::TransactionMut,
    columns: bool,
    rows: bool,
) -> Result<(), Error> {
    let projected = super::projection::xml(grid.children(tx), tx, 1, &mut 0)?;
    for (r, row) in projected.iter().enumerate() {
        let shared_row = grid
            .get(tx, r as u32)
            .and_then(|v| v.into_xml_element())
            .ok_or_else(invalid)?;
        for (c, node) in row.content.iter().enumerate() {
            let scope = crate::content::tables::cell_scope(columns, rows, r, c);
            if crate::content::tables::scope(node) == scope {
                continue;
            }
            let kind = if scope.is_some() {
                "tableHeader"
            } else {
                "tableCell"
            };
            if node.kind == kind {
                let cell = shared_row
                    .get(tx, c as u32)
                    .and_then(|v| v.into_xml_element())
                    .ok_or_else(invalid)?;
                if let Some(scope) = scope.filter(|s| *s != "column") {
                    cell.insert_attribute(tx, "scope", scope);
                } else {
                    cell.remove_attribute(tx, &"scope");
                }
            } else {
                let mut changed = node.clone();
                changed.kind = kind.into();
                let attrs = changed
                    .attrs
                    .get_or_insert_with(|| serde_json::json!({}))
                    .as_object_mut()
                    .ok_or_else(invalid)?;
                attrs.remove("scope");
                if let Some(scope) = scope.filter(|s| *s != "column") {
                    attrs.insert("scope".into(), serde_json::json!(scope));
                }
                shared_row.remove(tx, c as u32);
                seed::children(fragment(&shared_row), tx, &[changed], c as u32, 1, &mut 0)?;
            }
        }
    }
    Ok(())
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    if let Command::InsertTable {
        parent,
        index,
        rows,
        columns,
        header,
        row_headers,
    } = command
    {
        if parent.root != TreeRoot::Body || !parent.path.is_empty() {
            return Err(targets::invalid(TargetReason::NotBlockContainer));
        }
        return super::structure::apply(
            doc,
            state,
            &Command::InsertBlock {
                parent: parent.clone(),
                index: *index,
                node: crate::content::tables::create(*rows, *columns, *header, *row_headers)?,
            },
        );
    }
    let (target, index) = match command {
        Command::InsertTableRow { target, index }
        | Command::RemoveTableRow { target, index }
        | Command::InsertTableColumn { target, index }
        | Command::RemoveTableColumn { target, index } => (target, *index),
        Command::TableHeader { target, .. } => (target, 0),
        _ => unreachable!(),
    };
    let grid = grid(doc, target)?;
    let tx = doc.transact();
    let rows: Vec<_> = grid
        .children(&tx)
        .map(|v| v.into_xml_element().ok_or_else(invalid))
        .collect::<Result<_, _>>()?;
    let width = rows.first().map_or(0, |r| r.len(&tx));
    let projected = super::projection::xml(
        std::iter::once(XmlOut::Element(grid.clone())),
        &tx,
        0,
        &mut 0,
    )?;
    let (mut header, mut row_headers) = crate::content::tables::headers(&projected[0]);
    drop(tx);
    let mut tx = doc.transact_mut();
    match command {
        Command::InsertTableRow { .. } => {
            if index > rows.len() as u32 || rows.len() >= 100 {
                return Err(invalid());
            }
            let created = crate::content::tables::create(1, width, false, false)?;
            let row = &created.content[0].content[0];
            seed::children(
                fragment(&grid),
                &mut tx,
                std::slice::from_ref(row),
                index,
                1,
                &mut 0,
            )?;
        }
        Command::RemoveTableRow { .. } => {
            if index >= rows.len() as u32 || rows.len() <= 1 {
                return Err(invalid());
            }
            grid.remove(&mut tx, index);
        }
        Command::InsertTableColumn { .. } => {
            if index > width || width >= 12 {
                return Err(invalid());
            }
            for row in &rows {
                seed::children(
                    fragment(row),
                    &mut tx,
                    &[crate::content::tables::cell(false)],
                    index,
                    1,
                    &mut 0,
                )?;
            }
        }
        Command::RemoveTableColumn { .. } => {
            if index >= width || width <= 1 {
                return Err(invalid());
            }
            for row in &rows {
                row.remove(&mut tx, index);
            }
        }
        Command::TableHeader { enabled, axis, .. } => match axis {
            HeaderAxis::Columns => header = *enabled,
            HeaderAxis::Rows => row_headers = *enabled,
        },
        _ => unreachable!(),
    }
    roles(&grid, &mut tx, header, row_headers)?;
    Ok(())
}
