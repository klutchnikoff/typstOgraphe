use super::*;
use serde_json::{Value, json};
use typstographe_core::engine::{Engine, catalogue as core_catalogue};

fn call(host: &mut host::Host, id: &str, args: Value) -> Value {
    host.dispatch(id, args.as_object().unwrap().clone())
        .unwrap()
        .value
}
fn created(host: &mut host::Host) -> Value {
    call(
        host,
        "document.new",
        json!({"type":"courrier","model":"courrier.model.personal"}),
    )["document"]
        .clone()
}
fn resource(host: &host::Host, uri: &str) -> Value {
    host.resource(uri).unwrap().value
}
fn validator(schema: &Value) -> jsonschema::Validator {
    jsonschema::options().offline().build(schema).unwrap()
}

#[test]
fn documented_style_batches_keep_inserted_formatting_stop_future_style_and_undo_once() {
    let schema = json!(
        catalogue::tools()
            .iter()
            .find(|t| t.name == "document.batch")
            .unwrap()
            .input_schema
    );
    let valid = validator(&schema);
    for (id, style) in [("text.bold", "bold"), ("text.italic", "italic")] {
        let mut host = host::Host::new(false);
        let doc = created(&mut host);
        let docs = call(&mut host, "capabilities.describe", json!({"id":id}));
        let examples = docs["outcome"]["batch_examples"].as_array().unwrap();
        assert_eq!(examples.len(), 1);
        let example = &examples[0];
        assert_eq!(example["id"], "document.batch");
        assert!(example["preconditions"].as_str().unwrap().contains("empty"));
        assert!(example["expected"].as_str().unwrap().contains("undo"));
        let mut args = example["params"].clone();
        args["document"] = doc;
        assert!(valid.is_valid(&args));
        let edited = call(&mut host, "document.batch", args);
        let source = call(
            &mut host,
            "document.source",
            json!({"document":edited["document"]}),
        );
        let document = typstographe_core::document::Document::from_source(
            source["outcome"]["source"].as_str().unwrap(),
        )
        .unwrap();
        let steps = example["params"]["steps"].as_array().unwrap();
        let marked = steps[1]["params"]["text"].as_str().unwrap();
        let following = steps[3]["params"]["text"].as_str().unwrap();
        assert_eq!(document.view.text, format!("{marked}{following}"));
        for (index, actual) in document.view.styles.iter().enumerate() {
            let enabled = index < marked.chars().count();
            assert_eq!(actual.bold, style == "bold" && enabled);
            assert_eq!(actual.italic, style == "italic" && enabled);
        }
        let inspection = call(&mut host, "document.inspect", json!({}));
        assert_eq!(inspection["outcome"]["typing_style"]["bold"], false);
        assert_eq!(inspection["outcome"]["typing_style"]["italic"], false);
        let undone = call(
            &mut host,
            "edit.undo",
            json!({"document":edited["document"]}),
        );
        assert_eq!(
            call(
                &mut host,
                "document.text",
                json!({"document":undone["document"]})
            )["outcome"]["text"],
            ""
        );
        assert_eq!(
            call(&mut host, "document.inspect", json!({}))["outcome"]["can_undo"],
            false
        );
        let redone = call(
            &mut host,
            "edit.redo",
            json!({"document":undone["document"]}),
        );
        assert_eq!(
            call(
                &mut host,
                "document.source",
                json!({"document":redone["document"]})
            )["outcome"]["source"],
            source["outcome"]["source"]
        );
    }
}

