#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::{Value, json};
async fn read(c: &Client, uri: &str) -> Value {
    let r = c
        .read_resource(ReadResourceRequestParams::new(uri))
        .await
        .unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &r.contents[0] else {
        panic!()
    };
    serde_json::from_str(text).unwrap()
}
#[test]
fn virtual_resources_follow_confirmed_state_history_and_lifecycle() {
    runtime().block_on(async {
        let d = tempfile::tempdir().unwrap();
        let c = start(d.path(), ProtocolVersion::default(), &[]).await;
        assert_eq!(c.list_all_resources().await.unwrap().len(), 6);
        assert_eq!(
            read(&c, "typstographe://document").await["outcome"]["open"],
            false
        );
        let catalog = read(&c, "typstographe://commands").await;
        let tools = c.list_all_tools().await.unwrap();
        for tool in tools {
            assert!(
                catalog["outcome"]["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|s| s["id"] == tool.name.as_ref())
            );
        }
        let r = call(&c, "document.new", json!({"type":"article"})).await;
        let snapshot = read(&c, "typstographe://document").await;
        assert_eq!(snapshot["document"], r["document"]);
        assert_eq!(
            snapshot["outcome"]["inspect"]["state"]["settings"]["metadata"]["type"],
            "article"
        );
        assert!(
            read(&c, "typstographe://guide")
                .await
                .to_string()
                .contains("UTF-16")
        );
        call(
            &c,
            "document.close",
            json!({"document":r["document"],"discard_changes":true}),
        )
        .await;
        assert_eq!(
            read(&c, "typstographe://document").await["outcome"]["open"],
            false
        );
        assert!(
            c.read_resource(ReadResourceRequestParams::new("file:///etc/passwd"))
                .await
                .is_err()
        );
        c.cancel().await.unwrap();
    });
}
