//! Documentary table construction, reused by every native insertion adapter.
use super::{Node, contract::Error};
fn node(kind: &str, content: Vec<Node>) -> Node {
    Node {
        kind: kind.into(),
        text: None,
        attrs: None,
        content,
        marks: vec![],
    }
}
pub(crate) fn cell(header: bool) -> Node {
    node(
        if header { "tableHeader" } else { "tableCell" },
        vec![node("paragraph", vec![])],
    )
}
pub fn create(rows: u32, columns: u32, header: bool, row_headers: bool) -> Result<Node, Error> {
    if !(1..=100).contains(&rows) || !(1..=12).contains(&columns) {
        return Err(Error::InvalidParameters {
            parameter: "dimensions".into(),
        });
    }
    let grid = node(
        "table",
        (0..rows)
            .map(|r| {
                node(
                    "tableRow",
                    (0..columns)
                        .map(|c| {
                            scoped_cell(cell_scope(header, row_headers, r as usize, c as usize))
                        })
                        .collect(),
                )
            })
            .collect(),
    );
    Ok(node(
        "tableFigure",
        vec![grid, node("tableCaption", vec![])],
    ))
}

/// Header axes are unambiguous even for a one-cell grid: its scope can be
/// column, row or both. No redundant table-level flags can drift from its cells.
pub(crate) fn scope(cell: &Node) -> Option<&str> {
    (cell.kind == "tableHeader").then(|| {
        cell.attrs
            .as_ref()
            .and_then(|a| a.get("scope"))
            .and_then(|v| v.as_str())
            .unwrap_or("column")
    })
}
pub(crate) fn headers(grid: &Node) -> (bool, bool) {
    let columns = grid.content.first().is_some_and(|r| {
        !r.content.is_empty()
            && r.content
                .iter()
                .all(|c| matches!(scope(c), Some("column" | "both")))
    });
    let rows = !grid.content.is_empty()
        && grid.content.iter().all(|r| {
            r.content
                .first()
                .is_some_and(|c| matches!(scope(c), Some("row" | "both")))
        });
    (columns, rows)
}
pub(crate) fn cell_scope(
    columns: bool,
    rows: bool,
    row: usize,
    column: usize,
) -> Option<&'static str> {
    match (columns && row == 0, rows && column == 0) {
        (true, true) => Some("both"),
        (true, false) => Some("column"),
        (false, true) => Some("row"),
        _ => None,
    }
}
pub(crate) fn scoped_cell(scope: Option<&str>) -> Node {
    let mut result = cell(scope.is_some());
    if let Some(scope) = scope.filter(|s| *s != "column") {
        result.attrs = Some(serde_json::json!({"scope":scope}));
    }
    result
}
