use super::discovery::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use typstographe_core::{
    content::api,
    document_types::{DocumentType, discovery as profiles},
};

#[test]
fn bibliographic_preparation_is_read_only_and_describes_results_in_both_modes() {
    for attached in [false, true] {
        for id in [
            "bibliography.profiles",
            "bibliography.prepare_import",
            "bibliography.prepare_record",
        ] {
            let tool = crate::tools(attached)
                .into_iter()
                .find(|t| t.name == id)
                .unwrap();
            assert_eq!(tool.annotations.unwrap().read_only_hint, Some(true));
            let details = describe(id, attached).unwrap();
            assert_eq!(details["execution"]["direct_tool"], true);
            assert_eq!(details["execution"]["proposal_step"], false);
            assert_eq!(details["result_schema_scope"], "query_outcome");
            jsonschema::validator_for(&details["result_schema"]).unwrap();
            assert!(!details["examples"].as_array().unwrap().is_empty());
            if id != "bibliography.profiles" {
                assert_eq!(
                    details["result_schema"]["properties"]["active"]["const"],
                    false
                );
            }
        }
    }
}

#[test]
fn type_indexes_lead_to_exact_metadata_and_capability_details_without_eager_schemas() {
    let listing = types();
    for profile in &profiles::catalogue().types {
        let description = describe_type(&profile.id).unwrap();
        assert_eq!(description["can_create"], profile.can_create());
        if let Some(kind) = profile.runtime_type {
            let canonical = api::type_description(kind);
            let summary = listing["types"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["id"] == json!(kind))
                .unwrap();
            assert!(summary.get("metadata").is_none());
            assert!(summary.get("definition").is_none());
            assert_eq!(
                description["definition"],
                json!(kind.definition().definition())
            );
            assert_eq!(summary["models"], canonical["models"]);
            assert_eq!(description["metadata"], canonical["metadata"]);
            assert_eq!(description["fields"], canonical["fields"]);
            assert_eq!(
                description["structured_metadata"],
                canonical["structured_metadata"]
            );
            assert_eq!(
                description["capabilities"].as_array().unwrap().len(),
                canonical["capabilities"].as_array().unwrap().len()
            );
            for capability in description["capabilities"].as_array().unwrap() {
                assert!(capability.get("parameters").is_none());
                let id = capability["id"].as_str().unwrap();
                let canonical = api::describe(id).unwrap();
                for attached in [true, false] {
                    let details = describe(id, attached).unwrap();
                    assert_eq!(details["examples"], canonical["examples"]);
                    assert_eq!(details["capability"], canonical["capability"]);
                }
            }
        } else {
            assert_eq!(description["metadata"], Value::Null);
            assert!(description["capabilities"].as_array().unwrap().is_empty());
        }
    }
    assert!(describe_type("missing-type").is_none());
    assert!(
        serde_json::to_vec(&listing).unwrap().len() * 10
            < serde_json::to_vec(&api::types()).unwrap().len()
    );
}

