#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

#[test]
fn mcp_language_is_chosen_before_clap_initializes_localized_help() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(45), async {
            for (args, word) in [
                (vec![], "completed"),
                (vec!["--json"], "completed"),
                (vec!["--lang", "fr"], "terminé"),
                (vec!["--lang=en"], "completed"),
            ] {
                let dir = tempfile::tempdir().unwrap();
                let client = start(dir.path(), ProtocolVersion::default(), &args).await;
                let reply = call(&client, "document.inspect", json!({})).await;
                assert!(reply["summary"].as_str().unwrap().contains(word));
                client.cancel().await.unwrap();
            }
        })
        .await
        .unwrap();
    });
}

#[test]
fn official_sdk_clients_discover_edit_validate_undo_and_render_a_letter_over_stdio() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(90), async {
            for version in [ProtocolVersion::V_2025_11_25, ProtocolVersion::default()] {
                let dir = tempfile::tempdir().unwrap();
                let client = start(dir.path(), version, &[]).await;
                let tools = client.list_all_tools().await.unwrap();
                assert!(tools.iter().any(|t| t.name == "document_types.describe"));
                assert!(tools.iter().any(|t| t.name == "text.bold"));
                assert!(tools.iter().any(|t| t.name == "document.save"));
                let types = call(&client, "document_types.list", json!({})).await;
                let letter = types["outcome"]["types"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["id"] == "courrier")
                    .unwrap();
                let model = letter["models"][0]["id"].clone();
                let kind = call(
                    &client,
                    "document_types.describe",
                    json!({"document_type":"courrier"}),
                )
                .await;
                assert!(
                    kind["outcome"]["fields"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|f| f["field"] == "recipient.address")
                );
                let docs = call(
                    &client,
                    "capabilities.describe",
                    json!({"id":"text.insert"}),
                )
                .await;
                assert!(!docs["outcome"]["examples"].as_array().unwrap().is_empty());
                let resources = client.list_all_resources().await.unwrap();
                assert!(resources.iter().any(|r| r.uri == "typstographe://guide"));
                let guide = client
                    .read_resource(ReadResourceRequestParams::new("typstographe://guide"))
                    .await
                    .unwrap();
                assert!(
                    serde_json::to_string(&guide)
                        .unwrap()
                        .contains("autonomous")
                );
                let mut new: Value = serde_json::from_str(include_str!(
                    "../../../fixtures/capture-letter-params.json"
                ))
                .unwrap();
                new["type"] = json!("courrier");
                new["model"] = model;
                let created = call(&client, "document.new", new).await;
                let doc = created["document"].clone();
                let edited=call(&client,"document.batch",json!({"document":doc,"steps":[
                {"id":"metadata.set","params":{"field":"letter.subject","value":"Essai MCP"}},
                {"id":"text.insert","params":{"text":"Bonjour Élodie 日本.\n\nVoici le courrier."}},
                {"id":"text.bold","params":{"value":"on","selector":{"select":"Élodie"}}},
                {"id":"letter.insert_postscript","params":{}},
                {"id":"text.insert","params":{"text":"Une précision utile."}},
                {"id":"letter.insert_attachment","params":{}},
                {"id":"text.insert","params":{"text":"Formulaire signé"}}
            ]})).await;
                let doc = edited["document"].clone();
                let validation = call(&client, "document.validate", json!({"document":doc})).await;
                assert_eq!(validation["outcome"]["valid"], true);
                assert_eq!(validation["outcome"]["ready_for_export"], true);
                let source = call(&client, "document.source", json!({"document":doc})).await;
                let parsed = typstographe_core::document::Document::from_source(
                    source["outcome"]["source"].as_str().unwrap(),
                )
                .unwrap();
                let at = parsed.view.text.find("Élodie").unwrap();
                let start = parsed.view.text[..at].chars().count();
                assert!(parsed.view.styles[start..start + 6].iter().all(|s| s.bold));
                assert!(parsed.view.text.contains("Formulaire signé"));
                let rendered = result(
                    &client,
                    "render.page",
                    json!({"document":doc,"page":1}),
                    true,
                )
                .await;
                let image = rendered.content.iter().find_map(|c| c.as_image()).unwrap();
                use base64::Engine;
                let png = base64::prelude::BASE64_STANDARD
                    .decode(&image.data)
                    .unwrap();
                assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
                assert!(
                    typstographe_core::assets::decode(&png, "png")
                        .unwrap()
                        .width()
                        > 100.0
                );
                let undone = call(&client, "edit.undo", json!({"document":doc})).await;
                assert_eq!(
                    call(
                        &client,
                        "document.text",
                        json!({"document":undone["document"]})
                    )
                    .await["outcome"]["text"],
                    ""
                );
                let redone =
                    call(&client, "edit.redo", json!({"document":undone["document"]})).await;
                assert_eq!(
                    call(
                        &client,
                        "document.source",
                        json!({"document":redone["document"]})
                    )
                    .await["outcome"]["source"],
                    source["outcome"]["source"]
                );
                assert_eq!(client.list_all_tools().await.unwrap(), tools);
                client.cancel().await.unwrap();
                assert_eq!(
                    std::fs::read_to_string(dir.path().join("stderr.log")).unwrap(),
                    ""
                );
                assert_eq!(
                    std::fs::read_dir(dir.path()).unwrap().count(),
                    1,
                    "server must not write document files"
                );
            }
        })
        .await
        .unwrap();
    });
}

