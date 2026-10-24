use super::*;

fn query(e: &Engine, id: &str) -> Value {
    e.query_call(Query::from_call(id, json!({})).unwrap().to_call())
        .unwrap()
}
fn availability(e: &Engine, id: &str) -> Value {
    query(e, "commands.list")["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap()["context_availability"]
        .clone()
}

#[test]
fn discovery_works_without_document_and_inspection_has_no_dummy_document_context() {
    let e = Engine::empty();
    let types = query(&e, "document_types.list");
    assert_eq!(
        types["types"].as_array().unwrap().len(),
        DocumentType::ALL.len()
    );
    let letter = &types["types"][0];
    assert_eq!(letter["id"], "courrier");
    assert_eq!(letter["can_create"], true);
    assert_eq!(letter["models"][0]["id"], "courrier.model.personal");
    assert_eq!(types["types"][1]["can_create"], false);
    assert!(types["types"][1]["models"].as_array().unwrap().is_empty());
    let inspect = query(&e, "document.inspect");
    for key in ["document_type", "capabilities", "typing_style", "selection"] {
        assert!(inspect[key].is_null(), "{key}");
    }
    assert_eq!(
        availability(&e, "text.bold")["reason"]["params"]["reason"],
        "no_document"
    );
    assert_eq!(
        availability(&e, "document.new")["state"],
        "requires_parameters"
    );
    assert_eq!(availability(&e, "document_types.list")["state"], "enabled");
    for id in ["document.source", "document.assets"] {
        assert_eq!(
            e.query(Query::from_call(id, json!({})).unwrap())
                .unwrap_err()
                .reason(),
            "no_document"
        );
    }
}

#[test]
fn discovery_exposes_source_type_capabilities_typing_and_context_availability_without_edits() {
    let mut e = engine("Été et Noël.");
    let source = e.document().source().to_owned();
    assert_eq!(query(&e, "document.source")["source"], source);
    assert_eq!(
        availability(&e, "text.link")["reason"]["params"]["reason"],
        "select_link"
    );
    assert_eq!(
        availability(&e, "edit.undo")["reason"]["params"]["reason"],
        "undo_empty"
    );
    let target = explicit(&e, 0..3);
    call(&mut e, "text.select", json!({"target":target})).unwrap();
    assert_eq!(
        availability(&e, "text.link")["state"],
        "requires_parameters"
    );
    call(&mut e, "text.bold", json!({"value":"on"})).unwrap();
    let caret = explicit(&e, 0..0);
    call(&mut e, "text.select", json!({"target":caret})).unwrap();
    call(&mut e, "text.italic", json!({"value":"on"})).unwrap();
    let inspect = query(&e, "document.inspect");
    assert_eq!(inspect["document_type"], "courrier");
    assert_eq!(inspect["typing_style"]["italic"], true);
    assert!(
        inspect["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("letter.insert_postscript"))
    );
    let stored = e.stored_document();
    let history = (e.session.undo.len(), e.session.redo.len());
    for id in [
        "document_types.list",
        "document.source",
        "document.assets",
        "commands.list",
    ] {
        query(&e, id);
    }
    assert_eq!(query(&e, "document.inspect"), inspect);
    assert_eq!(e.stored_document(), stored);
    assert_eq!((e.session.undo.len(), e.session.redo.len()), history);
    e.session.metadata = Metadata::blank(DocumentType::CompteRendu);
    assert!(
        !query(&e, "document.inspect")["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("letter.insert_postscript"))
    );
    assert_eq!(
        availability(&e, "letter.insert_postscript")["reason"]["params"]["reason"],
        "document_type"
    );
}

#[test]
fn resource_inventory_excludes_deleted_images_keeps_model_and_dependencies_and_undo() {
    let mut e = engine("#figure(image(\"images/used.svg\"))");
    let data: std::sync::Arc<[u8]> = include_bytes!("../../../../../fixtures/parc.svg")
        .as_slice()
        .into();
    for path in ["images/used.svg", "images/model.svg", "images/unused.svg"] {
        e.session.document.assets.insert(path.into(), data.clone());
    }
    e.session
        .template
        .push_str("\n#let dormant = \"images/model.svg\"\n");
    let before = e.stored_document();
    let inventory = query(&e, "document.assets");
    let paths: Vec<_> = inventory["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, vec!["images/model.svg", "images/used.svg"]);
    assert!(!inventory["dependencies"].as_array().unwrap().is_empty());
    assert_eq!(e.stored_document(), before);
    let image = e
        .image_target(*e.document().view.images.keys().next().unwrap())
        .unwrap();
    call(&mut e, "image.remove", json!({"target":image})).unwrap();
    let inventory = query(&e, "document.assets");
    assert_eq!(inventory["assets"].as_array().unwrap().len(), 1);
    assert_eq!(inventory["assets"][0]["path"], "images/model.svg");
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(
        query(&e, "document.assets")["assets"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