#[test]
fn operation_indexes_match_the_effective_tools_and_explain_proposal_only_operations() {
    for attached in [false, true] {
        let index = commands(attached);
        let definitions = crate::tools(attached);
        let tools: BTreeSet<_> = definitions
            .iter()
            .map(|tool| tool.name.to_string())
            .collect();
        let advertised: BTreeSet<_> = index["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["execution"]["direct_tool"] == true)
            .map(|entry| entry["id"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(advertised, tools);
        for entry in index["entries"].as_array().unwrap() {
            assert!(entry.get("params_schema").is_none());
            let id = entry["id"].as_str().unwrap();
            let details = describe(id, attached).unwrap();
            assert_eq!(details["execution"], entry["execution"]);
            if tools.contains(id) {
                let tool = definitions.iter().find(|tool| tool.name == id).unwrap();
                assert_eq!(details["params_schema"], json!(tool.input_schema));
                jsonschema::validator_for(&details["params_schema"]).unwrap();
            }
        }
        if attached {
            assert!(
                index["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| entry["id"] == "text.insert"
                        && entry["execution"]["direct_tool"] == false
                        && entry["execution"]["proposal_step"] == true)
            );
            assert!(describe("proposals.submit", true).is_some());
            assert!(describe("proposals.accept", true).is_none());
        }
    }
}

#[test]
fn semantic_features_are_not_mislabeled_as_unsupported_operations() {
    let feature = describe("insert.footnote", true).unwrap();
    assert_eq!(feature["id"], "insert.footnote");
    assert!(feature.get("execution").is_none());
    let steps = feature["examples"].as_array().unwrap();
    assert!(!steps.is_empty());
    for step in steps {
        let operation = describe(step["id"].as_str().unwrap(), true).unwrap();
        assert_eq!(operation["execution"]["proposal_step"], true);
    }
}

#[test]
fn schema_pruning_keeps_recursive_transitive_definitions_and_input_validation() {
    let mut recursive = json!({"type":"object","properties":{"node":{"$ref":"#/$defs/Node"}},"$defs":{
        "Node":{"type":"object","properties":{"child":{"$ref":"#/$defs/Node"},"value":{"$ref":"#/$defs/Value"}}},
        "Value":{"type":"string"},"Unused":{"type":"integer"}
    }});
    let before = jsonschema::validator_for(&recursive).unwrap();
    prune_schema(&mut recursive);
    assert!(recursive["$defs"].get("Unused").is_none());
    let after = jsonschema::validator_for(&recursive).unwrap();
    for input in [
        json!({"node":{"value":"valid","child":{"value":"nested"}}}),
        json!({"node":{"value":123}}),
    ] {
        assert_eq!(before.is_valid(&input), after.is_valid(&input));
    }
    for entry in api::catalogue()["entries"].as_array().unwrap() {
        let original = &entry["params_schema"];
        let mut compact = original.clone();
        prune_schema(&mut compact);
        // The common catalogue is already compact; repeated adapter pruning is idempotent.
        assert_eq!(&compact, original);
        let before = jsonschema::validator_for(original).unwrap();
        let after = jsonschema::validator_for(&compact).unwrap();
        for example in api::describe(entry["id"].as_str().unwrap()).unwrap()["examples"]
            .as_array()
            .unwrap()
        {
            if example["id"] != entry["id"] {
                continue;
            }
            let mut input = example["params"].clone();
            assert_eq!(before.is_valid(&input), after.is_valid(&input));
            if let Some(params) = input.as_object_mut() {
                params.insert("unknown_parameter".into(), json!(true));
            }
            assert_eq!(before.is_valid(&input), after.is_valid(&input));
        }
    }
}

#[test]
fn overview_is_type_scoped_and_keeps_approval_rules_visible() {
    let empty = overview(false, None);
    assert_eq!(empty["document_open"], false);
    assert_eq!(empty["capabilities"], Value::Null);
    let attached = overview(true, Some(DocumentType::Article));
    assert_eq!(attached["document_type"], "article");
    assert!(
        attached["rules"]
            .as_str()
            .unwrap()
            .contains("only the human")
    );
    let letter = overview(true, Some(DocumentType::Courrier));
    for id in ["letter.insert_postscript", "letter.insert_attachment"] {
        assert!(
            letter["capabilities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|cap| cap["id"] == id)
        );
        assert!(
            !attached["capabilities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|cap| cap["id"] == id)
        );
    }
    assert!(
        attached["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cap| cap["id"] == "insert.citation")
    );
    assert!(
        !letter["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cap| cap["id"] == "insert.citation")
    );
    assert!(crate::instructions(true).contains("typstographe://overview"));
    assert!(crate::guide(true).contains("UTF-16"));
    assert!(crate::instructions(true).len() < crate::guide(true).len());
}
