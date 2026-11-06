use super::{Call, Session};
use serde_json::{Value, json};
fn article() -> Session {
    let mut s = Session::new();
    s.execute("document.new", json!({"type":"article"}))
        .unwrap();
    s
}
fn library(s: &mut Session) -> Value {
    s.query("bibliography.inspect", json!({})).unwrap()
}
fn change(s: &mut Session, change: Value) -> Result<Value, crate::protocol::CommandError> {
    let base = library(s);
    s.execute(
        "bibliography.apply",
        json!({"expected":base["fingerprint"],"change":change}),
    )
}
fn source(id: &str) -> Value {
    json!({"id":id,"name":"books.bib","format":"biblatex","contents":"@book{martin2026, author={Martin, Camille}, title={An {API} study}, year={2026}, publisher={Press}}"})
}
fn import(s: &mut Session, input: Value) -> Value {
    let prepared = s
        .query("bibliography.prepare_import", input.clone())
        .unwrap();
    change(s,json!({"kind":"import","source":input,"preparation":prepared["fingerprint"],"decisions":[]})).unwrap();
    prepared
}
fn info(s: &mut Session, id: &Value) -> Value {
    s.query("bibliography.record", json!({"id":id})).unwrap()
}
#[test]
fn create_and_cite_is_one_undoable_edit_and_key_changes_preserve_identity() {
    let mut s = article();
    let record = s
        .query(
            "bibliography.prepare_record",
            json!({"id":"book","profile":"book","key":"old","entry":{"title":"An example"}}),
        )
        .unwrap();
    let base = library(&mut s);
    let citation = json!({"type":"citation","attrs":{"items":[{"record_id":"book","supplement":"p. 2"}],"form":"normal"}});
    s.batch(&[
        Call{id:"bibliography.apply".into(),params:json!({"expected":base["fingerprint"],"change":{"kind":"create","record":record["record"]}})},
        Call{id:"inline.replace".into(),params:json!({"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"node":citation})},
    ],None).unwrap();
    let before = s.archive().unwrap();
    assert!(before.validate().unwrap().contains("label(\"old\")"));
    s.execute("edit.undo", json!({})).unwrap();
    assert!(library(&mut s)["records"].as_array().unwrap().is_empty());
    s.execute("edit.redo", json!({})).unwrap();
    assert_eq!(s.archive().unwrap().document, before.document);
    let first_info = info(&mut s, &json!("book"));
    change(
        &mut s,
        json!({"kind":"rename","id":"book","expected":first_info["fingerprint"],"key":"new"}),
    )
    .unwrap();
    let after = s.archive().unwrap();
    assert_eq!(after.document, before.document);
    assert!(after.validate().unwrap().contains("label(\"new\")"));
    let checkpoint = after.yjs;
    assert!(
        change(
            &mut s,
            json!({"kind":"rename","id":"book","expected":first_info["fingerprint"],"key":"stale"})
        )
        .is_err()
    );
    assert_eq!(s.archive().unwrap().yjs, checkpoint);
    let current = info(&mut s, &json!("book"));
    assert!(
        change(
            &mut s,
            json!({"kind":"remove","id":"book","expected":current["fingerprint"]})
        )
        .is_err()
    );
    assert_eq!(s.archive().unwrap().yjs, checkpoint);
}
#[test]
fn imported_originals_and_native_fields_survive_edits_reimport_and_undo() {
    let mut s = article();
    let input = source("original");
    import(&mut s, input.clone());
    let initial = s.archive().unwrap();
    let records = library(&mut s);
    let id = records["records"][0]["id"].clone();
    let before = info(&mut s, &id);
    change(&mut s,json!({"kind":"edit","id":id,"expected":before["fingerprint"],"patches":[{"path":"/note","value":"An added note"}],"profile":null})).unwrap();
    let edited = info(&mut s, &id);
    for field in ["title", "author", "date", "publisher"] {
        assert_eq!(
            before["record"]["entry"][field],
            edited["record"]["entry"][field]
        );
    }
    let bytes = s.archive().unwrap().yjs;
    import(&mut s, input);
    assert_eq!(s.archive().unwrap().yjs, bytes);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(s.archive().unwrap().metadata, initial.metadata);
    s.execute("edit.undo", json!({})).unwrap();
    assert!(library(&mut s)["sources"].as_array().unwrap().is_empty());
    s.execute("edit.redo", json!({})).unwrap();
    assert_eq!(s.archive().unwrap().metadata, initial.metadata);
}
#[test]
fn source_mutation_stale_preparation_and_generic_collection_bypasses_are_refused() {
    let mut s = article();
    let input = source("original");
    let prepared = import(&mut s, input.clone());
    let bytes = s.archive().unwrap().yjs;
    let mut changed = input;
    changed["contents"] = json!("@book{other,title={A different source}}");
    let new = s
        .query("bibliography.prepare_import", changed.clone())
        .unwrap();
    assert!(change(&mut s,json!({"kind":"import","source":changed,"preparation":new["fingerprint"],"decisions":[]})).is_err());
    assert!(change(&mut s,json!({"kind":"import","source":source("another"),"preparation":prepared["fingerprint"],"decisions":[]})).is_err());
    for call in [
        Call {
            id: "record.remove".into(),
            params: json!({"collection":"reference_sources","id":"original"}),
        },
        Call {
            id: "metadata.set".into(),
            params: json!({"target":{"kind":"record","collection":"references","id":"x","field":"key"},"value":"changed"}),
        },
    ] {
        assert!(s.batch(&[call], None).is_err());
    }
    assert_eq!(s.archive().unwrap().yjs, bytes);
}
#[test]
fn cas_recovery_and_export_keep_one_active_hayagriva_and_exact_original_bytes() {
    let mut s = article();
    let input = source("original");
    import(&mut s, input.clone());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.typsto");
    s.save_as(&path).unwrap();
    let before = s.archive().unwrap();
    let restored = crate::content::project::Project::read(&path)
        .unwrap()
        .current()
        .clone();
    assert_eq!(restored.metadata, before.metadata);
    let resources = crate::bibliography::resources(&restored.metadata);
    assert_eq!(resources.len(), 2);
    assert_eq!(
        resources["bibliography/sources/original.bib"].as_ref(),
        input["contents"].as_str().unwrap().as_bytes()
    );
    let metadata = &restored.metadata;
    assert!(
        metadata
            .main_source()
            .contains("bibliography/references.yaml")
    );
    assert!(!metadata.main_source().contains("sources/original"));
    let mut world = crate::rendering::TypstWorld::default();
    world.set_dependencies(&restored.dependencies);
    let source = restored.validate().unwrap();
    let compiled = world
        .compile_with_template(&source, &restored.metadata, &restored.template)
        .unwrap();
    let output = dir.path().join("export");
    crate::exporting::write_document(
        &compiled,
        &crate::rendering::RenderRequest {
            source,
            metadata: restored.metadata,
            template: restored.template,
            model_manifest: restored.model_manifest,
            assets: restored.assets,
            dependencies: restored.dependencies,
        },
        &output,
        crate::protocol::ExportFormat::Typst,
        false,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(output.join("bibliography/references.yaml")).unwrap(),
        resources["bibliography/references.yaml"].as_ref()
    );
    assert_eq!(
        std::fs::read(output.join("bibliography/sources/original.bib")).unwrap(),
        resources["bibliography/sources/original.bib"].as_ref()
    );
}
#[test]
fn removing_an_uncited_notice_releases_its_source_but_undo_recovers_both() {
    let mut s = article();
    import(&mut s, source("original"));
    let before = s.archive().unwrap();
    let records = library(&mut s);
    let id = &records["records"][0]["id"];
    let info = info(&mut s, id);
    change(
        &mut s,
        json!({"kind":"remove","id":id,"expected":info["fingerprint"]}),
    )
    .unwrap();
    assert!(crate::bibliography::resources(&s.archive().unwrap().metadata).is_empty());
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(s.archive().unwrap().metadata, before.metadata);
}

#[test]
fn imports_publish_only_after_all_conflicts_have_explicit_decisions() {
    let mut s = article();
    let proposed=s.query("bibliography.prepare_record",json!({"id":"existing","profile":"book","key":"martin2026","entry":{"title":"A completely different book","author":"Other, Alex","date":1990}})).unwrap();
    change(&mut s, json!({"kind":"create","record":proposed["record"]})).unwrap();
    let input = source("source");
    let report = s
        .query("bibliography.prepare_import", input.clone())
        .unwrap();
    assert_eq!(report["plan"]["blocked"], true);
    let bytes = s.archive().unwrap().yjs;
    assert!(change(&mut s,json!({"kind":"import","source":input,"preparation":report["fingerprint"],"decisions":[]})).is_err());
    assert_eq!(s.archive().unwrap().yjs, bytes);
    change(&mut s,json!({"kind":"import","source":input,"preparation":report["fingerprint"],"decisions":[{"kind":"distinct","incoming":report["records"][0]["id"],"key":"martin2026_imported"}]})).unwrap();
    let library = library(&mut s);
    assert_eq!(library["records"].as_array().unwrap().len(), 2);
    assert!(
        library["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["key"] == "martin2026_imported")
    );
}