#[test]
fn style_batch_examples_are_the_same_in_fiches_resources_and_the_mcp_guide() {
    let mut host = host::Host::new(false);
    let docs = call(
        &mut host,
        "capabilities.describe",
        json!({"id":"document.batch"}),
    );
    let examples = docs["outcome"]["batch_examples"].clone();
    assert_eq!(examples.as_array().unwrap().len(), 2);
    let insert = call(
        &mut host,
        "capabilities.describe",
        json!({"id":"text.insert"}),
    );
    assert_eq!(insert["outcome"]["batch_examples"], examples);
    let resource = resource(&host, "typstographe://commands");
    for id in ["text.bold", "text.italic", "text.insert", "document.batch"] {
        let entry = resource["outcome"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["command"]["id"] == id)
            .unwrap();
        assert_eq!(
            entry["batch_examples"],
            Engine::command_documentation(id).unwrap()["batch_examples"]
        );
    }
    let guide = include_str!("../../../../docs/mcp.md");
    let params: Vec<_> = examples
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["params"].clone())
        .collect();
    let lf = guide.replace("\r\n", "\n");
    // Exercise both checkout formats on every OS, regardless of core.autocrlf.
    for (format, guide) in [("LF", lf.clone()), ("CRLF", lf.replace('\n', "\r\n"))] {
        let guide = guide.replace("\r\n", "\n");
        let section = guide
            .split("## Gras et italique : délimiter les insertions")
            .nth(1)
            .unwrap()
            .split("\n## ")
            .next()
            .unwrap();
        let snippets: Vec<Value> = section
            .split("```json\n")
            .skip(1)
            .map(|s| serde_json::from_str(s.split("\n```").next().unwrap()).unwrap())
            .collect();
        assert_eq!(snippets, params, "Markdown {format}");
    }
    let guide = host.resource("typstographe://guide").unwrap().value;
    let instructions = guide["outcome"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("on/insert/off"));
    assert!(instructions.contains("*bold* and _italic_ are literal text"));
}

#[test]
fn batch_step_schema_advertises_the_object_params_required_by_the_adapter() {
    let schema = json!(
        catalogue::tools()
            .iter()
            .find(|tool| tool.name == "document.batch")
            .unwrap()
            .input_schema
    );
    let valid = validator(&schema);
    let mut host = host::Host::new(false);
    let doc = created(&mut host);
    let before = call(&mut host, "document.inspect", json!({}));
    let params = |value: Value| {
        json!({"document":doc,"steps":[
            {"id":"text.insert","params":{"text":"Do not publish"}},
            {"id":"text.insert","params":value}
        ]})
    };
    assert!(valid.is_valid(&params(json!({"text":"Valid object"}))));
    for value in [
        Value::Null,
        json!(true),
        json!(42),
        json!("text"),
        json!([]),
    ] {
        let args = params(value);
        assert!(!valid.is_valid(&args));
        assert!(
            host.dispatch("document.batch", args.as_object().unwrap().clone())
                .is_err()
        );
        assert_eq!(call(&mut host, "document.inspect", json!({})), before);
    }
    // L'adaptation MCP ne modifie pas le contrat gelé du cœur.
    assert_eq!(
        core_catalogue()
            .iter()
            .find(|spec| spec.id.as_str() == "document.batch")
            .unwrap()
            .params_schema["$defs"]["Call"]["properties"]["params"],
        true
    );
}

#[test]
fn every_documented_tool_example_validates_with_current_bound_references() {
    let mut h = host::Host::new(false);
    let doc = created(&mut h);
    let text = call(
        &mut h,
        "text.insert",
        json!({"document":doc,"text":"Élodie bonjor"}),
    );
    let image = call(
        &mut h,
        "insert.image",
        json!({"document":text["document"],"image":{"kind":"embedded","extension":"svg","data":"PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxIiBoZWlnaHQ9IjEiLz4="}}),
    );
    let doc = image["document"].clone();
    let structure = call(&mut h, "document.structure", json!({"document":doc}))["outcome"].clone();
    let image = structure["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "image")
        .unwrap();
    let image_target = json!({"guard":structure["guard"],"block_id":image["id"]});
    let search = call(
        &mut h,
        "search.find",
        json!({"document":doc,"query":"Élodie"}),
    )["outcome"]["matches"][0]
        .clone();
    let issue =
        call(&mut h, "spelling.issues", json!({"document":doc}))["outcome"]["issues"][0].clone();
    assert!(!search.is_null() && !issue.is_null());
    let render =
        call(&mut h, "render.compile", json!({"document":doc}))["outcome"]["render"]["reference"]
            .clone();
    assert!(!render.is_null());
    for tool in catalogue::tools() {
        let schema = json!(tool.input_schema);
        let valid = validator(&schema);
        assert!(
            !valid.is_valid(&json!({"unexpected":true})),
            "{}",
            tool.name
        );
        let docs =
            call(&mut h, "capabilities.describe", json!({"id":tool.name}))["outcome"].clone();
        assert_eq!(docs["mcp_input_schema"], schema);
        assert!(!docs["examples"].as_array().unwrap().is_empty());
        for example in docs["examples"].as_array().unwrap() {
            let mut params = example["params"].clone();
            for binding in example["bindings"].as_array().unwrap() {
                let value = match binding["reference"].as_str().unwrap() {
                    "image_block" => image_target.clone(),
                    "search_occurrence" => search.clone(),
                    "spelling_issue" => issue.clone(),
                    "render" => render.clone(),
                    other => panic!("{other}"),
                };
                *params
                    .pointer_mut(binding["parameter"].as_str().unwrap())
                    .unwrap() = value;
            }
            if !schema["properties"]["document"].is_null() {
                params["document"] = doc.clone();
            }
            let errors: Vec<_> = valid.iter_errors(&params).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "{}: {errors:?}; {params}", tool.name);
        }
    }
    // Métadonnées : un sélecteur, même vide, n’est pas un paramètre de cette commande.
    assert!(
        h.dispatch(
            "metadata.set",
            json!({"document":doc,"field":"letter.subject","value":"Non","selector":{}})
                .as_object()
                .unwrap()
                .clone()
        )
        .is_err()
    );
}

