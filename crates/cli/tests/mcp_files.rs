#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::json;
#[test]
fn sdk_file_roundtrip_obeys_independent_roots_locks_and_atomic_final_save() {
    runtime().block_on(async {
        let d=tempfile::tempdir().unwrap();let root=d.path().to_str().unwrap();let outside=tempfile::tempdir().unwrap();
        let c=start(d.path(),ProtocolVersion::default(),&["--read-root",root,"--write-root",root]).await;
        let r=call(&c,"document.new",json!({"type":"courrier"})).await;let doc=r["document"].clone();
        let step=json!({"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Sauvegardé"}});
        result(&c,"document.batch",json!({"document":doc,"steps":[step],"save":{"kind":"as","path":outside.path().join("denied.typsto")}}),false).await;
        assert_eq!(call(&c,"document.inspect",json!({})).await["document"],doc);
        let r=call(&c,"document.batch",json!({"document":doc,"steps":[step],"save":{"kind":"as","path":"letter.typsto"}})).await;let doc=r["document"].clone();
        assert!(d.path().join("letter.typsto").is_file());
        let other=start(d.path(),ProtocolVersion::default(),&["--read-root",root,"--write-root",root]).await;
        result(&other,"document.open",json!({"path":"letter.typsto"}),false).await;
        let read=call(&other,"document.open",json!({"path":"letter.typsto","read_only":true})).await;
        assert_eq!(call(&other,"document.text",json!({"document":read["document"]})).await["outcome"]["text"],"Sauvegardé\n");
        call(&other,"document.save_as",json!({"document":read["document"],"path":"copy.typsto"})).await;
        let before=std::fs::read(d.path().join("letter.typsto")).unwrap();
        result(&c,"document.batch",json!({"document":doc,"steps":[step],"save":{"kind":"as","path":"copy.typsto"}}),false).await;
        assert_eq!(std::fs::read(d.path().join("letter.typsto")).unwrap(),before);
        result(&c,"document.export",json!({"document":doc,"directory":"../escaped","format":"pdf"}),false).await;
        call(&c,"document.close",json!({"document":doc})).await;
        assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
        other.cancel().await.unwrap();c.cancel().await.unwrap();
    });
}
#[test]
fn missing_roots_fail_before_start_and_create_nothing() {
    let d = tempfile::tempdir().unwrap();
    let missing = d.path().join("missing");
    assert_cmd::cargo::cargo_bin_cmd!("typstographe-cli")
        .args(["mcp", "--write-root"])
        .arg(&missing)
        .assert()
        .failure();
    assert!(!missing.exists());
}

#[test]
fn sdk_versions_keep_reference_guards_and_require_explicit_file_write_authority() {
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap(); let root = dir.path().to_str().unwrap();
        let client = start(dir.path(),ProtocolVersion::default(),&["--read-root",root,"--write-root",root]).await;
        let created = call(&client,"document.new",json!({"type":"article"})).await;
        let saved = call(&client,"document.save_as",json!({"document":created["document"],"path":"article.typsto"})).await;
        let document = saved["document"].clone();
        let index = call(&client,"versions.list",json!({"document":document})).await;
        let change = json!({"document":document,"history":index["outcome"]["history"],"action":{"kind":"create","details":{"name":"MCP version"}}});
        let version = call(&client,"versions.change",change.clone()).await;
        assert_eq!(version["document"],document);
        let id = version["outcome"]["versions"][0]["id"].clone();
        let state = call(&client,"versions.read",json!({"document":document,"id":id})).await;
        assert_eq!(state["outcome"]["state"]["settings"]["metadata"]["type"],"article");
        let bytes = std::fs::read(dir.path().join("article.typsto")).unwrap();
        result(&client,"versions.change",change,false).await;
        let reader = start(dir.path(),ProtocolVersion::default(),&["--read-only","--read-root",root]).await;
        let opened = call(&reader,"document.open",json!({"path":"article.typsto","read_only":true})).await;
        call(&reader,"versions.read",json!({"document":opened["document"],"id":id})).await;
        result(&reader,"versions.change",json!({"document":opened["document"],"history":version["outcome"]["history"],"action":{"kind":"delete","id":id}}),false).await;
        assert_eq!(bytes,std::fs::read(dir.path().join("article.typsto")).unwrap());
        reader.cancel().await.unwrap(); client.cancel().await.unwrap();
    });
}
