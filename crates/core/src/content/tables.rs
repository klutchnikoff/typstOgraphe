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
pub fn create(rows: u32, columns: u32, header: bool) -> Result<Node, Error> {
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
                    (0..columns).map(|_| cell(header && r == 0)).collect(),
                )
            })
            .collect(),
    );
    Ok(node(
        "tableFigure",
        vec![grid, node("tableCaption", vec![])],
    ))
}
