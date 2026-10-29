use super::*;
use crate::{
    DocumentLease,
    content::{
        archive::Archive,
        commands::{TextTarget, TextZone},
    },
};
use serde_json::json;
fn new(kind: &str) -> Session {
    let mut s = Session::new();
    s.execute("document.new", json!({"type":kind})).unwrap();
    s
}
fn call(id: &str, params: Value) -> Call {
    Call {
        id: id.into(),
        params,
    }
}
fn target(path: &[usize], from: u32, to: u32) -> Value {
    json!(TextTarget {
        zone: TextZone::Body { path: path.into() },
        from,
        to
    })
}
#[test]
fn catalogue_schemas_are_executable_and_global_discovery_needs_no_document() {
    let mut s = Session::new();
    assert!(!s.is_open());
    assert_eq!(s.query("document_types.list", json!({})).unwrap(), types());
    assert!(s.query("document.text", json!({})).is_err());
    let mut ids = std::collections::HashSet::new();
    for spec in catalogue()["entries"].as_array().unwrap() {
        assert!(ids.insert(spec["id"].as_str().unwrap()));
        jsonschema::validator_for(&spec["params_schema"]).unwrap();
    }
    assert!(
        check_params(
            "text.insert",
            &json!({"target":target(&[0],0,0),"text":"X","unexpected":true})
        )
        .is_err()
    );
    assert!(check_params("search.find", &json!({"query":"x","case_sensitive":"yes"})).is_err());
}
#[test]
fn typed_article_header_cells_and_unicode_roundtrip_without_a_flat_text_engine() {
    let mut s = new("article");
    let steps = vec![
        call(
            "text.insert",
            json!({"target":target(&[0],0,0),"text":"A😀日本"}),
        ),
        call("text.bold", json!({"target":target(&[0],1,3),"value":"on"})),
        call(
            "insert.table",
            json!({"parent":{"root":{"kind":"body"},"path":[]},"index":1,"rows":2,"columns":2,"header":true}),
        ),
        call(
            "text.insert",
            json!({"target":target(&[1,0,1,0,0],0,0),"text":"Cellule"}),
        ),
    ];
    let guard = s.guard().unwrap().clone();
    let result = s.batch(&steps, None).unwrap();
    assert_eq!(
        result["publication"]["guard"]["revision"],
        guard.revision + 1
    );
    assert_eq!(s.query("context.get", json!({})).unwrap()["undo_steps"], 1);
    let bytes = s.archive().unwrap().yjs;
    assert!(
        s.execute("text.delete", json!({"target":target(&[0],2,3)}))
            .is_err()
    );
    assert_eq!(s.archive().unwrap().yjs, bytes);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("article.typsto");
    s.save_as(&path).unwrap();
    let archive = Archive::read(&path).unwrap();
    assert_eq!(archive.state(), *s.state().unwrap());
    assert_eq!(archive.yjs, bytes);
    assert!(
        s.query("document.structure", json!({})).unwrap()["zones"]
            .as_array()
            .unwrap()
            .iter()
            .any(|z| z["text"] == "Cellule")
    );
    s.execute("edit.undo", json!({})).unwrap();
    assert!(
        !s.query("document.text", json!({})).unwrap()["text"]
            .as_str()
            .unwrap()
            .contains("Cellule")
    );
    s.execute("edit.redo", json!({})).unwrap();
    assert_eq!(s.state().unwrap().document, archive.document);
}
#[test]
fn a_failed_late_step_or_save_publishes_nothing_and_keeps_context_and_file() {
    let mut s = new("courrier");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    s.save_as(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let state = s.archive().unwrap().yjs;
    let steps = [
        call(
            "text.insert",
            json!({"target":target(&[0],0,0),"text":"Private"}),
        ),
        call("text.delete", json!({"target":target(&[9],0,0)})),
    ];
    assert!(s.batch(&steps, Some(Save::Current)).is_err());
    assert_eq!(s.archive().unwrap().yjs, state);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(s.query("context.get", json!({})).unwrap()["undo_steps"], 0);
    std::fs::write(&path, b"external edit").unwrap();
    assert!(s.batch(&steps[..1], Some(Save::Current)).is_err());
    assert_eq!(s.archive().unwrap().yjs, state);
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
}
#[test]
fn selectors_resolve_in_the_evolving_draft_and_do_not_cross_zones() {
    let mut s = new("courrier");
    s.batch(
        &[
            call(
                "text.insert",
                json!({"target":target(&[0],0,0),"text":"Élodie Élodie 😀"}),
            ),
            call(
                "text.italic",
                json!({"selector":{"select":"Élodie","occurrence":2},"value":"on"}),
            ),
        ],
        None,
    )
    .unwrap();
    let node = &s.state().unwrap().document.content[0];
    assert!(
        node.content
            .iter()
            .any(|n| n.text.as_deref() == Some("Élodie")
                && n.marks.iter().any(|m| m.kind == "italic"))
    );
    let before = s.archive().unwrap().yjs;
    assert!(
        s.execute("text.bold", json!({"selector":{"select":"Élodie"}}))
            .is_err()
    );
    assert_eq!(s.archive().unwrap().yjs, before);
    assert_eq!(
        s.query("search.find", json!({"query":"éLODIE"})).unwrap()["matches"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn contexts_and_typing_styles_are_private_and_clearable() {
    let mut s = new("courrier");
    let before = s.archive().unwrap().yjs;
    let selection: crate::content::editing::Selection =
        serde_json::from_value::<TextTarget>(target(&[0], 0, 0))
            .unwrap()
            .into();
    s.execute(
        "context.set",
        json!({"selection":selection,"typing_styles":[{"type":"bold"}]}),
    )
    .unwrap();
    assert_eq!(s.archive().unwrap().yjs, before);
    s.execute("context.insert", json!({"text":"Gras"})).unwrap();
    s.execute(
        "context.style",
        json!({"style":{"type":"bold"},"value":"off"}),
    )
    .unwrap();
    s.execute("context.insert", json!({"text":" simple"}))
        .unwrap();
    let nodes = &s.state().unwrap().document.content[0].content;
    assert_eq!(nodes.len(), 2);
    assert!(!nodes[0].marks.is_empty());
    assert!(nodes[1].marks.is_empty());
    s.execute("context.set", json!({"selection":null,"typing_styles":[]}))
        .unwrap();
    assert!(s.execute("context.insert", json!({"text":"No"})).is_err());
}
#[test]
fn lock_before_read_read_only_snapshot_and_save_as_do_not_steal_the_writer() {
    let mut owner = new("courrier");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("locked.typsto");
    owner.save_as(&path).unwrap();
    let mut reader = Session::new();
    reader
        .execute("document.open", json!({"path":path,"read_only":true}))
        .unwrap();
    assert!(reader.read_only());
    assert!(!reader.dirty());
    assert!(
        reader
            .execute(
                "text.insert",
                json!({"target":target(&[0],0,0),"text":"No"})
            )
            .is_err()
    );
    let mut writer = Session::new();
    assert!(
        writer
            .execute("document.open", json!({"path":path,"read_only":false}))
            .is_err()
    );
    let copy = dir.path().join("copy.typsto");
    reader.save_as(&copy).unwrap();
    assert!(!reader.read_only());
    assert!(DocumentLease::acquire(&path).is_err());
    assert!(owner.save_as(&copy).is_err());
}
#[test]
fn drafts_are_valid_without_required_export_fields_and_models_are_embedded() {
    let mut s = new("article");
    let validation = s.query("document.validate", json!({})).unwrap();
    assert_eq!(validation["valid"], true);
    assert_eq!(validation["ready_for_export"], false);
    let template = s.state().unwrap().settings.template.clone() + "\n// project-owned version\n";
    s.execute(
        "document.set_model",
        json!({"model_id":"article.model.working","source":template}),
    )
    .unwrap();
    assert_eq!(
        s.query("models.list", json!({})).unwrap()["active"]["embedded"],
        true
    );
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("export");
    assert!(
        s.export(&output, crate::engine::ExportFormat::Pdf, false)
            .is_err()
    );
    assert!(!output.exists());
    assert_eq!(
        s.query("render.compile", json!({})).unwrap()["pages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn semantic_capabilities_explain_actual_structured_execution() {
    for kind in ["courrier", "article"] {
        for cap in crate::content::catalogue::for_type(serde_json::from_value(json!(kind)).unwrap())
        {
            let mut s = new(kind);
            let description = describe(cap.id).unwrap();
            assert!(description.get("implementations").is_none());
            let calls: Vec<Call> = serde_json::from_value(description["examples"].clone()).unwrap();
            for call in &calls {
                check_params(&call.id, &call.params).unwrap();
            }
            // Existing references/resources are prerequisites, never invented by the API.
            if !matches!(
                cap.id,
                "insert.citation"
                    | "insert.image"
                    | "text.link"
                    | "block.heading_1"
                    | "block.heading_2"
                    | "block.heading_3"
                    | "block.paragraph"
            ) {
                assert!(!calls.is_empty(), "{}", cap.id);
                s.batch(&calls, None)
                    .unwrap_or_else(|e| panic!("{}: {e:?}", cap.id));
            }
        }
    }
}
#[test]
fn header_record_forward_references_and_rich_zones_use_the_same_atomic_owner() {
    let mut s = new("article");
    let id=serde_json::to_value(&s.state().unwrap().settings.metadata).unwrap()["fields"]["localizations"][0]["id"].clone();
    let t = json!({"zone":{"kind":"record","collection":"localizations","id":id,"field":"abstract","path":[0]},"from":0,"to":0});
    s.batch(&[
        call("record.add",json!({"collection":"authors","id":"camille","values":{"name":"Camille","affiliation-ids":["irmar"]}})),
        call("record.add",json!({"collection":"affiliations","id":"irmar","values":{"text":"Univ Rennes, CNRS, IRMAR"}})),
        call("text.insert",json!({"target":t,"text":"Résumé 日本😀","marks":[{"type":"italic"}]})),
    ],None).unwrap();
    assert!(
        s.query("search.find", json!({"query":"Résumé"})).unwrap()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["target"]["zone"]["kind"] == "record")
    );
    assert_eq!(s.query("context.get", json!({})).unwrap()["undo_steps"], 1);
    let before = s.archive().unwrap().yjs;
    assert!(
        s.execute(
            "record.remove",
            json!({"collection":"affiliations","id":"irmar"})
        )
        .is_err()
    );
    assert_eq!(s.archive().unwrap().yjs, before);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(
        serde_json::to_value(&s.state().unwrap().settings.metadata).unwrap()["fields"]["authors"],
        json!([])
    );
}
#[test]
fn preview_navigation_exposes_typed_targets_instead_of_flat_positions() {
    let mut s = new("courrier");
    s.execute(
        "text.insert",
        json!({"target":target(&[0],0,0),"text":"Bonjour Élodie"}),
    )
    .unwrap();
    let t = target(&[0], 0, 7);
    let point = s.query("render.locate", json!({"target":t})).unwrap()["point"].clone();
    assert!(point["point"].is_array(), "{point}");
    let r = s
        .query("render.selection_boxes", json!({"target":t}))
        .unwrap();
    let boxes = r["boxes"].as_array().unwrap();
    assert!(!boxes.is_empty());
    let corners = boxes[0]["corners"].as_array().unwrap();
    let center: Vec<f64> = (0..2)
        .map(|axis| {
            corners
                .iter()
                .map(|c| c[axis].as_f64().unwrap())
                .sum::<f64>()
                / 4.0
        })
        .collect();
    let back = s
        .query(
            "render.locate",
            json!({"point":{"page":boxes[0]["page"],"point":center}}),
        )
        .unwrap();
    assert_eq!(back["target"]["zone"]["kind"], "body");
}
#[test]
fn admitted_images_signatures_and_figures_share_retention_and_runtime_undo() {
    let mut s = Session::new();
    s.execute("document.new",json!({"type":"courrier","metadata":crate::document_types::Metadata::demo(crate::document_types::DocumentType::Courrier)})).unwrap();
    let path = "images/signature.svg";
    let resource = crate::content::commands::ImageResource {
        path: path.into(),
        data: include_bytes!("../../../../../fixtures/signature.svg").to_vec(),
    };
    let mut figure = describe("insert.image").unwrap()["examples"][0].clone();
    figure["params"]["node"]["attrs"]["src"] = json!(path);
    s.batch(
        &[
            call("resource.add", json!({"resource":resource})),
            serde_json::from_value(figure).unwrap(),
            call("document.set_signature", json!({"path":path})),
        ],
        None,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("signed.typsto");
    s.save_as(&file).unwrap();
    assert_eq!(Archive::read(&file).unwrap().assets.len(), 1);
    s.batch(
        &[
            call(
                "block.remove",
                json!({"target":{"root":{"kind":"body"},"path":[0]}}),
            ),
            call("document.set_signature", json!({"path":null})),
        ],
        Some(Save::Current),
    )
    .unwrap();
    assert!(Archive::read(&file).unwrap().assets.is_empty());
    assert!(s.assets().unwrap().contains_key(path));
    s.execute("edit.undo", json!({})).unwrap();
    s.save().unwrap();
    assert_eq!(Archive::read(&file).unwrap().assets.len(), 1);
    s.export(
        &dir.path().join("export"),
        crate::engine::ExportFormat::Both,
        false,
    )
    .unwrap();
    assert!(dir.path().join("export/images/signature.svg").exists());
}
#[test]
fn spelling_uses_the_language_of_each_localized_header_zone() {
    let mut s = new("article");
    s.batch(&[
        call("record.add",json!({"collection":"localizations","id":"en","values":{"language":"en"}})),
        call("text.insert",json!({"target":{"zone":{"kind":"record","collection":"localizations","id":"en","field":"abstract","path":[0]},"from":0,"to":0},"text":"The quick brown fox."})),
    ],None).unwrap();
    let result = s.query("spelling.issues", json!({})).unwrap();
    assert_eq!(result["available"], true);
    assert!(
        !result["issues"].as_array().unwrap().iter().any(
            |i| i["target"]["zone"]["id"] == "en" && i["target"]["zone"]["field"] == "abstract"
        )
    );
    s.execute("text.insert",json!({"target":{"zone":{"kind":"record","collection":"localizations","id":"en","field":"abstract","path":[0]},"from":0,"to":0},"text":"helllo "})).unwrap();
    assert!(
        s.query("spelling.issues", json!({})).unwrap()["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["word"] == "helllo" && i["language"] == "en")
    );
}
#[test]
fn resource_admission_alone_is_dirty_even_without_a_text_update() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = new("courrier");
    let path = dir.path().join("draft.typsto");
    s.save_as(&path).unwrap();
    assert!(!s.dirty());
    let resource = crate::content::commands::ImageResource {
        path: "images/signature.svg".into(),
        data: include_bytes!("../../../../../fixtures/signature.svg").to_vec(),
    };
    s.execute("resource.add", json!({"resource":resource}))
        .unwrap();
    assert!(s.dirty());
    assert_eq!(
        s.query("document.inspect", json!({})).unwrap()["dirty"],
        true
    );
    s.save().unwrap();
    assert!(!s.dirty());
    assert!(Archive::read(&path).unwrap().assets.is_empty());
}
#[test]
fn semantic_type_restrictions_cannot_be_bypassed_by_generic_nodes_or_archives() {
    let mut article = new("article");
    let step: Call = serde_json::from_value(
        describe("letter.insert_postscript").unwrap()["examples"][0].clone(),
    )
    .unwrap();
    let guard = article.guard().unwrap().clone();
    let before = article.archive().unwrap().yjs;
    let error = article
        .batch(std::slice::from_ref(&step), None)
        .unwrap_err();
    assert_eq!(error_value(&error)["reason"], "unsupported_document_type");
    assert_eq!(article.guard(), Some(&guard));
    assert_eq!(article.archive().unwrap().yjs, before);
    let mut archive = article.archive().unwrap();
    archive.yjs.clear();
    archive.document.content.insert(
        0,
        serde_json::from_value(step.params["node"].clone()).unwrap(),
    );
    assert!(archive.validate().is_err());
    // Only the final state is checked, so an intermediate private node can be removed.
    article
        .batch(
            &[
                step.clone(),
                call(
                    "block.remove",
                    json!({"target":{"root":{"kind":"body"},"path":[0]}}),
                ),
            ],
            None,
        )
        .unwrap();
    new("courrier").batch(&[step], None).unwrap();
}