#[test]
fn selector_alternatives_and_automatic_render_are_advertised_by_the_effective_schemas() {
    let doc = json!({"id":"example","guard":{"document_id":1,"revision":1}});
    let schema = |name: &str| {
        json!(
            catalogue::tools()
                .iter()
                .find(|t| t.name == name)
                .unwrap()
                .input_schema
        )
    };
    for (id, params) in [
        (
            "image.remove",
            json!({"document":doc,"selector":{"block":1}}),
        ),
        (
            "image.set_alt",
            json!({"document":doc,"selector":{"block":1},"alt":"Description"}),
        ),
        (
            "spelling.correct",
            json!({"document":doc,"selector":{"select":"bonjor"},"replacement":"bonjour"}),
        ),
        (
            "edit.replace_current",
            json!({"document":doc,"selector":{"select":"Élodie"},"query":"Élodie","replacement":"Camille"}),
        ),
        (
            "render.locate",
            json!({"document":doc,"position":{"direction":"to_preview"}}),
        ),
        ("render.selection_boxes", json!({"document":doc})),
    ] {
        assert!(validator(&schema(id)).is_valid(&params), "{id}");
    }
    assert!(!validator(&schema("image.remove")).is_valid(&json!({"document":doc})));
    assert!(
        !validator(&schema("text.bold")).is_valid(
            &json!({"document":doc,"selector":{"select":"Élodie"},"target":{"kind":"all"}})
        )
    );
    assert!(
        !validator(&schema("metadata.set")).is_valid(
            &json!({"document":doc,"field":"letter.subject","value":"Non","selector":{}})
        )
    );
}

#[test]
fn resources_are_current_coherent_snapshots_with_no_document_mutations() {
    let mut h = host::Host::new(false);
    let empty = resource(&h, "typstographe://document");
    assert_eq!(empty["outcome"]["state"]["open"], false);
    for key in [
        "document",
        "outcome/text",
        "outcome/structure",
        "outcome/metadata",
        "outcome/source",
    ] {
        assert!(empty.pointer(&format!("/{key}")).unwrap().is_null());
    }
    let models = resource(&h, "typstographe://models");
    assert!(models["outcome"]["active_document"].is_null());
    assert_eq!(
        models["outcome"]["types"].as_array().unwrap().len(),
        typstographe_core::document_types::DocumentType::ALL.len()
    );
    let doc = created(&mut h);
    let edited = call(
        &mut h,
        "text.insert",
        json!({"document":doc,"text":"Élodie"}),
    );
    let before = call(&mut h, "document.inspect", json!({}));
    for item in resources::list() {
        let value = resource(&h, &item.uri);
        assert_eq!(value["document"], edited["document"]);
        assert_eq!(value["permissions"], before["permissions"]);
    }
    assert_eq!(call(&mut h, "document.inspect", json!({})), before);
    let value = resource(&h, "typstographe://document");
    assert_eq!(value["outcome"]["text"]["text"], "Élodie");
    for part in ["text", "structure", "metadata", "source", "state"] {
        assert_eq!(value["outcome"][part]["guard"], edited["document"]["guard"]);
    }
    let models = resource(&h, "typstographe://models");
    let card = &models["outcome"]["active_document"]["models"][0];
    assert_eq!(card["active"], true);
    assert!(card["source"].is_null() && card["fixed_page"].is_null());
    let undone = call(&mut h, "edit.undo", json!({"document":edited["document"]}));
    assert_eq!(
        resource(&h, "typstographe://document")["outcome"]["text"]["text"],
        ""
    );
    call(
        &mut h,
        "document.close",
        json!({"document":undone["document"],"discard_changes":true}),
    );
    assert!(resource(&h, "typstographe://document")["document"].is_null());
    assert!(h.resource("file:///etc/passwd").is_err());
    assert!(
        h.resource("typstographe://document?path=/etc/passwd")
            .is_err()
    );
}

