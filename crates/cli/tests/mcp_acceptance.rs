#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
//! Rejoue le courrier réellement composé par un agent sans skill le 24 octobre 2026.
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::{Value, json};
use std::time::Duration;

async fn emphasis(client: &Client, document: Value, text: &str, value: &str) -> Value {
    let found = call(
        client,
        "search.find",
        json!({"document":document,"query":text}),
    )
    .await;
    let matches = found["outcome"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    let mut target = matches[0]["target"].clone();
    target["kind"] = json!("selection");
    call(
        client,
        "text.bold",
        json!({"document":found["document"],"target":target,"value":value}),
    )
    .await["document"]
        .clone()
}

#[test]
fn agent_letter_roundtrip_preserves_metadata_emphasis_annotations_and_export() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(120), async {
            for version in [ProtocolVersion::V_2025_11_25, ProtocolVersion::default()] {
                roundtrip(version).await;
            }
        })
        .await
        .unwrap();
    });
}

async fn roundtrip(version: ProtocolVersion) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    let client = start(
        dir.path(),
        version,
        &["--read-root", root, "--write-root", root],
    )
    .await;
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/mcp-agent-letter.json")).unwrap();
    let types = call(&client, "document_types.list", json!({})).await;
    assert!(
        types["outcome"]["types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|kind| { kind["id"] == fixture["new"]["type"] && kind["can_create"] == true })
    );
    let created = call(&client, "document.new", fixture["new"].clone()).await;
    let inserted = call(
        &client,
        "text.insert",
        json!({"document":created["document"],"text":fixture["body"]}),
    )
    .await;
    let mut document = emphasis(
        &client,
        inserted["document"].clone(),
        fixture["date_emphasis"].as_str().unwrap(),
        "on",
    )
    .await;
    for (tool, field) in [
        ("letter.insert_postscript", "postscript"),
        ("letter.insert_attachment", "attachment"),
    ] {
        let inserted = call(&client, tool, json!({"document":document})).await;
        document = call(
            &client,
            "text.insert",
            json!({"document":inserted["document"],"text":fixture[field]}),
        )
        .await["document"]
            .clone();
    }
    // L'agent a constaté l'héritage du style dans l'aperçu et a corrigé
    // ces deux sélections avec les références fournies par search.find.
    for field in ["postscript", "attachment"] {
        document = emphasis(&client, document, fixture[field].as_str().unwrap(), "off").await;
    }
    let validation = call(&client, "document.validate", json!({"document":document})).await;
    assert_eq!(validation["outcome"]["valid"], true);
    assert_eq!(validation["outcome"]["ready_for_export"], true);
    let source = call(&client, "document.source", json!({"document":document})).await;
    let parsed = typstographe_core::document::Document::from_source(
        source["outcome"]["source"].as_str().unwrap(),
    )
    .unwrap();
    for (field, bold) in [
        ("date_emphasis", true),
        ("postscript", false),
        ("attachment", false),
    ] {
        let text = fixture[field].as_str().unwrap();
        let byte = parsed.view.text.find(text).unwrap();
        let start = parsed.view.text[..byte].chars().count();
        assert!(
            parsed.view.styles[start..start + text.chars().count()]
                .iter()
                .all(|s| s.bold == bold)
        );
    }
    let metadata = call(&client, "metadata.get", json!({"document":document})).await;
    let page = result(
        &client,
        "render.page",
        json!({"document":document,"page":1}),
        true,
    )
    .await;
    use base64::Engine;
    let png = base64::prelude::BASE64_STANDARD
        .decode(&page.content.iter().find_map(|c| c.as_image()).unwrap().data)
        .unwrap();
    let archive = dir.path().join("agent.typsto");
    let saved = call(
        &client,
        "document.save",
        json!({"document":document,"path":archive}),
    )
    .await;
    document = saved["document"].clone();
    let export = dir.path().join("export");
    call(
        &client,
        "document.export",
        json!({"document":document,"directory":export,"format":"both","page_images":true}),
    )
    .await;
    assert!(
        std::fs::read(export.join("courrier.pdf"))
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert_eq!(std::fs::read(export.join("page-1.png")).unwrap(), png);
    assert!(export.join("main.typ").is_file() && export.join("dependencies.json").is_file());
    call(&client, "document.close", json!({"document":document})).await;
    let opened = call(&client, "document.open", json!({"path":archive})).await;
    assert_ne!(opened["document"]["id"], document["id"]);
    document = opened["document"].clone();
    let text = call(&client, "document.text", json!({"document":document})).await;
    assert_eq!(text["outcome"]["text"], parsed.view.text);
    let reopened_source = call(&client, "document.source", json!({"document":document})).await;
    assert_eq!(
        reopened_source["outcome"]["source"],
        source["outcome"]["source"]
    );
    let mut reopened = call(&client, "metadata.get", json!({"document":document})).await;
    // La garde appartient à la session, les valeurs documentaires doivent rester identiques.
    reopened["outcome"]["guard"] = metadata["outcome"]["guard"].clone();
    assert_eq!(reopened["outcome"], metadata["outcome"]);
    let inspected = call(&client, "document.inspect", json!({"document":document})).await;
    assert_eq!(inspected["outcome"]["dirty"], false);
    client.cancel().await.unwrap();
}
