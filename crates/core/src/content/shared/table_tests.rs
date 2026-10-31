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

#[test]
fn header_axes_are_independent_even_in_one_cell_and_invalid_scope_is_rejected() {
    for columns in [false, true] {
        for rows in [false, true] {
            let grid = crate::content::tables::create(1, 1, columns, rows).unwrap();
            assert_eq!(
                crate::content::tables::headers(&grid.content[0]),
                (columns, rows)
            );
            let mut s = table();
            execute(
                &mut s,
                vec![
                    command(
                        "table.header",
                        json!({"target":tree(),"enabled":columns,"axis":"columns"}),
                    ),
                    command(
                        "table.header",
                        json!({"target":tree(),"enabled":rows,"axis":"rows"}),
                    ),
                ],
            );
            assert_eq!(
                crate::content::tables::headers(&s.state.document.content[1].content[0]),
                (columns, rows)
            );
            let receipt = execute(
                &mut s,
                vec![command(
                    "table.header",
                    json!({"target":tree(),"enabled":rows,"axis":"rows"}),
                )],
            );
            assert!(!receipt.changed);
        }
    }
    assert!(
        serde_json::from_value::<Command>(
            json!({"id":"table.header","params":{"target":tree(),"enabled":true,"axis":"diagonal"}})
        )
        .is_err()
    );
    let mut node = crate::content::tables::create(2, 2, true, true).unwrap();
    node.content[0].content[1].content[0].attrs = Some(json!({"scope":"unknown"}));
    let doc = serde_json::from_value(json!({"type":"doc","content":[node]})).unwrap();
    assert!(crate::content::source(&doc, &Assets::default()).is_err());
}

#[test]
fn toggling_headers_preserves_rich_text_caption_unrelated_cells_and_atomic_rollback() {
    let mut s = table();
    execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([1,0,1,0,0]),0,0),"text":"Nom é😀日本","marks":[{"type":"italic"}]}),
        )],
    );
    let caption = xml(&s, &[1, 1]).hook();
    let untouched = xml(&s, &[1, 0, 1, 1]).hook();
    execute(
        &mut s,
        vec![command(
            "table.header",
            json!({"target":tree(),"enabled":true,"axis":"rows"}),
        )],
    );
    assert_eq!(xml(&s, &[1, 1]).hook().id(), caption.id());
    assert_eq!(xml(&s, &[1, 0, 1, 1]).hook().id(), untouched.id());
    let cell = &s.state.document.content[1].content[0].content[1].content[0];
    assert_eq!(crate::content::tables::scope(cell), Some("row"));
    assert_eq!(
        cell.content[0].content[0].text.as_deref(),
        Some("Nom é😀日本")
    );
    assert_eq!(cell.content[0].content[0].marks[0].kind, "italic");
    let before = s.update();
    assert!(
        s.prepare_commands(
            s.guard(),
            &[
                command(
                    "table.header",
                    json!({"target":tree(),"enabled":false,"axis":"rows"})
                ),
                command("table.column.remove", json!({"target":tree(),"index":42}))
            ]
        )
        .is_err()
    );
    assert_eq!(s.update(), before);
    execute(
        &mut s,
        vec![
            command(
                "table.header",
                json!({"target":tree(),"enabled":false,"axis":"rows"}),
            ),
            command(
                "table.header",
                json!({"target":tree(),"enabled":false,"axis":"columns"}),
            ),
        ],
    );
    assert_eq!(
        s.state.document.content[1].content[0].content[1].content[0].content[0].content[0]
            .text
            .as_deref(),
        Some("Nom é😀日本")
    );
}

#[test]
fn leading_track_edits_keep_header_options_and_do_not_copy_old_labels() {
    let mut s = table();
    execute(
        &mut s,
        vec![
            command(
                "table.header",
                json!({"target":tree(),"enabled":true,"axis":"rows"}),
            ),
            command(
                "text.insert",
                json!({"target":body(json!([1,0,1,0,0]),0,0),"text":"Ancien nom"}),
            ),
        ],
    );
    run(&mut s, "table.column.insert", 0);
    assert_eq!(
        crate::content::tables::scope(
            &s.state.document.content[1].content[0].content[1].content[0]
        ),
        Some("row")
    );
    assert_eq!(
        s.state.document.content[1].content[0].content[1].content[1].kind,
        "tableCell"
    );
    assert_eq!(
        s.state.document.content[1].content[0].content[1].content[1].content[0].content[0]
            .text
            .as_deref(),
        Some("Ancien nom")
    );
    for (id, index) in [
        ("table.row.insert", 0),
        ("table.row.remove", 0),
        ("table.column.remove", 0),
    ] {
        run(&mut s, id, index);
        assert_eq!(
            crate::content::tables::headers(&s.state.document.content[1].content[0]),
            (true, true)
        );
    }
    assert_eq!(
        crate::content::tables::scope(
            &s.state.document.content[1].content[0].content[0].content[0]
        ),
        Some("both")
    );
}