#[test]
fn resource_catalogue_and_tools_share_docs_schemas_and_read_only_authorization() {
    let h = host::Host::new(true);
    let docs = resource(&h, "typstographe://commands");
    let entries = docs["outcome"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), core_catalogue().len() + 3);
    for entry in entries {
        let id = entry["command"]["id"].as_str().unwrap();
        let tool = catalogue::tools().iter().find(|t| t.name == id);
        assert_eq!(entry["prototype_support"]["exposed"], tool.is_some());
        assert_eq!(
            entry["mcp_input_schema"],
            tool.map(|t| json!(t.input_schema)).unwrap_or(Value::Null)
        );
        assert!(entry["defaults"].is_object());
        assert!(!entry["examples"].as_array().unwrap().is_empty());
        if id == "text.insert" || id == "document.export" {
            assert_eq!(entry["prototype_support"]["authorized"], false);
        }
        if id == "document.inspect" {
            assert_eq!(entry["prototype_support"]["authorized"], true);
        }
        if core_catalogue().iter().any(|s| s.id.as_str() == id) {
            assert_eq!(
                entry["command"],
                Engine::command_documentation(id).unwrap()["command"]
            );
        }
    }
    assert!(
        docs["outcome"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| !t["description"].is_null())
    );
}

#[test]
fn article_discovery_and_collection_edits_use_the_same_typed_contract() {
    let mut h = host::Host::new(false);
    let description = call(
        &mut h,
        "document_types.describe",
        json!({"document_type":"article"}),
    );
    let collections = description["outcome"]["structured_metadata"]["collections"]
        .as_array()
        .unwrap();
    assert_eq!(collections.len(), 3);
    let affiliations = collections
        .iter()
        .find(|c| c["parameter"] == "affiliations")
        .unwrap();
    assert_eq!(affiliations["fields"].as_array().unwrap().len(), 1);
    assert_eq!(affiliations["fields"][0]["key"], "text");
    let created = call(
        &mut h,
        "document.new",
        json!({"type":"article","model":"article.model.working"}),
    );
    let mut doc = created["document"].clone();
    let incomplete = call(&mut h, "document.validate", json!({"document":doc}));
    assert_eq!(incomplete["outcome"]["valid"], true);
    assert_eq!(incomplete["outcome"]["ready_for_export"], false);
    assert_eq!(
        incomplete["outcome"]["diagnostics"][0]["field"],
        "article.localizations"
    );
    for (field, value) in [
        (
            "article.affiliations",
            json!([{"id":"irmar","text":"Univ Rennes, CNRS, IRMAR - UMR 6625, F-35000 Rennes, France"}]),
        ),
        (
            "article.authors",
            json!([{"id":"a1","name":"Camille","affiliation-ids":["irmar"],"email":"","orcid":"","corresponding":false}]),
        ),
        (
            "article.localizations",
            json!([{"id":"fr","language":"fr","title":{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Article scientifique"}]}]},"short-title":{"type":"doc","content":[{"type":"paragraph"}]},"subtitle":{"type":"doc","content":[{"type":"paragraph"}]},"abstract":{"type":"doc","content":[{"type":"paragraph"}]},"keywords":["recherche SHS"]}]),
        ),
    ] {
        let result = call(
            &mut h,
            "metadata.set",
            json!({"document":doc,"field":field,"value":value}),
        );
        doc = result["document"].clone();
    }
    let complete = call(&mut h, "document.validate", json!({"document":doc}));
    assert_eq!(complete["outcome"]["ready_for_export"], true);
    assert_eq!(complete["outcome"]["diagnostics"], json!([]));
    let schema = json!(
        catalogue::tools()
            .iter()
            .find(|t| t.name == "metadata.set")
            .unwrap()
            .input_schema
    );
    assert!(
        validator(&schema).is_valid(&json!({"document":doc,"field":"article.authors","value":[]}))
    );
    assert!(
        !validator(&schema)
            .is_valid(&json!({"document":doc,"field":"article.authors","value":true}))
    );
}