#[test]
fn stdio_errors_preserve_the_document_and_read_only_mode_is_enforced() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(45), async {
            let dir = tempfile::tempdir().unwrap();
            let client = start(dir.path(), ProtocolVersion::V_2025_11_25, &["--json"]).await;
            let created = call(
                &client,
                "document.new",
                json!({"type":"courrier","model":"courrier.model.personal"}),
            )
            .await;
            let doc = created["document"].clone();
            let incomplete = call(&client, "document.validate", json!({"document":doc})).await;
            assert_eq!(incomplete["outcome"]["valid"], true);
            assert_eq!(incomplete["outcome"]["ready_for_export"], false);
            let failure = result(
                &client,
                "document.batch",
                json!({"document":doc,"steps":[
                    {"id":"text.insert","params":{"text":"Non publié"}},
                    {"id":"insert.image","params":{"image":{"kind":"path","path":"../secret.svg"}}}
                ]}),
                false,
            )
            .await
            .structured_content
            .unwrap();
            assert_eq!(failure["error"]["params"]["step"], 1);
            assert_eq!(
                call(&client, "document.text", json!({"document":doc})).await["outcome"]["text"],
                ""
            );
            let changed = call(
                &client,
                "text.insert",
                json!({"document":doc,"text":"Texte conservé"}),
            )
            .await;
            let stale = result(
                &client,
                "text.insert",
                json!({"document":doc,"text":"Non"}),
                false,
            )
            .await
            .structured_content
            .unwrap();
            assert_eq!(stale["error"]["params"]["reason"], "stale_revision");
            let invalid = result(
                &client,
                "text.insert",
                json!({"document":changed["document"],"text":"Non","unexpected":1}),
                false,
            )
            .await;
            assert!(
                invalid.structured_content.unwrap()["error"]["params"]["reason"]
                    == "invalid_params"
            );
            result(
                &client,
                "document.open",
                json!({"document":changed["document"],"path":"anything.typsto"}),
                false,
            )
            .await;
            client.cancel().await.unwrap();
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
            let readonly = start(
                dir.path(),
                ProtocolVersion::default(),
                &["--read-only", "--lang", "fr"],
            )
            .await;
            let denied = result(
                &readonly,
                "document.new",
                json!({"type":"courrier","model":"courrier.model.personal"}),
                false,
            )
            .await
            .structured_content
            .unwrap();
            assert_eq!(denied["error"]["params"]["reason"], "mcp_read_only");
            assert!(
                denied["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("lecture seule")
            );
            readonly.cancel().await.unwrap();
        })
        .await
        .unwrap();
    });
}

#[test]
fn concurrent_calls_with_the_same_revision_cannot_both_modify_the_document() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(45), async {
            let dir = tempfile::tempdir().unwrap();
            let client = Arc::new(start(dir.path(), ProtocolVersion::default(), &[]).await);
            let created = call(
                &client,
                "document.new",
                json!({"type":"courrier","model":"courrier.model.personal"}),
            )
            .await;
            let mut handles = Vec::new();
            for text in ["Premier", "Second"] {
                let client = client.clone();
                let doc = created["document"].clone();
                handles.push(tokio::spawn(async move {
                    client
                        .call_tool(
                            CallToolRequestParams::new("text.insert").with_arguments(
                                json!({"document":doc,"text":text})
                                    .as_object()
                                    .unwrap()
                                    .clone(),
                            ),
                        )
                        .await
                        .unwrap()
                }));
            }
            let mut successes = 0;
            for handle in handles {
                if !handle.await.unwrap().is_error.unwrap_or(false) {
                    successes += 1;
                }
            }
            assert_eq!(successes, 1);
            let state = call(&client, "document.inspect", json!({})).await;
            assert_eq!(
                state["document"]["guard"]["revision"].as_u64().unwrap(),
                created["document"]["guard"]["revision"].as_u64().unwrap() + 1
            );
            match Arc::try_unwrap(client) {
                Ok(client) => {
                    client.cancel().await.unwrap();
                }
                Err(_) => panic!("client still shared"),
            };
        })
        .await
        .unwrap();
    });
}
