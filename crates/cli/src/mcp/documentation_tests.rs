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
    assert_eq!(models["outcome"]["types"].as_array().unwrap().len(), 2);
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
