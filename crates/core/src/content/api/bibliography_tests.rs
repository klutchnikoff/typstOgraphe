use super::{Session, check_params, describe, error_value};
use serde_json::{Value, json};

fn article() -> Session {
    let mut session = Session::new();
    session
        .execute("document.new", json!({"type":"article"}))
        .unwrap();
    session
}
fn validates(id: &str, value: &Value) {
    let description = describe(id).unwrap();
    jsonschema::validator_for(&description["result_schema"])
        .unwrap()
        .validate(value)
        .unwrap();
}

#[test]
fn preparation_is_repeatable_and_does_not_publish_change_history_or_dirty_the_project() {
    let mut session = article();
    let guard = session.guard().unwrap().clone();
    let bytes = session.archive().unwrap().yjs;
    let context = session.query("context.get", json!({})).unwrap();
    let dirty = session.dirty();
    let params = json!({"id":"source-1","name":"native.yaml","format":"hayagriva",
        "contents":include_str!("../../../../../fixtures/bibliography/native.yaml")});
    let result = session
        .query("bibliography.prepare_import", params.clone())
        .unwrap();
    validates("bibliography.prepare_import", &result);
    assert_eq!(result["active"], false);
    assert_eq!(result["records"].as_array().unwrap().len(), 6);
    assert!(result["source"].get("contents").is_none());
    assert_eq!(result["source"]["id"], "source-1");
    assert!(!result["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(
        session
            .query("bibliography.prepare_import", params.clone())
            .unwrap(),
        result
    );
    let mut changed = params;
    changed["contents"] = json!("other: {type: book}");
    let other = session
        .query("bibliography.prepare_import", changed)
        .unwrap();
    assert_ne!(other["fingerprint"], result["fingerprint"]);
    assert_eq!(
        session
            .query("bibliography.search", json!({"query":""}))
            .unwrap()["entries"],
        json!([])
    );
    assert_eq!(session.archive().unwrap().yjs, bytes);
    assert_eq!(session.guard().unwrap(), &guard);
    assert_eq!(session.query("context.get", json!({})).unwrap(), context);
    assert_eq!(session.dirty(), dirty);
    let rejected = session.batch(&[super::Call{id:"inline.replace".into(),params:json!({
        "target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},
        "node":{"type":"citation","attrs":{"form":"normal","items":[{"record_id":result["records"][0]["id"],"supplement":""}]}}
    })}],None);
    assert!(rejected.is_err());
    assert_eq!(session.archive().unwrap().yjs, bytes);
}

#[test]
fn genres_and_new_notices_have_the_same_machine_readable_types_as_the_crate() {
    let mut session = article();
    let kinds = session.query("bibliography.profiles", json!({})).unwrap();
    validates("bibliography.profiles", &kinds);
    assert_eq!(
        kinds["profiles"],
        json!(typstographe_bibliography::profiles())
    );
    for (i, genre) in kinds["profiles"].as_array().unwrap().iter().enumerate() {
        let result = session
            .query(
                "bibliography.prepare_record",
                json!({"id":format!("draft-{i}"),"profile":genre["profile"]}),
            )
            .unwrap();
        validates("bibliography.prepare_record", &result);
        assert_eq!(result["active"], false);
        assert_eq!(result["profile"], genre["profile"]);
        assert!(!result["diagnostics"].as_array().unwrap().is_empty());
        assert_eq!(result["record"]["key"], "anonymous_untitled_nd");
    }
}

#[test]
fn preparation_preserves_compound_fields_and_uses_available_names_without_renaming_existing_entries()
 {
    let mut session = article();
    let record = crate::bibliography::test_record(
        "source",
        "existing.yaml",
        "durand_une_2025: {type: article, title: Existing work}",
    );
    let library = session.query("bibliography.inspect", json!({})).unwrap();
    session
        .execute(
            "bibliography.apply",
            json!({"expected":library["fingerprint"],"change":{"kind":"create","record":record}}),
        )
        .unwrap();
    let before = session.archive().unwrap().yjs;
    let params = json!({"id":"proposal","profile":"journal_article","entry":{
        "title":{"value":"Une étude","short":"Étude","verbatim":true},
        "author":{"name":"Durand","given-name":"Élise"},"date":2025,
        "parent":{"type":"periodical","title":"Revue","publisher":{"name":"Press","location":"Paris"}},
        "serial-number":{"custom":"preserved"}}});
    let result = session
        .query("bibliography.prepare_record", params.clone())
        .unwrap();
    validates("bibliography.prepare_record", &result);
    assert_eq!(result["record"]["key"], "durand_une_2025_1");
    let entry = &result["record"]["entry"];
    assert_eq!(entry["title"]["short"], params["entry"]["title"]["short"]);
    let native: typstographe_bibliography::Record =
        serde_json::from_value(result["record"].clone()).unwrap();
    assert_eq!(
        native
            .entry()
            .title()
            .unwrap()
            .format_title_case(Default::default()),
        "Une étude"
    );
    assert_eq!(
        native.entry().parents()[0]
            .title()
            .unwrap()
            .value
            .to_string(),
        "Revue"
    );
    assert_eq!(
        native.entry().serial_number().unwrap().0["custom"],
        "preserved"
    );
    let mut occupied = params;
    occupied["key"] = json!("durand_une_2025");
    let error = error_value(
        &session
            .query("bibliography.prepare_record", occupied)
            .unwrap_err(),
    );
    assert_eq!(error["code"], "bibliography");
    assert_eq!(error["error"]["code"], "duplicate_key");
    assert_eq!(error["error"]["key"], "durand_une_2025");
    assert_eq!(session.archive().unwrap().yjs, before);
}

#[test]
fn invalid_sources_fields_and_bounds_return_structured_diagnostics_without_mutation() {
    let mut session = article();
    let before = session.archive().unwrap().yjs;
    let error = error_value(
        &session
            .query(
                "bibliography.prepare_record",
                json!({"id":"r","profile":"book","entry":{"custom":"would disappear"}}),
            )
            .unwrap_err(),
    );
    assert_eq!(error["error"]["code"], "unsupported_field");
    assert_eq!(error["error"]["field"], "/custom");
    for contents in [
        "@book{broken".to_owned(),
        "x".repeat(typstographe_bibliography::MAX_SOURCE_BYTES + 1),
    ] {
        let error = error_value(
            &session
                .query(
                    "bibliography.prepare_import",
                    json!({"id":"src","name":"in.bib","format":"biblatex","contents":contents}),
                )
                .unwrap_err(),
        );
        assert_eq!(error["code"], "bibliography");
        assert!(
            ["invalid_syntax", "limit_exceeded"]
                .contains(&error["error"]["code"].as_str().unwrap())
        );
    }
    assert_eq!(session.archive().unwrap().yjs, before);
}

#[test]
fn preparation_schemas_refuse_paths_unknown_fields_and_invalid_identities_and_types() {
    for params in [
        json!({"id":"../../escape","name":"in.bib","format":"biblatex","contents":""}),
        json!({"id":"src","name":"in.bib","format":"biblatex","contents":"","path":"/tmp/input.bib"}),
        json!({"id":"src","name":"in.bib","format":"other","contents":""}),
    ] {
        assert!(
            check_params("bibliography.prepare_import", &params).is_err(),
            "{params}"
        );
    }
    for params in [
        json!({"id":"bad id","profile":"book"}),
        json!({"id":"r","profile":"invented"}),
        json!({"id":"r","profile":"book","entry":"not an object"}),
        json!({"id":"r","profile":"book","publish":true}),
    ] {
        assert!(
            check_params("bibliography.prepare_record", &params).is_err(),
            "{params}"
        );
    }
    assert!(check_params("bibliography.profiles", &json!({"publish":true})).is_err());
}

#[test]
fn bibliography_preparation_does_not_grant_unsupported_types_a_new_capability() {
    let mut session = Session::new();
    assert!(session.query("bibliography.profiles", json!({})).is_err());
    session
        .execute("document.new", json!({"type":"courrier"}))
        .unwrap();
    let error = error_value(
        &session
            .query(
                "bibliography.prepare_record",
                json!({"id":"r","profile":"book"}),
            )
            .unwrap_err(),
    );
    assert_eq!(error["code"], "command_unavailable");
    assert_eq!(error["reason"], "unsupported_document_type");
}

#[test]
fn manager_form_preparation_is_private_and_edit_with_key_is_one_guarded_operation() {
    let mut s = article();
    let seed = s.query("bibliography.prepare_record", json!({"id":"r","profile":"journal_article","key":"old","entry":{"title":{"value":"A title","short":"Short"},"note":"Keep this"}})).unwrap();
    let before = s.archive().unwrap().yjs;
    let form = s
        .query("bibliography.form", json!({"record":seed["record"]}))
        .unwrap();
    assert!(
        form["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "/title")
    );
    let prepared = s.query("bibliography.prepare_form", json!({"record":seed["record"],"values":{"/title":"Edited title","/parent/0/title":"Journal"},"key":"new"})).unwrap();
    assert_eq!(s.archive().unwrap().yjs, before);
    assert_eq!(prepared["record"]["key"], "new");
    assert_eq!(prepared["record"]["entry"]["title"]["short"], "Short");
    let base = s.query("bibliography.inspect", json!({})).unwrap();
    s.execute(
        "bibliography.apply",
        json!({"expected":base["fingerprint"],"change":{"kind":"create","record":seed["record"]}}),
    )
    .unwrap();
    let original = s.archive().unwrap();
    let current = s.query("bibliography.inspect", json!({})).unwrap();
    let edit = json!({"kind":"edit","id":"r","expected":seed["fingerprint"],"patches":prepared["patches"],"key":"new"});
    s.execute(
        "bibliography.apply",
        json!({"expected":current["fingerprint"],"change":edit}),
    )
    .unwrap();
    let result = s.query("bibliography.record", json!({"id":"r"})).unwrap();
    assert_eq!(result["record"]["key"], "new");
    assert_eq!(result["record"]["entry"]["note"], "Keep this");
    let after = s.archive().unwrap().yjs;
    let current = s.query("bibliography.inspect", json!({})).unwrap();
    assert!(
        s.execute(
            "bibliography.apply",
            json!({"expected":current["fingerprint"],"change":edit})
        )
        .is_err()
    );
    assert_eq!(s.archive().unwrap().yjs, after);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(s.archive().unwrap().metadata, original.metadata);
}

#[test]
fn import_review_is_private_and_refuses_a_changed_library() {
    let mut s = article();
    let source =
        json!({"id":"s","name":"in.bib","format":"biblatex","contents":"@book{a,title={Example}}"});
    let prepared = s
        .query("bibliography.prepare_import", source.clone())
        .unwrap();
    let before = s.archive().unwrap().yjs;
    let input = json!({"source":source,"expected":prepared["library_fingerprint"],"decisions":[]});
    let review = s
        .query("bibliography.review_import", input.clone())
        .unwrap();
    assert_eq!(review["plan"], prepared["plan"]);
    assert_eq!(s.archive().unwrap().yjs, before);
    let record = s
        .query(
            "bibliography.prepare_record",
            json!({"id":"r","profile":"book"}),
        )
        .unwrap();
    s.execute("bibliography.apply",json!({"expected":prepared["library_fingerprint"],"change":{"kind":"create","record":record["record"]}})).unwrap();
    let before = s.archive().unwrap().yjs;
    let error = error_value(&s.query("bibliography.review_import", input).unwrap_err());
    assert_eq!(error["error"]["code"], "stale_base");
    assert_eq!(s.archive().unwrap().yjs, before);
}

#[test]
fn bibliography_usage_finds_objects_inside_widgets_without_changing_editor_context() {
    let mut s = article();
    let record = s
        .query(
            "bibliography.prepare_record",
            json!({"id":"r","profile":"book","entry":{"title":"Example"}}),
        )
        .unwrap();
    let base = s.query("bibliography.inspect", json!({})).unwrap();
    s.batch(&[
        super::Call{id:"bibliography.apply".into(),params:json!({"expected":base["fingerprint"],"change":{"kind":"create","record":record["record"]}})},
        super::Call{id:"block.insert".into(),params:json!({"parent":{"root":{"kind":"body"}},"index":0,"node":{"type":"quote","content":[{"type":"paragraph","content":[{"type":"citation","attrs":{"form":"normal","items":[{"record_id":"r"}]}}]},{"type":"quoteAttribution"}]}})},
    ],None).unwrap();
    let before = s.archive().unwrap().yjs;
    let context = s.query("context.get", json!({})).unwrap();
    let usage = s.query("bibliography.usage", json!({"id":"r"})).unwrap();
    assert_eq!(
        usage["targets"],
        json!([{"root":{"kind":"body"},"path":[0,0,0]}])
    );
    let browse = s
        .query("bibliography.browse", json!({"query":"example"}))
        .unwrap();
    assert_eq!(browse["total"], 1);
    assert_eq!(browse["entries"][0]["id"], "r");
    assert_eq!(s.archive().unwrap().yjs, before);
    assert_eq!(s.query("context.get", json!({})).unwrap(), context);
}

#[test]
fn changing_native_profiles_is_private_until_one_guarded_publication_and_preserves_hidden_data() {
    let mut s = article();
    let initial=s.query("bibliography.prepare_record",json!({"id":"r","profile":"journal_article","key":"fixed","entry":{"title":"A work","parent":{"type":"periodical","title":"Original journal","date":2026},"serial-number":{"isbn":"9780000000000"}}})).unwrap();
    let base = s.query("bibliography.inspect", json!({})).unwrap();
    s.execute("bibliography.apply",json!({"expected":base["fingerprint"],"change":{"kind":"create","record":initial["record"]}})).unwrap();
    let before = s.archive().unwrap();
    let prepared = s
        .query(
            "bibliography.prepare_form",
            json!({"record":initial["record"],"values":{"/title":"Edited work"},"profile":"book"}),
        )
        .unwrap();
    assert_eq!(prepared["profile"], "book");
    assert!(prepared["record"].get("profile").is_none());
    assert!(prepared["record"].get("genre").is_none());
    assert_eq!(
        prepared["record"]["retained_parents"][0]["title"],
        "Original journal"
    );
    assert_eq!(s.archive().unwrap().yjs, before.yjs);
    let current = s.query("bibliography.inspect", json!({})).unwrap();
    let call = json!({"expected":current["fingerprint"],"change":{"kind":"replace","id":"r","expected":initial["fingerprint"],"record":prepared["record"]}});
    s.execute("bibliography.apply", call).unwrap();
    let after = s.archive().unwrap();
    let resources = crate::bibliography::resources(&after.metadata);
    let retained = &resources[crate::bibliography::RETAINED_PATH];
    assert!(
        std::str::from_utf8(retained)
            .unwrap()
            .contains("Original journal")
    );
    assert!(
        !std::str::from_utf8(&resources[crate::bibliography::ACTIVE_PATH])
            .unwrap()
            .contains("Original journal")
    );
    let project = crate::content::project::Project::new(after.clone()).unwrap();
    let bytes = project.to_bytes().unwrap();
    let project = crate::content::project::Project::from_bytes(bytes).unwrap();
    let reopened = project.current();
    assert_eq!(reopened.metadata, after.metadata);
    let crate::document_types::Metadata::Article(meta) = &reopened.metadata else {
        panic!()
    };
    assert_eq!(
        meta.references[0].profile(),
        typstographe_bibliography::Profile::Book
    );
    let restored = meta.references[0]
        .reclassified(
            &meta.references[0].fingerprint(),
            typstographe_bibliography::Profile::JournalArticle,
        )
        .unwrap();
    assert_eq!(
        restored.entry().parents()[0]
            .title()
            .unwrap()
            .value
            .to_string(),
        "Original journal"
    );
    let mut world = crate::rendering::TypstWorld::default();
    world.set_dependencies(&after.dependencies);
    let source = after.validate().unwrap();
    let compiled = world
        .compile_with_template(&source, &after.metadata, &after.template)
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("export");
    crate::exporting::write_document(
        &compiled,
        &crate::rendering::RenderRequest {
            source,
            metadata: after.metadata.clone(),
            template: after.template.clone(),
            model_manifest: after.model_manifest.clone(),
            assets: after.assets.clone(),
            dependencies: after.dependencies.clone(),
        },
        &output,
        crate::protocol::ExportFormat::Typst,
        false,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(output.join(crate::bibliography::RETAINED_PATH)).unwrap(),
        retained.as_ref()
    );
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(s.archive().unwrap().metadata, before.metadata);
    s.execute("edit.redo", json!({})).unwrap();
    assert_eq!(s.archive().unwrap().metadata, after.metadata);
}

#[test]
fn prepared_type_changes_refuse_invalid_private_fields_and_guard_record_replacement() {
    let mut s = article();
    let initial=s.query("bibliography.prepare_record",json!({"id":"r","profile":"book","key":"fixed","entry":{"title":"An example","date":2026}})).unwrap();
    let base = s.query("bibliography.inspect", json!({})).unwrap();
    s.execute("bibliography.apply",json!({"expected":base["fingerprint"],"change":{"kind":"create","record":initial["record"]}})).unwrap();
    let before = s.archive().unwrap().yjs;
    assert!(s.query("bibliography.prepare_form",json!({"record":initial["record"],"values":{"/date":"not-a-date"},"profile":"journal_article"})).is_err());
    assert_eq!(s.archive().unwrap().yjs, before);
    let prepared = s
        .query(
            "bibliography.prepare_form",
            json!({"record":initial["record"],"profile":"thesis"}),
        )
        .unwrap();
    for field in ["id", "provenance"] {
        let mut record = prepared["record"].clone();
        record[field] = if field == "id" {
            json!("wrong")
        } else {
            json!([{"source_id":"invented","source_digest":"0".repeat(64),"original_key":"fixed"}])
        };
        let base = s.query("bibliography.inspect", json!({})).unwrap();
        assert!(s.execute("bibliography.apply",json!({"expected":base["fingerprint"],"change":{"kind":"replace","id":"r","expected":initial["fingerprint"],"record":record}})).is_err());
        assert_eq!(s.archive().unwrap().yjs, before);
    }
    let base = s.query("bibliography.inspect", json!({})).unwrap();
    s.execute("bibliography.apply",json!({"expected":base["fingerprint"],"change":{"kind":"edit","id":"r","expected":initial["fingerprint"],"patches":[],"profile":"journal_article"}})).unwrap();
    let after = s.archive().unwrap().yjs;
    let base = s.query("bibliography.inspect", json!({})).unwrap();
    assert!(s.execute("bibliography.apply",json!({"expected":base["fingerprint"],"change":{"kind":"replace","id":"r","expected":initial["fingerprint"],"record":prepared["record"]}})).is_err());
    assert_eq!(s.archive().unwrap().yjs, after);
}
