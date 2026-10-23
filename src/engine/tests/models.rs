use super::*;
use crate::{
    document::{DocumentLanguage, PaperFormat},
    document_types::models::Model,
};

const CUSTOM: &str = "#let editor-options = (custom-bullet-marker: true,)\n#let courrier(..args) = args.pos().last()";

fn install(e: &mut Engine, source: &str) -> Result<Outcome, CommandError> {
    call(
        e,
        "document.set_model",
        json!({"guard":e.target_guard(),"model":{"kind":"source","source":source}}),
    )
}
fn preset(e: &mut Engine) -> Result<Outcome, CommandError> {
    call(
        e,
        "courrier.model.personal",
        json!({"guard":e.target_guard()}),
    )
}

#[test]
fn model_catalogue_is_type_driven_pure_and_keeps_the_existing_preset_identity() {
    let mut e = engine("Bonjour");
    let before = e.stored_document();
    let value = e.query_call(Query::Models(Empty {}).to_call()).unwrap();
    assert_eq!(value["models"].as_array().unwrap().len(), 1);
    assert_eq!(value["models"][0]["id"], "courrier.model.personal");
    assert_eq!(value["models"][0]["active"], true);
    assert_eq!(
        value["models"][0]["fixed_page"],
        json!({"paper":"a4","landscape":false})
    );
    assert!(e.stored_document() == before);
    assert!(e.undo.is_empty() && e.redo.is_empty() && !e.is_dirty());
    e.metadata = Metadata::blank(DocumentType::CompteRendu);
    assert!(
        e.query(Query::Models(Empty {})).unwrap()["models"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(preset(&mut e).unwrap_err().reason(), "document_type");
}

#[test]
fn model_changes_preserve_content_metadata_selection_assets_and_have_one_step_history() {
    let mut e = engine("Un *été* et une _idée_.");
    let target = explicit(&e, 3..6);
    call(&mut e, "text.select", json!({"target":target})).unwrap();
    let before = e.stored_document();
    let selection = e.selection();
    let outcome = install(&mut e, CUSTOM).unwrap();
    assert!(outcome.changed);
    assert_eq!(outcome.effects, [Effect::Render]);
    assert_eq!(
        outcome.value.unwrap()["presentation"]["custom_bullet_marker"],
        true
    );
    assert_eq!(e.metadata, before.metadata);
    assert_eq!(e.document.source(), before.source);
    assert_eq!(e.document.assets, before.assets);
    assert_eq!(e.selection(), selection);
    assert!(e.dependencies.is_empty());
    assert_eq!(e.undo.len(), 1);
    assert_eq!(
        e.query(Query::ModelPresentation(Empty {})).unwrap()["custom_bullet_marker"],
        true
    );
    let after = e.stored_document();
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), selection);
    assert_eq!(
        e.query(Query::ModelPresentation(Empty {})).unwrap()["custom_bullet_marker"],
        false
    );
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert!(e.stored_document() == after);
    assert_eq!(e.selection(), selection);
}

#[test]
fn preset_and_generic_source_share_page_defaults_and_reset_only_the_marker_on_change() {
    let mut alias = engine("Bonjour");
    install(&mut alias, CUSTOM).unwrap();
    for option in [
        DocumentOption::Paper(PaperFormat::A3),
        DocumentOption::Landscape(true),
        DocumentOption::Language(DocumentLanguage::English),
        DocumentOption::BulletMarker(Some("◆".into())),
    ] {
        let guard = alias.target_guard();
        call(
            &mut alias,
            "document.set_option",
            json!({"guard":guard,"option":option}),
        )
        .unwrap();
    }
    alias.metadata.courrier_mut().objet = "Objet conservé".into();
    let before = alias.stored_document();
    let mut generic: Engine =
        Session::new(before.clone(), alias.document.clone(), None, alias.revision).into();
    let history = alias.undo.len();
    preset(&mut alias).unwrap();
    install(&mut generic, &Model::Personal.source()).unwrap();
    assert!(alias.stored_document() == generic.stored_document());
    assert_eq!(alias.metadata.options().paper, PaperFormat::A4);
    assert!(!alias.metadata.options().landscape);
    assert!(alias.metadata.options().bullet_marker.is_none());
    assert_eq!(alias.metadata.options().language, DocumentLanguage::English);
    assert_eq!(alias.metadata.courrier().objet, "Objet conservé");
    assert_eq!(alias.undo.len(), history + 1);
    let revision = alias.revision;
    assert!(!preset(&mut alias).unwrap().changed);
    assert_eq!(alias.revision, revision);
    assert_eq!(alias.undo.len(), history + 1);
    call(&mut alias, "edit.undo", json!({})).unwrap();
    assert!(alias.stored_document() == before);
}

