#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::json;
#[test]
fn sdk_clients_discover_structured_schemas_edit_undo_render_and_reject_stale_references() {
    runtime().block_on(async {
        for version in [ProtocolVersion::V_2025_11_25,ProtocolVersion::default()] {
            let d=tempfile::tempdir().unwrap();let c=start(d.path(),version,&[]).await;
            let tools=c.list_all_tools().await.unwrap();assert!(tools.iter().any(|t|t.name=="record.add"));assert!(!tools.iter().any(|t|t.name=="text.select"));
            for tool in &tools {jsonschema::validator_for(&json!(tool.input_schema)).unwrap();}
            let types=call(&c,"document_types.list",json!({})).await;assert!(types["outcome"]["types"].as_array().unwrap().iter().any(|t|t["id"]=="article"));
            let r=call(&c,"document.new",json!({"type":"courrier"})).await;let doc=r["document"].clone();
            let r=call(&c,"document.batch",json!({"document":doc,"steps":[{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Bonjour 日本😀"}},{"id":"text.bold","params":{"selector":{"select":"日本"},"value":"on"}}]})).await;
            let stale=result(&c,"document.text",json!({"document":doc}),false).await;assert_eq!(stale.structured_content.unwrap()["error"]["code"],"stale_revision");
            let doc=r["document"].clone();let r=call(&c,"document.validate",json!({"document":doc})).await;assert_eq!(r["outcome"]["valid"],true);
            let png=result(&c,"render.page",json!({"document":doc,"page":1}),true).await;assert!(png.content.iter().any(|b|matches!(b,ContentBlock::Image(_))));
            let r=call(&c,"edit.undo",json!({"document":doc})).await;
            assert_eq!(call(&c,"document.text",json!({"document":r["document"]})).await["outcome"]["text"],"");
            c.cancel().await.unwrap();
        }
    });
}
#[test]
fn concurrent_requests_with_one_guard_cannot_both_publish() {
    runtime().block_on(async {
        let d=tempfile::tempdir().unwrap();let c=start(d.path(),ProtocolVersion::default(),&[]).await;
        let r=call(&c,"document.new",json!({"type":"courrier"})).await;
        let args=json!({"document":r["document"],"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"One"});
        let (a,b)=tokio::join!(c.call_tool(CallToolRequestParams::new("text.insert").with_arguments(args.as_object().unwrap().clone())),c.call_tool(CallToolRequestParams::new("text.insert").with_arguments(args.as_object().unwrap().clone())));
        assert_ne!(a.unwrap().is_error.unwrap_or(false),b.unwrap().is_error.unwrap_or(false));
        c.cancel().await.unwrap();
    });
}
#[test]
fn mcp_language_and_read_only_policy_are_explicit() {
    runtime().block_on(async {
        for (args, word) in [
            (vec!["--lang=fr"], "terminé"),
            (vec!["--lang=en"], "completed"),
            (vec!["--json"], "completed"),
        ] {
            let d = tempfile::tempdir().unwrap();
            let c = start(d.path(), ProtocolVersion::default(), &args).await;
            assert!(
                call(&c, "document.inspect", json!({})).await["summary"]
                    .as_str()
                    .unwrap()
                    .contains(word)
            );
            c.cancel().await.unwrap();
        }
        let d = tempfile::tempdir().unwrap();
        let c = start(d.path(), ProtocolVersion::default(), &["--read-only"]).await;
        result(&c, "document.new", json!({"type":"courrier"}), false).await;
        c.cancel().await.unwrap();
    });
}
