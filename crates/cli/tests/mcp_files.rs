#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

fn args(read: &Path, write: Option<&Path>) -> Vec<String> {
    let mut args = vec!["--read-root".into(), read.to_str().unwrap().into()];
    if let Some(write) = write {
        args.extend(["--write-root".into(), write.to_str().unwrap().into()]);
    }
    args
}
fn workspace(root: &Path, name: &str) -> std::path::PathBuf {
    let path = root.join(name);
    std::fs::create_dir(&path).unwrap();
    path
}
fn initial() -> Value {
    let mut params: Value =
        serde_json::from_str(include_str!("../../../fixtures/capture-letter-params.json")).unwrap();
    params["type"] = json!("courrier");
    params["model"] = json!("courrier.model.personal");
    params
}

#[test]
fn sdk_clients_save_reopen_lock_copy_and_export_an_autonomous_letter() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(90), roundtrip())
            .await
            .unwrap();
    });
}
async fn roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let files = workspace(dir.path(), "documents");
    let copies = workspace(dir.path(), "copies");
    let working = workspace(dir.path(), "server-1");
    let second = workspace(dir.path(), "server-2");
    let arguments = args(&files, Some(&files));
    let refs: Vec<_> = arguments.iter().map(String::as_str).collect();
    let client = start(&working, ProtocolVersion::default(), &refs).await;
    let inspection = call(&client, "document.inspect", json!({})).await;
    assert_eq!(
        inspection["permissions"]["read_roots"][0],
        json!(files.canonicalize().unwrap())
    );
    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 67);
    for id in [
        "document.open",
        "document.save",
        "document.save_as",
        "document.export",
        "document.set_model",
    ] {
        assert!(tools.iter().any(|t| t.name == id));
        let description = call(&client, "capabilities.describe", json!({"id":id})).await;
        assert_eq!(
            description["outcome"]["prototype_support"]["authorized"],
            true
        );
    }
    let new = call(&client, "document.new", initial()).await;
    let path = files.join("letter.typsto");
    let saved = call(
        &client,
        "document.batch",
        json!({"document":new["document"],"steps":[
            {"id":"text.insert","params":{"text":"Bonjour Élodie.\n\nMerci pour votre retour."}},
            {"id":"text.bold","params":{"selector":{"select":"Élodie"},"value":"on"}}
        ],"save":{"kind":"as","path":path}}),
    )
    .await;
    let disk = typstographe_core::storage::read(&path).unwrap();
    assert!(disk.source.contains("Élodie"));
    assert!(!saved["summary"].as_str().unwrap().contains("no file"));
    let saved_bytes = std::fs::read(&path).unwrap();
    let edited = call(
        &client,
        "text.insert",
        json!({"document":saved["document"],"selector":{"select":"Élodie"},"text":"Camille"}),
    )
    .await;
    assert_eq!(std::fs::read(&path).unwrap(), saved_bytes);
    let arguments = args(&files, Some(&copies));
    let refs: Vec<_> = arguments.iter().map(String::as_str).collect();
    let peer = start(&second, ProtocolVersion::V_2025_11_25, &refs).await;
    // Une racine de lecture seule ne suffit pas pour créer le compagnon de verrou.
    let denied = result(&peer, "document.open", json!({"path":path}), false).await;
    assert_eq!(
        denied.structured_content.unwrap()["error"]["params"]["reason"],
        "mcp_file_access"
    );
    let reader = call(
        &peer,
        "document.open",
        json!({"path":path,"read_only":true}),
    )
    .await;
    let snapshot = call(
        &peer,
        "document.text",
        json!({"document":reader["document"]}),
    )
    .await;
    assert!(
        snapshot["outcome"]["text"]
            .as_str()
            .unwrap()
            .contains("Élodie")
    );
    let copied = call(
        &peer,
        "document.save_as",
        json!({"document":reader["document"],"path":copies.join("copy.typsto")}),
    )
    .await;
    let text = call(
        &peer,
        "text.insert",
        json!({"document":copied["document"],"text":"Copie indépendante.\n\n"}),
    )
    .await;
    call(&peer, "document.save", json!({"document":text["document"]})).await;
    peer.cancel().await.unwrap();
    // Autre processus avec les deux racines : conflit sur le verrou du moteur.
    let arguments = args(&files, Some(&files));
    let refs: Vec<_> = arguments.iter().map(String::as_str).collect();
    let peer = start(&second, ProtocolVersion::default(), &refs).await;
    let locked = result(&peer, "document.open", json!({"path":path}), false).await;
    assert_eq!(
        locked.structured_content.unwrap()["error"]["params"]["reason"],
        "file_locked"
    );
    call(
        &client,
        "document.save",
        json!({"document":edited["document"]}),
    )
    .await;
    let export = files.join("export");
    call(&client,"document.export",json!({"document":edited["document"],"directory":export,"format":"both","page_images":true})).await;
    assert!(
        std::fs::read(export.join("courrier.pdf"))
            .unwrap()
            .starts_with(b"%PDF")
    );
    assert!(export.join("main.typ").exists());
    assert!(disk.dependencies.keys().any(|p| p.contains("datify")));
    for (name, data) in disk.dependencies.iter() {
        assert_eq!(std::fs::read(export.join(name)).unwrap(), data.as_ref());
    }
    let png = std::fs::read(export.join("page-1.png")).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    call(
        &client,
        "document.close",
        json!({"document":edited["document"]}),
    )
    .await;
    let reopened = call(&peer, "document.open", json!({"path":path})).await;
    let source = call(
        &peer,
        "document.source",
        json!({"document":reopened["document"]}),
    )
    .await;
    assert!(
        source["outcome"]["source"]
            .as_str()
            .unwrap()
            .contains("Camille")
    );
    assert_ne!(reopened["document"]["id"], edited["document"]["id"]);
    assert_eq!(client.list_all_tools().await.unwrap(), tools);
    peer.cancel().await.unwrap();
    client.cancel().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(working.join("stderr.log")).unwrap(),
        ""
    );
    assert_eq!(
        std::fs::read_to_string(second.join("stderr.log")).unwrap(),
        ""
    );
}