#[test]
fn row_headers_compile_with_only_the_embedded_wrapper_and_export_tagged_pdf() {
    use crate::{
        document_types::{DocumentType, Metadata},
        rendering::TypstWorld,
    };
    let mut table = crate::content::tables::create(3, 2, true, true).unwrap();
    for (r, row) in table.content[0].content.iter_mut().enumerate() {
        for (c, cell) in row.content.iter_mut().enumerate() {
            cell.content[0].content = vec![
                serde_json::from_value(json!({"type":"text","text":format!("Cell {r}x{c}")}))
                    .unwrap(),
            ];
        }
    }
    let document = serde_json::from_value(json!({"type":"doc","content":[table]})).unwrap();
    let source = crate::content::source(&document, &Assets::default()).unwrap();
    assert!(source.contains("table-header-cell([#par[Cell 1x0]], scope: \"row\")"));
    assert!(source.contains("scope: \"both\""));
    assert!(source.contains("--features a11y-extras"));
    let dependencies = crate::packages::for_template(&source);
    assert!(dependencies.contains_key("capabilities/tables.typ"));
    let mut world = TypstWorld::default();
    world.set_dependencies(&dependencies);
    let metadata = Metadata::blank(DocumentType::Courrier);
    let document = world
        .compile_with_template(
            &source,
            &metadata,
            "#let courrier(..args) = args.pos().last()",
        )
        .unwrap();
    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    let structure = String::from_utf8_lossy(&pdf).replace(' ', "");
    assert!(
        structure.contains("/Scope/Row"),
        "PDF must carry semantic row headers"
    );
    assert!(
        structure.contains("/Scope/Both"),
        "Top-left cell must label both axes"
    );
    let mut missing = (*dependencies).clone();
    missing.remove("capabilities/tables.typ");
    world.set_dependencies(&std::sync::Arc::new(missing));
    assert!(
        world
            .compile_with_template(
                &source,
                &metadata,
                "#let courrier(..args) = args.pos().last()"
            )
            .is_err()
    );
}

#[test]
fn row_header_project_reopens_and_exports_with_its_own_typst_dependency() {
    let mut s = table();
    execute(
        &mut s,
        vec![command(
            "table.header",
            json!({"target":tree(),"axis":"rows","enabled":true}),
        )],
    );
    let metadata = s.state.settings.metadata.clone();
    let archive = crate::content::archive::Archive {
        document: s.state.document.clone(),
        metadata,
        template: s.state.settings.template.clone(),
        model_id: s.state.settings.model_id.clone(),
        assets: Assets::default(),
        dependencies: crate::packages::bundled(),
        yjs: s.update(),
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("headers.typsto");
    let mut lease = crate::DocumentLease::acquire(&path).unwrap();
    crate::content::project::Project::new(archive.clone())
        .unwrap()
        .save(&mut lease)
        .unwrap();
    let saved = crate::content::project::Project::read(&path)
        .unwrap()
        .current()
        .clone();
    assert_eq!(saved.document, archive.document);
    assert_eq!(saved.yjs, archive.yjs);
    assert!(saved.dependencies.contains_key("capabilities/tables.typ"));
    let source = saved.validate().unwrap();
    let mut world = crate::rendering::TypstWorld::default();
    world.set_dependencies(&saved.dependencies);
    let rendered = world
        .compile_with_template(&source, &saved.metadata, &saved.template)
        .unwrap();
    let output = directory.path().join("export");
    crate::exporting::write_document(
        &rendered,
        &crate::rendering::RenderRequest {
            source: source.clone(),
            metadata: saved.metadata.clone(),
            template: saved.template.clone(),
            assets: saved.assets.clone(),
            dependencies: saved.dependencies.clone(),
        },
        &output,
        crate::protocol::ExportFormat::Typst,
        false,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(output.join("capabilities/tables.typ")).unwrap(),
        saved.dependencies["capabilities/tables.typ"].as_ref()
    );
}