#[test]
fn reinstalling_identical_source_preserves_marker_and_does_not_add_history() {
    let mut e = engine("Bonjour");
    install(&mut e, CUSTOM).unwrap();
    let guard = e.target_guard();
    call(
        &mut e,
        "document.set_option",
        json!({"guard":guard,"option":{"key":"bullet_marker","value":"◆"}}),
    )
    .unwrap();
    let before = e.stored_document();
    let history = e.undo.len();
    let revision = e.revision;
    let outcome = install(&mut e, CUSTOM).unwrap();
    assert!(!outcome.changed && outcome.effects.is_empty());
    assert!(e.stored_document() == before);
    assert_eq!(e.undo.len(), history);
    assert_eq!(e.revision, revision);
}

#[test]
fn incompatible_or_broken_models_never_publish_source_dependencies_options_or_history() {
    let mut e = engine("Bonjour");
    for source in [
        "",
        "#let compte-rendu(body) = body",
        "#let courrier(body) = body",
        "#let courrier(..args) = unknown()",
        "#let editor-options = (bullet-markers: (42,),)\n#let courrier(..args) = []",
    ] {
        let before = e.stored_document();
        let selection = e.selection();
        let state = (e.revision, e.undo.len(), e.redo.len(), e.changed_at);
        let error = install(&mut e, source).unwrap_err();
        assert_eq!(error.code, ErrorCode::Template);
        assert_eq!(error.reason(), "model_validation");
        assert!(
            error.params["signature"]
                .as_str()
                .unwrap()
                .contains("courrier(")
        );
        assert!(e.stored_document() == before);
        assert_eq!(e.selection(), selection);
        assert_eq!(
            (e.revision, e.undo.len(), e.redo.len(), e.changed_at),
            state
        );
    }
    e.metadata = Metadata::blank(DocumentType::CompteRendu);
    install(&mut e, "#let compte-rendu(..args) = args.pos().last()").unwrap();
    assert_eq!(e.metadata.document_type(), DocumentType::CompteRendu);
}

