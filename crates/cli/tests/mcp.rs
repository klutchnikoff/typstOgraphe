#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::json;

#[test]
fn bibliography_preparation_through_mcp_preserves_the_guard_and_active_project() {
    runtime().block_on(async {
        let folder=tempfile::tempdir().unwrap();let client=start(folder.path(),ProtocolVersion::default(),&[]).await;
        let created=call(&client,"document.new",json!({"type":"article"})).await;
        let document=created["document"].clone();
        let before=call(&client,"document.inspect",json!({"document":document})).await;
        for (operation,params) in [
            ("bibliography.profiles",json!({})),
            ("bibliography.prepare_record",json!({"id":"draft","profile":"book","entry":{"title":"A book","author":"Doe, Jane"}})),
            ("bibliography.prepare_import",json!({"id":"source","name":"in.yaml","format":"hayagriva","contents":"example: {type: book, title: Another book}"}))
        ] {
            let mut arguments=params;arguments["document"]=document.clone();
            let prepared=call(&client,operation,arguments).await;
            assert_eq!(prepared["document"],document);
            if operation!="bibliography.profiles" { assert_eq!(prepared["outcome"]["active"],false); }
        }
        assert_eq!(call(&client,"document.inspect",json!({"document":document})).await,before);
        assert_eq!(call(&client,"bibliography.search",json!({"document":document,"query":""})).await["outcome"]["entries"],json!([]));
        client.cancel().await.unwrap();
    });
}
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

#[test]
fn mcp_imports_native_notices_with_the_same_library_guards_and_identity_citations() {
    runtime().block_on(async {
        let d=tempfile::tempdir().unwrap();let c=start(d.path(),ProtocolVersion::default(),&[]).await;
        let opened=call(&c,"document.new",json!({"type":"article"})).await;
        let source=json!({"id":"source","name":"books.yaml","format":"hayagriva","contents":"example: {type: book, title: A book, author: 'Doe, Jane', date: 2026}"});
        let mut params=source.clone();params["document"]=opened["document"].clone();
        let prepared=call(&c,"bibliography.prepare_import",params).await;
        let report=&prepared["outcome"];
        let imported=call(&c,"bibliography.apply",json!({"document":prepared["document"],"expected":report["library_fingerprint"],"change":{"kind":"import","source":source,"preparation":report["fingerprint"],"decisions":[]}})).await;
        let inspected=call(&c,"bibliography.inspect",json!({"document":imported["document"]})).await;
        assert_eq!(inspected["outcome"]["records"][0]["id"],report["records"][0]["id"]);
        let cited=call(&c,"inline.replace",json!({"document":inspected["document"],"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"node":{"type":"citation","attrs":{"items":[{"record_id":report["records"][0]["id"]}],"form":"normal"}}})).await;
        let rendered=call(&c,"render.compile",json!({"document":cited["document"]})).await;
        assert!(!rendered["outcome"].is_null());
        c.cancel().await.unwrap();
    });
}
