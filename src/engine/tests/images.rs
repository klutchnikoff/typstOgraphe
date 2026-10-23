use super::*;
use crate::{assets::ImageData, document::edits::EditError};

fn embedded() -> Value {
    serde_json::to_value(ImageInput::from(&ImageData {
        extension: "svg".into(),
        data: include_bytes!("../../../tests/fixtures/parc.svg")
            .as_slice()
            .into(),
    }))
    .unwrap()
}
fn target(e: &Engine) -> Value {
    let value = e.query(Query::Structure(Empty {})).unwrap();
    let block = value["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["kind"] == "image")
        .unwrap();
    json!({"guard":value["guard"],"block_id":block["id"]})
}

#[test]
fn image_json_lifecycle_preserves_annotations_identities_resources_and_history() {
    let mut e = engine("Avant.\n\nAprès.");
    let insertion = explicit(&e, 6..6);
    call(
        &mut e,
        "insert.image",
        json!({"target":insertion,"image":embedded(),"caption":"Été *日本*","alt":"Des arbres"}),
    )
    .unwrap();
    let structure = e.query(Query::Structure(Empty {})).unwrap();
    let block = structure["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["kind"] == "image")
        .unwrap();
    let identity = (block["id"].clone(), block["zones"][0]["id"].clone());
    assert_eq!(block["image"]["alt"], "Des arbres");
    assert!(e.session.document.view.text.contains("Été *日本*"));
    assert_eq!(e.session.undo.len(), 1);
    let t = target(&e);
    call(
        &mut e,
        "image.set_alt",
        json!({"target":t,"alt":"Une allée"}),
    )
    .unwrap();
    let t = target(&e);
    call(
        &mut e,
        "image.replace",
        json!({"target":t,"image":embedded()}),
    )
    .unwrap();
    let structure = e.query(Query::Structure(Empty {})).unwrap();
    let block = structure["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["kind"] == "image")
        .unwrap();
    assert_eq!(
        (block["id"].clone(), block["zones"][0]["id"].clone()),
        identity
    );
    assert_eq!(block["image"]["alt"], "Une allée");
    assert_eq!(e.session.document.assets.len(), 2);
    let before_remove = e.session.document.clone();
    let selection = e.selection();
    let t = target(&e);
    call(&mut e, "image.remove", json!({"target":t})).unwrap();
    assert!(e.session.document.view.images.is_empty());
    assert!(!e.session.document.view.text.contains("Été"));
    assert_eq!(
        e.session.document.assets.len(),
        2,
        "l’historique conserve les données"
    );
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.session.document, before_remove);
    assert_eq!(e.selection(), selection);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert!(e.session.document.view.images.is_empty());
}

#[test]
fn image_import_accepts_png_jpeg_and_svg_and_remains_autonomous_after_original_removal() {
    for (extension, bytes) in [
        (
            "png",
            include_bytes!("../../../tests/fixtures/colours.png").as_slice(),
        ),
        (
            "JPEG",
            include_bytes!("../../../tests/fixtures/colours.jpg").as_slice(),
        ),
        (
            "svg",
            include_bytes!("../../../tests/fixtures/parc.svg").as_slice(),
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(format!("image.{extension}"));
        std::fs::write(&file, bytes).unwrap();
        let mut e = engine("");
        call(
            &mut e,
            "insert.image",
            json!({"image":{"kind":"path","path":file}}),
        )
        .unwrap();
        std::fs::remove_file(&file).unwrap();
        let image = e.session.document.view.images.values().next().unwrap();
        assert_eq!(e.session.document.assets[&image.path].as_ref(), bytes);
        assert_eq!(
            Document::from_source(e.session.document.source())
                .unwrap()
                .view,
            e.session.document.view
        );
        assert_eq!(e.selection().range(), 1..1);
    }
}

#[test]
fn image_targets_and_invalid_inputs_are_rejected_before_any_state_is_published() {
    let mut e = engine("#figure(image(\"images/parc.svg\"),caption:[Été])\n\nSuite.");
    e.session.document.assets.insert(
        "images/parc.svg".into(),
        include_bytes!("../../../tests/fixtures/parc.svg")
            .as_slice()
            .into(),
    );
    let stale = target(&e);
    let other = engine("#figure(image(\"images/parc.svg\"))");
    let foreign = target(&other);
    let t = target(&e);
    call(&mut e, "image.set_alt", json!({"target":t,"alt":"Arbres"})).unwrap();
    let current = target(&e);
    let mut unknown = current.clone();
    unknown["block_id"] = json!(u64::MAX);
    let structure = e.query(Query::Structure(Empty {})).unwrap();
    let text_block = structure["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["kind"] == "paragraph")
        .unwrap();
    let wrong_kind = json!({"guard":e.target_guard(),"block_id":text_block["id"]});
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("absent.png");
    for (id, params, reason, code) in [
        (
            "image.remove",
            json!({"target":unknown}),
            "unknown_block",
            ErrorCode::Document,
        ),
        (
            "image.remove",
            json!({"target":wrong_kind}),
            "image_required",
            ErrorCode::Unavailable,
        ),
        (
            "image.replace",
            json!({"target":stale,"image":{"kind":"path","path":missing}}),
            "stale_revision",
            ErrorCode::Document,
        ),
        (
            "image.remove",
            json!({"target":foreign}),
            "foreign_document",
            ErrorCode::Document,
        ),
        (
            "image.replace",
            json!({"target":current,"image":{"kind":"path","path":missing}}),
            "image_io",
            ErrorCode::Io,
        ),
        (
            "image.replace",
            json!({"target":current,"image":{"kind":"embedded","extension":"svg","data":"!"}}),
            "image_base64",
            ErrorCode::InvalidParams,
        ),
        (
            "image.replace",
            json!({"target":current,"image":{"kind":"embedded","extension":"svg","data":"bm90IHN2Zw=="}}),
            "image_svg",
            ErrorCode::InvalidParams,
        ),
    ] {
        let before = e.stored_document();
        let selection = e.selection();
        let state = (
            e.session.revision,
            e.session.undo.len(),
            e.session.redo.len(),
            e.session.changed_at,
        );
        let error = call(&mut e, id, params).unwrap_err();
        assert_eq!((error.reason(), error.code), (reason, code));
        assert!(e.stored_document() == before);
        assert_eq!(e.selection(), selection);
        assert_eq!(
            (
                e.session.revision,
                e.session.undo.len(),
                e.session.redo.len(),
                e.session.changed_at
            ),
            state
        );
    }
    for caption in ["une\nligne", "une\tligne", "\u{fffc}"] {
        assert_eq!(
            Command::from_call(
                "insert.image",
                json!({"image":embedded(),"caption":caption})
            )
            .unwrap()
            .id(),
            "insert.image"
        );
        let selection = explicit(
            &e,
            e.session.document.view.len()..e.session.document.view.len(),
        );
        let before = e.stored_document();
        assert_eq!(
            call(
                &mut e,
                "insert.image",
                json!({"target":selection,"image":embedded(),"caption":caption})
            )
            .unwrap_err()
            .reason(),
            "invalid_text"
        );
        assert!(e.stored_document() == before);
    }
}

#[test]
fn resource_budget_failure_does_not_publish_assets_metadata_selection_or_history() {
    let mut e = engine("");
    let bytes: std::sync::Arc<[u8]> = vec![0; 1024 * 1024].into();
    for i in 0..65 {
        std::sync::Arc::make_mut(&mut e.session.dependencies)
            .insert(format!("capabilities/large-{i}.typ"), bytes.clone());
    }
    for id in ["insert.image", "document.set_signature"] {
        let params = if id == "insert.image" {
            json!({"image":embedded()})
        } else {
            json!({"guard":e.target_guard(),"image":embedded()})
        };
        let before = e.stored_document();
        let selection = e.selection();
        let state = (
            e.session.revision,
            e.session.undo.len(),
            e.session.redo.len(),
            e.session.changed_at,
        );
        assert_eq!(
            call(&mut e, id, params).unwrap_err().reason(),
            "document_too_large"
        );
        assert!(e.stored_document() == before);
        assert_eq!(e.selection(), selection);
        assert_eq!(
            (
                e.session.revision,
                e.session.undo.len(),
                e.session.redo.len(),
                e.session.changed_at
            ),
            state
        );
    }
}

#[test]
fn signature_is_type_scoped_guarded_idempotent_and_fully_undoable() {
    let mut e = engine("Bonjour");
    let original = e.stored_document();
    let old_guard = e.target_guard();
    call(
        &mut e,
        "document.set_signature",
        json!({"guard":old_guard,"image":embedded()}),
    )
    .unwrap();
    let with_image = e.stored_document();
    let guard = e.target_guard();
    let outcome = call(
        &mut e,
        "document.set_signature",
        json!({"guard":guard,"image":embedded()}),
    )
    .unwrap();
    assert!(!outcome.changed && outcome.effects.is_empty());
    assert_eq!(e.session.document.assets.len(), 1);
    assert_eq!(e.session.undo.len(), 1);
    assert_eq!(
        call(
            &mut e,
            "document.set_signature",
            json!({"guard":old_guard,"image":null})
        )
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    call(
        &mut e,
        "document.set_signature",
        json!({"guard":guard,"image":null}),
    )
    .unwrap();
    assert!(e.session.metadata.signature_path().is_none());
    assert_eq!(
        e.session.template, original.template,
        "la réserve manuscrite appartient au modèle"
    );
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert!(e.stored_document() == with_image);
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert!(e.stored_document() == original);
    let stored = StoredDocument::blank(DocumentType::CompteRendu);
    let mut minutes: Engine =
        Session::new(stored, Document::from_source("").unwrap(), None, 0).into();
    let guard = minutes.target_guard();
    assert_eq!(
        call(
            &mut minutes,
            "document.set_signature",
            json!({"guard":guard,"image":embedded()})
        )
        .unwrap_err()
        .reason(),
        "document_type"
    );
}

#[test]
fn full_snapshot_publication_rejects_invalid_or_stale_candidates_without_partial_changes() {
    let mut e = engine("Bonjour");
    let before = e.session.snapshot(e.selection());
    let mut candidate = before.clone();
    candidate.document.view.styles.clear();
    candidate
        .document
        .assets
        .insert("images/test.svg".into(), b"invalid".as_slice().into());
    let saved = e.stored_document();
    assert_eq!(
        e.session
            .commit_snapshot(
                e.target_guard(),
                before.clone(),
                candidate,
                e.selection(),
                EditGroup::Discrete
            )
            .unwrap_err(),
        EditError::InvalidProjection
    );
    assert!(e.stored_document() == saved);
    let guard = e.target_guard();
    call(&mut e, "text.insert", json!({"text":"Été"})).unwrap();
    let current = e.stored_document();
    assert_eq!(
        e.session
            .commit_snapshot(
                guard,
                before.clone(),
                before,
                e.selection(),
                EditGroup::Discrete
            )
            .unwrap_err(),
        EditError::StaleDraft
    );
    assert!(e.stored_document() == current);
}
