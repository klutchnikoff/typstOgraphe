//! Fine edits preserve the grid, caption, rows and cells outside the edited boundary.
use super::{seed, targets};
use crate::content::{
    commands::{Command, TreeRoot, TreeTarget},
    contract::{Error, State, TargetReason},
};
use yrs::{Doc, ReadTxn, Transact, XmlElementRef, XmlFragment, XmlFragmentRef};
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
fn row_kind(row: &XmlElementRef, tx: &mut yrs::TransactionMut, header: bool) -> Result<(), Error> {
    let width = row.len(tx);
    let mut cells = super::projection::xml(row.children(tx), tx, 1, &mut 0)?;
    for cell in &mut cells {
        cell.kind = if header { "tableHeader" } else { "tableCell" }.into();
    }
    row.remove_range(tx, 0, width);
    seed::children(fragment(row), tx, &cells, 0, 1, &mut 0)
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    if let Command::InsertTable {
        parent,
        index,
        rows,
        columns,
        header,
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
                node: crate::content::tables::create(*rows, *columns, *header)?,
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
    let header = rows.first().is_some_and(|r| {
        r.children(&tx).all(|c| {
            c.into_xml_element()
                .is_some_and(|c| c.tag().as_ref() == "tableHeader")
        })
    });
    drop(tx);
    let mut tx = doc.transact_mut();
    match command {
        Command::InsertTableRow { .. } => {
            if index > rows.len() as u32 || rows.len() >= 100 {
                return Err(invalid());
            }
            if header && index == 0 {
                row_kind(&rows[0], &mut tx, false)?;
            }
            let created = crate::content::tables::create(1, width, header && index == 0)?;
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
            for (r, row) in rows.iter().enumerate() {
                seed::children(
                    fragment(row),
                    &mut tx,
                    &[crate::content::tables::cell(header && r == 0)],
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
        Command::TableHeader { enabled, .. } => {
            if header == *enabled {
                return Ok(());
            }
            let row = rows.first().ok_or_else(invalid)?;
            row_kind(row, &mut tx, *enabled)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
