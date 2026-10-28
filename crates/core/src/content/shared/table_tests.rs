use super::command_tests::{body, command, execute, session};
use super::*;
use crate::{content::commands::Command, document_types::DocumentType};
use serde_json::{Value, json};
use yrs::types::SharedRef;
fn tree() -> Value {
    json!({"root":{"kind":"body"},"path":[1]})
}
fn table() -> SharedSession {
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![command(
            "insert.table",
            json!({"parent":{"root":{"kind":"body"}},"index":1,"rows":2,"columns":2,"header":true}),
        )],
    );
    s
}
fn run(s: &mut SharedSession, id: &str, index: u32) {
    execute(s, vec![command(id, json!({"target":tree(),"index":index}))]);
}
fn xml(s: &SharedSession, path: &[usize]) -> XmlElementRef {
    targets::element(
        s.shared.doc.transact().get_xml_fragment("default").unwrap(),
        &s.shared.doc.transact(),
        path,
    )
    .unwrap()
}
use yrs::XmlElementRef;
#[test]
fn captions_are_rich_annotations_and_tables_always_export_as_figures_without_alt_override() {
    let mut s = table();
    execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([1,1]),0,0),"text":"Données #test 日本","marks":[{"type":"italic"}]}),
        )],
    );
    let source = s.state.validate(&Assets::default()).unwrap();
    assert!(source.contains("#figure(table(columns: 2"));
    assert!(source.contains("caption: [#emph[Données \\#test 日本]]"));
    assert!(!source.contains("alt:"));
    let table = &s.state.document.content[1];
    assert_eq!(table.kind, "tableFigure");
    assert_eq!(table.content[1].kind, "tableCaption");
}
#[test]
fn fine_edits_preserve_grid_caption_existing_rows_and_unaffected_cells() {
    let mut s = table();
    let grid = xml(&s, &[1, 0]).hook();
    let caption = xml(&s, &[1, 1]).hook();
    let row = xml(&s, &[1, 0, 1]).hook();
    let cell = xml(&s, &[1, 0, 1, 1]).hook();
    execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([1,0,1,1,0]),0,0),"text":"é😀","marks":[{"type":"bold"}]}),
        )],
    );
    run(&mut s, "table.row.insert", 1);
    run(&mut s, "table.column.insert", 1);
    assert_eq!(xml(&s, &[1, 0]).hook().id(), grid.id());
    assert_eq!(xml(&s, &[1, 1]).hook().id(), caption.id());
    assert_eq!(xml(&s, &[1, 0, 2]).hook().id(), row.id());
    assert_eq!(xml(&s, &[1, 0, 2, 2]).hook().id(), cell.id());
    run(&mut s, "table.row.remove", 1);
    run(&mut s, "table.column.remove", 1);
    assert_eq!(xml(&s, &[1, 0, 1, 1]).hook().id(), cell.id());
    assert_eq!(
        s.state.document.content[1].content[0].content[1].content[1].content[0].content[0]
            .text
            .as_deref(),
        Some("é😀")
    );
}
#[test]
fn inserting_before_the_header_keeps_one_semantic_header_and_preserves_its_text() {
    let mut s = table();
    execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([1,0,0,0,0]),0,0),"text":"Ancien en-tête"}),
        )],
    );
    run(&mut s, "table.row.insert", 0);
    let grid = &s.state.document.content[1].content[0];
    assert!(
        grid.content[0]
            .content
            .iter()
            .all(|n| n.kind == "tableHeader")
    );
    assert!(
        grid.content[1]
            .content
            .iter()
            .all(|n| n.kind == "tableCell")
    );
    assert_eq!(
        grid.content[1].content[0].content[0].content[0]
            .text
            .as_deref(),
        Some("Ancien en-tête")
    );
    let row = xml(&s, &[1, 0, 2]).hook();
    execute(
        &mut s,
        vec![command(
            "table.header",
            json!({"target":tree(),"enabled":false}),
        )],
    );
    assert_eq!(xml(&s, &[1, 0, 2]).hook().id(), row.id());
    assert!(
        !execute(
            &mut s,
            vec![command(
                "table.header",
                json!({"target":tree(),"enabled":false})
            )]
        )
        .changed
    );
}
#[test]
fn last_row_column_limits_and_late_failure_cannot_publish_partial_changes() {
    let mut s = table();
    for (id, index) in [
        ("table.row.remove", 5),
        ("table.column.remove", 5),
        ("table.row.insert", 3),
        ("table.column.insert", 3),
    ] {
        let before = s.update();
        let steps: Vec<Command> = vec![
            command(
                "text.insert",
                json!({"target":body(json!([1,1]),0,0),"text":"Provisoire"}),
            ),
            command(id, json!({"target":tree(),"index":index})),
        ];
        assert!(s.prepare_commands(s.guard(), &steps).is_err());
        assert_eq!(s.update(), before);
    }
    run(&mut s, "table.row.remove", 0);
    run(&mut s, "table.column.remove", 0);
    for id in ["table.row.remove", "table.column.remove"] {
        assert!(
            !s.availability(s.guard(), &command(id, json!({"target":tree(),"index":0})))
                .enabled
        );
    }
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![command(
            "insert.table",
            json!({"parent":{"root":{"kind":"body"}},"index":1,"rows":100,"columns":12,"header":false}),
        )],
    );
    for id in ["table.row.insert", "table.column.insert"] {
        assert!(
            !s.availability(s.guard(), &command(id, json!({"target":tree(),"index":0})))
                .enabled
        );
    }
}