#[test]
fn protocol_enforces_roots_and_read_only_and_preserves_state_on_late_failure() {
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(90), confinement())
            .await
            .unwrap();
    });
}
async fn confinement() {
    let dir = tempfile::tempdir().unwrap();
    let root = workspace(dir.path(), "allowed");
    let working = workspace(dir.path(), "server");
    let arguments = args(&root, Some(&root));
    let refs: Vec<_> = arguments.iter().map(String::as_str).collect();
    let client = start(&working, ProtocolVersion::default(), &refs).await;
    let doc = call(&client, "document.new", initial()).await["document"].clone();
    let original = call(&client, "document.inspect", json!({})).await;
    for args in [
        json!({"document":doc,"steps":[{"id":"text.insert","params":{"text":"No publication"}}],"save":{"kind":"as","path":dir.path().join("outside.typsto")}}),
        json!({"document":doc,"steps":[{"id":"text.insert","params":{"text":"No publication"}},{"id":"document.save","params":{"path":root.join("forbidden.typsto")}}]}),
        json!({"document":doc,"steps":[{"id":"text.insert","params":{"text":"No publication"}},{"id":"document.set_signature","params":{"image":{"kind":"path","path":dir.path().join("outside.svg")}}}],"save":{"kind":"as","path":root.join("late.typsto")}}),
    ] {
        result(&client, "document.batch", args, false).await;
        assert_eq!(call(&client, "document.inspect", json!({})).await, original);
    }
    for path in [
        root.join("../escape.typsto"),
        dir.path().join("absolute.typsto"),
    ] {
        let denied = result(
            &client,
            "document.save_as",
            json!({"document":doc,"path":path}),
            false,
        )
        .await;
        assert_eq!(
            denied.structured_content.unwrap()["error"]["params"]["reason"],
            "mcp_file_access"
        );
    }
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    let path = root.join("letter.typsto");
    call(
        &client,
        "document.save_as",
        json!({"document":doc,"path":path}),
    )
    .await;
    client.cancel().await.unwrap();
    let mut arguments = args(&root, Some(&root));
    arguments.push("--read-only".into());
    let refs: Vec<_> = arguments.iter().map(String::as_str).collect();
    let client = start(&working, ProtocolVersion::default(), &refs).await;
    let opened = call(
        &client,
        "document.open",
        json!({"path":path,"read_only":true}),
    )
    .await;
    assert_eq!(opened["permissions"]["write_enabled"], false);
    for (id, args) in [
        ("document.save", json!({})),
        ("document.save_as", json!({"path":root.join("copy.typsto")})),
        ("document.export", json!({"directory":root.join("out")})),
        ("text.insert", json!({"text":"No"})),
    ] {
        let mut args = args;
        args["document"] = opened["document"].clone();
        let denied = result(&client, id, args, false).await;
        assert_eq!(
            denied.structured_content.unwrap()["error"]["params"]["reason"],
            "mcp_read_only"
        );
    }
    result(
        &client,
        "render.page",
        json!({"document":opened["document"],"page":1}),
        true,
    )
    .await;
    call(
        &client,
        "document.close",
        json!({"document":opened["document"]}),
    )
    .await;
    client.cancel().await.unwrap();
    assert!(!root.join("copy.typsto").exists());
    assert!(!root.join("out").exists());
}

#[test]
fn invalid_roots_fail_before_protocol_start_without_creating_directories() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_typstographe-cli"))
        .args(["mcp", "--read-root"])
        .arg(&missing)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert!(!missing.exists());
}
