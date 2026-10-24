use super::*;
use serde_json::json;
use typstographe_core::engine::{Engine, ErrorCode};

fn run(host: &mut host::Host, id: &str, args: Value) -> Value {
    host.dispatch(id, args.as_object().unwrap().clone())
        .unwrap()
        .value
}
fn created(host: &mut host::Host) -> Value {
    run(
        host,
        "document.new",
        json!({"type":"courrier","model":"courrier.model.personal"}),
    )["document"]
        .clone()
}
fn state(host: &mut host::Host) -> Value {
    run(host, "document.inspect", json!({}))
}

#[test]
fn tools_are_generated_from_the_registry_and_local_schema_references_resolve() {
    let tools = catalogue::tools();
    assert_eq!(
        tools.len(),
        typstographe_core::engine::catalogue()
            .iter()
            .filter(|s| policy::exposed(s))
            .count()
            + 3
    );
    let names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.windows(2).all(|w| w[0] < w[1]));
    fn refs(value: &Value, root: &Value) {
        match value {
            Value::Object(map) => {
                if let Some(path) = map
                    .get("$ref")
                    .and_then(Value::as_str)
                    .and_then(|r| r.strip_prefix('#'))
                {
                    assert!(root.pointer(path).is_some(), "unresolved {path}");
                }
                for value in map.values() {
                    refs(value, root);
                }
            }
            Value::Array(values) => {
                for value in values {
                    refs(value, root);
                }
            }
            _ => {}
        }
    }
    for tool in tools {
        let schema = json!(tool.input_schema);
        refs(&schema, &schema);
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema["properties"]["guard"].is_null());
        if let Some(image) = schema.get("$defs").and_then(|d| d.get("ImageInput")) {
            assert!(
                serde_json::to_string(image)
                    .unwrap()
                    .contains("\"const\":\"path\"")
            );
        }
    }
    assert!(names.contains(&"document.open"));
    assert!(names.contains(&"document.save"));
    assert!(names.contains(&"document.batch"));
    assert!(names.contains(&"render.page"));
}

