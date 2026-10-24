#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::{model::*, service::ServiceError};
use serde_json::{Value, json};
use std::time::Duration;

async fn read(client: &Client, uri: &str) -> Value {
    let result = client
        .read_resource(ReadResourceRequestParams::new(uri.to_owned()))
        .await
        .unwrap();
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["contents"][0]["mimeType"], "application/json");
    assert_eq!(result["contents"][0]["uri"], uri);
    serde_json::from_str(result["contents"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn sdk_clients_read_coherent_resources_through_document_lifecycle_and_history() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(90), lifecycle())
            .await
            .unwrap();
    });
}
async fn lifecycle() {
    for version in [ProtocolVersion::V_2025_11_25, ProtocolVersion::default()] {
        let dir = tempfile::tempdir().unwrap();
        let client = start(dir.path(), version, &[]).await;
        let resources = client.list_all_resources().await.unwrap();
        assert_eq!(resources.len(), 5);
        let empty = read(&client, "typstographe://document").await;
        assert!(empty["document"].is_null());
        assert_eq!(empty["outcome"]["state"]["open"], false);
        let info = client.peer_info().unwrap();
        assert_ne!(
            info.capabilities.resources.as_ref().unwrap().subscribe,
            Some(true)
        );
        assert_ne!(
            info.capabilities.resources.as_ref().unwrap().list_changed,
            Some(true)
        );
        let created = call(
            &client,
            "document.new",
            json!({"type":"courrier","model":"courrier.model.personal"}),
        )
        .await;
        let edited = call(
            &client,
            "document.batch",
            json!({"document":created["document"],"steps":[
                {"id":"text.insert","params":{"text":"Élodie et ses collègues."}},
                {"id":"text.bold","params":{"value":"on","selector":{"select":"Élodie"}}},
                {"id":"metadata.set","params":{"field":"letter.subject","value":"Ressources MCP"}}
            ]}),
        )
        .await;
        let before = call(&client, "document.inspect", json!({})).await;
        let snapshot = read(&client, "typstographe://document").await;
        assert_eq!(snapshot["document"], edited["document"]);
        assert_eq!(
            snapshot["outcome"]["text"]["text"],
            "Élodie et ses collègues."
        );
        for key in ["state", "text", "structure", "metadata", "source"] {
            assert_eq!(
                snapshot["outcome"][key]["guard"],
                edited["document"]["guard"]
            );
        }
        let models = read(&client, "typstographe://models").await;
        let card = &models["outcome"]["active_document"]["models"][0];
        assert_eq!(card["active"], true);
        assert!(card["fixed_page"].is_null());
        assert!(card["source"].is_null());
        assert!(card["description"].is_string() && card["keywords"].is_array());
        for resource in &resources {
            let value = read(&client, &resource.uri).await;
            assert_eq!(value["document"], edited["document"]);
        }
        assert_eq!(call(&client, "document.inspect", json!({})).await, before);
        let undone = call(&client, "edit.undo", json!({"document":edited["document"]})).await;
        assert_eq!(
            read(&client, "typstographe://document").await["outcome"]["text"]["text"],
            ""
        );
        let redone = call(&client, "edit.redo", json!({"document":undone["document"]})).await;
        assert_eq!(
            read(&client, "typstographe://document").await["outcome"]["source"]["source"],
            snapshot["outcome"]["source"]["source"]
        );
        call(
            &client,
            "document.close",
            json!({"document":redone["document"],"discard_changes":true}),
        )
        .await;
        assert!(read(&client, "typstographe://document").await["document"].is_null());
        assert!(
            read(&client, "typstographe://models").await["outcome"]["active_document"].is_null()
        );
        assert_eq!(client.list_all_resources().await.unwrap(), resources);
        for uri in [
            "file:///etc/passwd",
            "typstographe://document?path=secret",
            "typstographe://unknown",
        ] {
            let error = client
                .read_resource(ReadResourceRequestParams::new(uri))
                .await
                .unwrap_err();
            match error {
                ServiceError::McpError(error) => assert!(matches!(
                    error.code,
                    ErrorCode::RESOURCE_NOT_FOUND | ErrorCode::INVALID_PARAMS
                )),
                other => panic!("unexpected resource error: {other}"),
            }
        }
        assert!(
            client
                .list_resources(Some(
                    PaginatedRequestParams::default().with_cursor(Some("unknown".into()))
                ))
                .await
                .is_err()
        );
        client.cancel().await.unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("stderr.log")).unwrap(),
            ""
        );
    }
}

#[test]
fn catalogue_resources_match_tools_and_report_read_only_permissions() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(90), discovery())
            .await
            .unwrap();
    });
}
async fn discovery() {
    let dir = tempfile::tempdir().unwrap();
    let client = start(dir.path(), ProtocolVersion::default(), &["--read-only"]).await;
    let tools = client.list_all_tools().await.unwrap();
    let resource = read(&client, "typstographe://commands").await;
    let entries = resource["outcome"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 71);
    for tool in tools {
        let entry = entries
            .iter()
            .find(|e| e["command"]["id"] == tool.name.as_ref())
            .unwrap();
        let described = call(&client, "capabilities.describe", json!({"id":tool.name})).await;
        assert_eq!(entry, &described["outcome"]);
        assert_eq!(entry["mcp_input_schema"], json!(tool.input_schema));
        let schema = json!(tool.input_schema);
        jsonschema::options().offline().build(&schema).unwrap();
        assert!(!entry["examples"].as_array().unwrap().is_empty());
    }
    let text = entries
        .iter()
        .find(|e| e["command"]["id"] == "text.insert")
        .unwrap();
    assert_eq!(text["prototype_support"]["authorized"], false);
    let recovery = entries
        .iter()
        .find(|e| e["command"]["id"] == "recovery.restore")
        .unwrap();
    assert_eq!(recovery["prototype_support"]["exposed"], false);
    let rejected = result(
        &client,
        "document.new",
        json!({"type":"courrier","model":"courrier.model.personal"}),
        false,
    )
    .await;
    assert_eq!(
        rejected.structured_content.unwrap()["error"]["params"]["reason"],
        "mcp_read_only"
    );
    assert!(read(&client, "typstographe://document").await["document"].is_null());
    client.cancel().await.unwrap();
}
