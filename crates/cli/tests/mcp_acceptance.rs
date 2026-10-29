#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
//! An SDK client uses only discovery results, not a skill or the repository's type definitions.
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::{Value, json};
#[test]
fn discovery_is_sufficient_to_create_a_letter_and_export_it_offline() {
    runtime().block_on(async {
        let d=tempfile::tempdir().unwrap();let root=d.path().to_str().unwrap();
        let c=start(d.path(),ProtocolVersion::default(),&["--read-root",root,"--write-root",root]).await;
        let description=call(&c,"document_types.describe",json!({"type":"courrier"})).await;
        let model=description["outcome"]["models"][0]["id"].clone();
        let created=call(&c,"document.new",json!({"type":"courrier","model":model})).await;let doc=created["document"].clone();
        let fields=description["outcome"]["fields"].as_array().unwrap();let mut steps:Vec<Value>=fields.iter().filter(|f|f["required_for_export"]==true).map(|f|json!({"id":"metadata.set","params":{"target":{"kind":"metadata","field":f["field"]},"value":if f["field"]=="recipient.address" {"10 rue des Essais, 35000 Rennes"}else{"Camille Exemple"}}})).collect();
        let insertion=call(&c,"capabilities.describe",json!({"id":"text.insert"})).await;
        let mut example=insertion["outcome"]["examples"][0].clone();example["params"]["text"]=json!("Bonjour, merci pour votre réponse.");example["params"]["marks"]=json!([]);steps.push(example);
        let note=call(&c,"capabilities.describe",json!({"id":"insert.footnote"})).await;
        steps.extend(note["outcome"]["examples"].as_array().unwrap().iter().cloned());
        let r=call(&c,"document.batch",json!({"document":doc,"steps":steps,"save":{"kind":"as","path":"courrier.typsto"}})).await;
        let doc=r["document"].clone();assert_eq!(call(&c,"document.validate",json!({"document":doc})).await["outcome"]["ready_for_export"],true);
        call(&c,"document.export",json!({"document":doc,"format":"both","directory":"export"})).await;
        assert!(std::fs::read(d.path().join("export/courrier.pdf")).unwrap().starts_with(b"%PDF"));
        assert!(d.path().join("export/main.typ").is_file());
        call(&c,"document.close",json!({"document":doc})).await;
        let r=call(&c,"document.open",json!({"path":"courrier.typsto","read_only":true})).await;
        assert_eq!(call(&c,"document.validate",json!({"document":r["document"]})).await["outcome"]["valid"],true);
        c.cancel().await.unwrap();
    });
}