#[test]
fn types_and_models_are_self_describing_without_exposing_a_model_contract() {
    let mut host = host::Host::new(false);
    let types = run(&mut host, "document_types.list", json!({}));
    assert_eq!(types["outcome"]["types"][1]["can_create"], false);
    let model = &types["outcome"]["types"][0]["models"][0];
    assert!(model["description"].as_str().unwrap().contains("DL"));
    assert!(model["keywords"].as_array().unwrap().len() > 2);
    assert!(model["fixed_page"].is_null());
    let kind = run(
        &mut host,
        "document_types.describe",
        json!({"document_type":"courrier"}),
    );
    assert!(
        kind["outcome"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["field"] == "letter.date")
    );
    assert!(
        kind["outcome"]["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == "letter.insert_postscript")
    );
    let doc = run(
        &mut host,
        "capabilities.describe",
        json!({"id":"text.insert"}),
    );
    assert!(
        doc["outcome"]["details"]
            .as_str()
            .unwrap()
            .contains("literal")
    );
    assert!(!doc["outcome"]["examples"].as_array().unwrap().is_empty());
    let denied = run(
        &mut host,
        "capabilities.describe",
        json!({"id":"document.save"}),
    );
    assert_eq!(denied["outcome"]["prototype_support"]["exposed"], true);
    assert_eq!(denied["outcome"]["prototype_support"]["authorized"], false);
    assert_eq!(resources::list().len(), 3);
    let types: Value =
        serde_json::from_str(&resources::read("typstographe://types").unwrap()).unwrap();
    assert_eq!(types["types"][0]["id"], "courrier");
    assert!(resources::read("file:///etc/passwd").is_none());
}

#[test]
fn independent_handles_and_revision_guards_protect_persistent_sessions() {
    let mut host = host::Host::new(false);
    let doc = created(&mut host);
    let mut other = host::Host::new(false);
    let foreign = created(&mut other);
    let error = host
        .dispatch(
            "text.insert",
            json!({"document":foreign,"text":"Non"})
                .as_object()
                .unwrap()
                .clone(),
        )
        .err()
        .unwrap();
    assert_eq!(error.reason(), "mcp_document_foreign");
    let first = run(
        &mut host,
        "text.insert",
        json!({"document":doc,"text":"Élodie"}),
    );
    assert!(first["document"]["guard"]["revision"].as_u64() > doc["guard"]["revision"].as_u64());
    assert_eq!(
        host.dispatch(
            "text.insert",
            json!({"document":doc,"text":"Non"})
                .as_object()
                .unwrap()
                .clone()
        )
        .err()
        .unwrap()
        .reason(),
        "stale_revision"
    );
    assert_eq!(
        host.dispatch(
            "text.insert",
            json!({"text":"Non"}).as_object().unwrap().clone()
        )
        .err()
        .unwrap()
        .reason(),
        "mcp_document_required"
    );
    let current = state(&mut host)["document"].clone();
    assert_eq!(
        run(&mut host, "document.text", json!({"document":current}))["outcome"]["text"],
        "Élodie"
    );
    let error = host
        .dispatch(
            "document.new",
            json!({"document":current,"type":"courrier","model":"courrier.model.personal"})
                .as_object()
                .unwrap()
                .clone(),
        )
        .err()
        .unwrap();
    assert_eq!(error.reason(), "unsaved_changes");
    let replacement = run(
        &mut host,
        "document.new",
        json!({"document":current,"type":"courrier","model":"courrier.model.personal","discard_changes":true}),
    );
    assert_ne!(replacement["document"]["id"], current["id"]);
}

#[test]
fn a_resolved_batch_is_one_transaction_and_a_late_failure_preserves_the_whole_state() {
    let mut host = host::Host::new(false);
    let document = created(&mut host);
    let before = state(&mut host);
    let error = host
        .dispatch(
            "document.batch",
            json!({"document":document,"steps":[
                {"id":"text.insert","params":{"text":"Temporaire Élodie"}},
                {"id":"text.bold","params":{"selector":{"select":"Absent"}}}
            ]})
            .as_object()
            .unwrap()
            .clone(),
        )
        .err()
        .unwrap();
    assert_eq!(error.params["step"], 1);
    assert_eq!(state(&mut host), before);
    let result = run(
        &mut host,
        "document.batch",
        json!({"document":document,"steps":[
            {"id":"text.insert","params":{"text":"Bonjour Élodie."}},
            {"id":"text.bold","params":{"value":"on","selector":{"select":"Élodie"}}},
            {"id":"letter.insert_postscript","params":{}},
            {"id":"text.insert","params":{"text":"Une précision."}}
        ]}),
    );
    let document = result["document"].clone();
    assert!(
        run(&mut host, "document.text", json!({"document":document}))["outcome"]["text"]
            .as_str()
            .unwrap()
            .contains("Une précision.")
    );
    let undone = run(&mut host, "edit.undo", json!({"document":document}));
    assert_eq!(
        run(
            &mut host,
            "document.text",
            json!({"document":undone["document"]})
        )["outcome"]["text"],
        ""
    );
    let redo = run(
        &mut host,
        "edit.redo",
        json!({"document":undone["document"]}),
    );
    let source = run(
        &mut host,
        "document.source",
        json!({"document":redo["document"]}),
    );
    assert!(
        source["outcome"]["source"]
            .as_str()
            .unwrap()
            .contains("Élodie")
    );
    let before = state(&mut host);
    let error=host.dispatch("insert.image",json!({"document":redo["document"],"selector":{"select":"Élodie"},"image":{"kind":"embedded","extension":"svg","data":"invalid base64"}}).as_object().unwrap().clone()).err().unwrap();
    assert!(matches!(
        error.code,
        ErrorCode::InvalidParams | ErrorCode::Document
    ));
    assert_eq!(state(&mut host), before);
}

#[test]
fn all_filesystem_variants_and_read_only_mutations_are_refused_before_publication() {
    let mut host = host::Host::new(false);
    let document = created(&mut host);
    let before = state(&mut host);
    for (id, params) in [
        (
            "document.new",
            json!({"type":"courrier","model":"courrier.model.personal","discard_changes":true,"signature":{"kind":"path","path":"/etc/passwd"}}),
        ),
        (
            "insert.image",
            json!({"image":{"kind":"path","path":"missing.svg"}}),
        ),
        (
            "document.set_signature",
            json!({"image":{"kind":"path","path":"../signature.svg"}}),
        ),
        (
            "document.set_model",
            json!({"model":{"kind":"path","path":"anything.typ"}}),
        ),
        ("document.open", json!({"path":"anything.typsto"})),
        ("document.save", json!({})),
        (
            "document.batch",
            json!({"steps":[{"id":"text.insert","params":{"text":"Non"}}],"save":{"kind":"as","path":"anything.typsto"}}),
        ),
        (
            "document.batch",
            json!({"steps":[{"id":"text.insert","params":{"text":"Non"}},{"id":"insert.image","params":{"image":{"kind":"path","path":"missing.svg"}}}]}),
        ),
    ] {
        let mut params = params.as_object().unwrap().clone();
        params.insert("document".into(), document.clone());
        let error = host.dispatch(id, params).err().unwrap();
        assert_eq!(error.code, ErrorCode::Unavailable, "{id}: {error}");
        assert_eq!(state(&mut host), before, "{id}");
    }
    let mut readonly = host::Host::new(true);
    assert_eq!(
        readonly
            .dispatch(
                "document.new",
                json!({"type":"courrier","model":"courrier.model.personal"})
                    .as_object()
                    .unwrap()
                    .clone()
            )
            .err()
            .unwrap()
            .reason(),
        "mcp_read_only"
    );
    assert!(
        !run(&mut readonly, "document_types.list", json!({}))["outcome"]["types"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unknown_parameters_and_oversized_requests_are_refused_without_implicit_edits() {
    let mut host = host::Host::new(false);
    let document = created(&mut host);
    let before = state(&mut host);
    for args in [
        json!({"document":document,"text":"Non","unexpected":true}),
        json!({"document":document,"text":"Non","guard":document["guard"]}),
        json!({"document":document,"text":"x".repeat(typstographe_core::engine::MAX_BATCH_BYTES)}),
    ] {
        assert_eq!(
            host.dispatch("text.insert", args.as_object().unwrap().clone())
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidParams
        );
        assert_eq!(state(&mut host), before);
    }
    assert_eq!(
        Engine::command_documentation("missing")
            .unwrap_err()
            .reason(),
        "unknown_command"
    );
}