#[test]
fn guards_strict_payloads_file_errors_and_limits_are_checked_before_publication() {
    let mut e = engine("Bonjour");
    let guard = e.target_guard();
    let params = json!({"guard":guard,"model":{"kind":"path","path":"does-not-exist.typ"}});
    let mut other = engine("Bonjour");
    assert_eq!(
        call(&mut other, "document.set_model", params.clone())
            .unwrap_err()
            .reason(),
        "foreign_document"
    );
    assert_eq!(
        call(&mut e, "document.set_model", params.clone())
            .unwrap_err()
            .reason(),
        "model_io"
    );
    install(&mut e, CUSTOM).unwrap();
    call(&mut e, "edit.undo", json!({})).unwrap();
    let before = e.stored_document();
    assert_eq!(
        call(&mut e, "document.set_model", params)
            .unwrap_err()
            .reason(),
        "stale_revision"
    );
    let dir = tempfile::tempdir().unwrap();
    for (bytes, reason) in [
        (vec![0xff], "model_utf8"),
        (
            vec![b'a'; super::super::models::MAX_MODEL_BYTES + 1],
            "model_too_large",
        ),
    ] {
        let path = dir.path().join("model.typ");
        std::fs::write(&path, bytes).unwrap();
        let guard = e.target_guard();
        assert_eq!(
            call(
                &mut e,
                "document.set_model",
                json!({"guard":guard,"model":{"kind":"path","path":path}})
            )
            .unwrap_err()
            .reason(),
            reason
        );
    }
    assert_eq!(
        install(
            &mut e,
            &"a".repeat(super::super::models::MAX_MODEL_BYTES + 1)
        )
        .unwrap_err()
        .reason(),
        "model_too_large"
    );
    for (id, params) in [
        (
            "document.set_model",
            json!({"model":{"kind":"source","source":CUSTOM}}),
        ),
        (
            "document.set_model",
            json!({"guard":e.target_guard(),"model":{"kind":"source","source":CUSTOM,"dependencies":{}}}),
        ),
        (
            "document.set_model",
            json!({"guard":e.target_guard(),"model":{"kind":"path"}}),
        ),
        (
            "courrier.model.personal",
            json!({"guard":e.target_guard(),"source":CUSTOM}),
        ),
    ] {
        assert_eq!(
            call(&mut e, id, params).unwrap_err().code,
            ErrorCode::InvalidParams
        );
    }
    assert!(e.stored_document() == before);
    assert!(e.undo.is_empty());
    assert_eq!(e.redo.len(), 1);
}

#[test]
fn imported_model_keeps_controlled_document_dependencies_and_survives_original_file_removal() {
    let mut e = engine("Bonjour");
    std::sync::Arc::make_mut(&mut e.dependencies).insert(
        "packages/PROVENANCE.md".into(),
        b"Exemplaire du document".as_slice().into(),
    );
    let template = format!("{}\n// Modèle d’organisation", Model::Personal.source());
    let dir = tempfile::tempdir().unwrap();
    let model = dir.path().join("modèle-é.typ");
    std::fs::write(&model, &template).unwrap();
    let guard = e.target_guard();
    call(
        &mut e,
        "document.set_model",
        json!({"guard":guard,"model":{"kind":"path","path":model}}),
    )
    .unwrap();
    assert_eq!(
        &*e.dependencies["packages/PROVENANCE.md"],
        b"Exemplaire du document"
    );
    std::fs::remove_file(&model).unwrap();
    let stored = e.stored_document();
    let archive = dir.path().join("autonome.typsto");
    crate::storage::save(&archive, &stored).unwrap();
    let reopened = crate::storage::read(&archive).unwrap();
    assert!(reopened == stored);
    let mut world = crate::rendering::TypstWorld::default();
    world.set_dependencies(&reopened.dependencies);
    world.set_assets(&reopened.assets);
    world
        .compile_with_template(&reopened.source, &reopened.metadata, &reopened.template)
        .unwrap();
}

#[test]
fn resource_budget_failure_never_publishes_a_prepared_model() {
    let mut e = engine("Bonjour");
    let inflated: std::sync::Arc<[u8]> = vec![b'x'; crate::assets::MAX_IMAGE_BYTES].into();
    let names: Vec<_> = e
        .dependencies
        .keys()
        .filter(|s| {
            s.ends_with("/LICENSE") || s.ends_with("/README.md") || s.ends_with("/PROVENANCE.md")
        })
        .cloned()
        .collect();
    assert!(names.len() >= 5);
    for name in names {
        std::sync::Arc::make_mut(&mut e.dependencies).insert(name, inflated.clone());
    }
    let before = e.stored_document();
    let state = (e.revision, e.undo.len(), e.redo.len(), e.selection());
    let template = format!("{}\n// Modèle dérivé", Model::Personal.source());
    assert_eq!(
        install(&mut e, &template).unwrap_err().reason(),
        "document_too_large"
    );
    assert!(e.stored_document() == before);
    assert_eq!(
        (e.revision, e.undo.len(), e.redo.len(), e.selection()),
        state
    );
}
