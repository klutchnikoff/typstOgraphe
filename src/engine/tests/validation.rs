use super::*;

fn read(e: &Engine, id: &str) -> Value {
    e.query_call(
        Query::from_call(id, json!({"guard":e.target_guard()}))
            .unwrap()
            .to_call(),
    )
    .unwrap()
}

#[test]
fn validation_distinguishes_valid_draft_missing_export_fields_and_ready_document() {
    let mut e = engine("Un *brouillon* avec Noël.");
    let before = e.stored_document();
    let inspect = e.query(Query::Inspect(Empty {})).unwrap();
    let report = read(&e, "document.validate");
    assert_eq!(report["valid"], true);
    assert_eq!(report["ready_for_export"], false);
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    for d in diagnostics {
        assert_eq!(d["severity"], "warning");
        assert_eq!(d["code"], "export_required_field");
    }
    assert!(diagnostics.iter().any(|d| d["field"] == "recipient.name"));
    assert!(diagnostics.iter().any(|d| d["field"] == "letter.subject"));
    assert_eq!(e.stored_document(), before);
    assert_eq!(e.query(Query::Inspect(Empty {})).unwrap(), inspect);
    for (field, value) in [
        ("recipient.name", "Service administratif"),
        ("letter.subject", "Un sujet"),
    ] {
        let guard = e.target_guard();
        call(
            &mut e,
            "metadata.set",
            json!({"guard":guard,"field":field,"value":value}),
        )
        .unwrap();
    }
    let report = read(&e, "document.validate");
    assert_eq!(report["ready_for_export"], true);
    assert!(report["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn validation_reports_broken_resources_contract_dependencies_and_model_options_without_committing()
{
    let cases: Vec<(Engine, &str)> = vec![
        (
            engine("#figure(image(\"images/missing.svg\"))"),
            "document_storage",
        ),
        (
            {
                let mut e = engine("Texte");
                e.session.template = "#let unrelated(body) = body".into();
                e
            },
            "model_validation",
        ),
        (
            {
                let mut e = engine("Texte");
                e.session.template = "#let courrier(".into();
                e
            },
            "document_storage",
        ),
        (
            {
                let mut e = engine("Texte");
                e.session.dependencies = Default::default();
                e
            },
            "document_storage",
        ),
        (
            {
                let mut e = engine("Texte");
                e.session.metadata.options_mut().paper = crate::document::PaperFormat::A3;
                e
            },
            "model_fixed_page",
        ),
    ];
    for (e, code) in cases {
        let before = e.stored_document();
        let report = read(&e, "document.validate");
        assert_eq!(report["valid"], false, "{code}: {report}");
        assert_eq!(report["ready_for_export"], false);
        assert!(
            report["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"] == code && d["severity"] == "error"),
            "{report}"
        );
        assert_eq!(e.stored_document(), before);
        assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
        assert!(e.render_info().is_none());
    }
}

#[test]
fn expensive_reads_reject_closed_foreign_and_stale_guards_and_strictly_validate_arguments() {
    let mut e = engine("Été");
    let other = engine("Autre");
    let old = e.target_guard();
    call(&mut e, "text.insert", json!({"text":"Noël "})).unwrap();
    for id in ["document.validate", "render.compile"] {
        for (guard, reason) in [
            (old, "stale_revision"),
            (other.target_guard(), "foreign_document"),
        ] {
            let q = Query::from_call(id, json!({"guard":guard})).unwrap();
            assert_eq!(e.query(q).unwrap_err().reason(), reason);
        }
        for params in [
            json!({}),
            json!({"guard":e.target_guard(),"export":true}),
            json!({"guard":{"document_id":1}}),
        ] {
            assert!(Query::from_call(id, params).is_err());
        }
        let closed = Engine::empty();
        assert_eq!(
            closed
                .query(Query::from_call(id, json!({"guard":closed.target_guard()})).unwrap())
                .unwrap_err()
                .reason(),
            "no_document"
        );
    }
}

#[test]
fn public_compile_registers_usable_geometry_reuses_current_render_and_preserves_published_state() {
    let mut e = engine(&"Noël rédige un *courrier*.\n\n".repeat(70));
    let before = e.stored_document();
    let selected = explicit(&e, 0..4);
    call(&mut e, "text.select", json!({"target":selected})).unwrap();
    e.session.typing_group = true;
    let output = read(&e, "render.compile");
    let render: RenderInfo = serde_json::from_value(output["render"].clone()).unwrap();
    assert!(render.pages.len() > 1);
    assert!(output["diagnostics"].as_array().unwrap().is_empty());
    let boxes = e
        .selection_boxes(SelectionBoxesParams {
            render: render.reference,
            target: selected,
        })
        .unwrap();
    assert!(!boxes.boxes.is_empty());
    let location = e
        .locate(LocateParams {
            render: render.reference,
            position: RenderPosition::ToPreview { target: selected },
        })
        .unwrap();
    let back = e
        .locate(LocateParams {
            render: render.reference,
            position: RenderPosition::ToSource {
                point: location.point.unwrap(),
            },
        })
        .unwrap();
    assert_eq!(e.resolve_target(back.target.unwrap()).unwrap(), (4, 4));
    assert_eq!(read(&e, "render.compile"), output); // Même cache, pas d'invalidation des utilisateurs du rendu.
    assert_eq!(e.stored_document(), before);
    assert_eq!(
        TextTarget::from(e.selection_target(e.selection()).unwrap()),
        selected
    );
    assert!(e.session.typing_group);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    assert!(!e.is_dirty());
    assert_eq!(
        e.query(Query::Inspect(Empty {})).unwrap()["render"],
        output["render"]
    );
    call(&mut e, "text.insert", json!({"text":"Bonjour"})).unwrap();
    let next = read(&e, "render.compile");
    assert_ne!(next["render"]["reference"], output["render"]["reference"]);
    assert_eq!(
        e.selection_boxes(SelectionBoxesParams {
            render: render.reference,
            target: TextTarget::default()
        })
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
}

#[test]
fn failed_compilation_returns_diagnostics_and_ignores_unpublished_native_preedit() {
    let mut e = engine("Texte publié.");
    let edit = e.begin_native_edit(e.selection());
    let mut draft = edit.view().clone();
    draft.insert_text("にほん", 0);
    assert_eq!(
        e.query(Query::Source(Empty {})).unwrap()["source"],
        "Texte publié."
    );
    let before = e.stored_document();
    assert!(read(&e, "render.compile")["render"].is_object());
    assert_eq!(e.stored_document(), before);
    assert_eq!(e.document().source(), "Texte publié.");
    drop(edit);
    e.session.template = "#let courrier(..args) = unknown-function()".into();
    *e.render.get_mut() = None;
    let before = e.stored_document();
    let report = read(&e, "render.compile");
    assert!(report["render"].is_null());
    assert_eq!(report["diagnostics"][0]["code"], "render_compile");
    assert_eq!(report["diagnostics"][0]["severity"], "error");
    assert!(
        report["diagnostics"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("unknown-function")
    );
    assert!(e.render_info().is_none());
    assert_eq!(e.stored_document(), before);
    assert!(e.session.undo.is_empty());
}
