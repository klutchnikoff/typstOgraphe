use super::*;
use crate::engine::{self, Call, Empty, Query};
use serde_json::json;

#[test]
fn profile_creation_matches_explicit_json_values_and_never_changes_global_preferences() {
    let (_ctx, mut app) = demo_app();
    app.preferences.profile.name = "Zoé Martin".into();
    app.preferences.profile.address = "12 rue des Fleurs\nRennes".into();
    app.preferences.profile.signature = Some("signature-profile.svg".into());
    let image = crate::assets::ImageData {
        extension: "svg".into(),
        data: include_bytes!("../../../tests/fixtures/signature.svg")
            .as_slice()
            .into(),
    };
    app.profile_signature = Some(image.clone());
    let prefs = app.preferences.clone();
    let mut core = engine::Engine::empty();
    core.run_call(Call{id:"document.new".to_owned().try_into().unwrap(),params:json!({"guard":core.target_guard(),"type":"courrier","model":"courrier.model.personal",
        "initial_values":[{"field":"sender.name","value":"Zoé Martin"},{"field":"sender.address","value":"12 rue des Fleurs\nRennes"}],"signature":engine::ImageInput::from(&image)})}).unwrap();
    app.create_document(DocumentType::Courrier);
    assert!(app.stored_document() == core.stored_document());
    assert_eq!(app.preferences, prefs);
    assert!(app.document.open && app.document.engine.is_open());
    assert!(!app.is_dirty() && app.document.engine.undo().is_empty());
    assert!(app.preview.pages.is_empty() && app.preview.sync.is_none());
    assert_eq!(app.document.selection.primary.index, 0);
}

#[test]
fn file_adapter_and_json_preserve_history_on_save_and_reset_caches_only_after_successful_open() {
    let (_ctx, mut app) = demo_app();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.typsto");
    app.document.selection = editor::cursor(10..24);
    app.apply(Command::Bold);
    let stored = app.stored_document();
    let mut core: engine::Engine = Session::new(
        stored.clone(),
        Document::from_source(&stored.source).unwrap(),
        None,
        0,
    )
    .into();
    let selection = app.document.selection;
    app.document.force_selection = false;
    app.preview.forward_pending = false;
    let undo = app.document.engine.undo().len();
    assert!(app.save_path(path.clone()));
    core.run_call(
        engine::Command::SaveDocument(engine::SaveDocumentParams {
            guard: core.target_guard(),
            path: Some(dir.path().join("core.typsto")),
        })
        .to_call(),
    )
    .unwrap();
    assert!(!app.is_dirty() && !core.is_dirty());
    assert_eq!(app.document.selection, selection);
    assert_eq!(app.document.engine.undo().len(), undo);
    assert!(!app.document.force_selection && !app.preview.forward_pending);
    assert!(app.stored_document() == core.stored_document());
    let guard = app.document.engine.target_guard();
    let bad = dir.path().join("bad.typsto");
    std::fs::write(&bad, "invalid").unwrap();
    app.search.open = true;
    assert!(!app.open_path(bad));
    assert!(app.search.open);
    assert_eq!(app.document.engine.target_guard(), guard);
    assert_eq!(app.document.selection, selection);
    assert_eq!(app.document.engine.undo().len(), undo);
    assert!(app.open_path(path.clone()));
    core.run_call(
        engine::Command::OpenDocument(engine::OpenDocumentParams {
            guard: core.target_guard(),
            path,
            discard_changes: false,
        })
        .to_call(),
    )
    .unwrap();
    assert!(app.stored_document() == core.stored_document());
    assert!(!app.search.open && app.document.engine.undo().is_empty());
    assert!(app.preview.pages.is_empty() && app.preview.revision.is_none());
    assert_ne!(
        app.document.engine.target_guard().document_id,
        guard.document_id
    );
    let stale = app
        .execute_document_command(engine::Command::SaveDocument(engine::SaveDocumentParams {
            guard,
            path: Some(dir.path().join("must-not-exist.typsto")),
        }))
        .unwrap_err();
    assert_eq!(stale.reason(), "foreign_document");
    assert!(!dir.path().join("must-not-exist.typsto").exists());
    app.close_document();
    assert!(!app.document.open && !app.document.engine.is_open());
}

#[test]
fn asynchronous_export_uses_the_requested_snapshot_and_keeps_edits_and_history() {
    let (ctx, mut app) = demo_app();
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("export");
    let before = app.stored_document();
    let prefs = app.preferences.clone();
    app.document.selection = editor::cursor(0..5);
    let selection = app.document.selection;
    assert!(app.export_document(
        engine::ExportParams {
            guard: app.document.engine.target_guard(),
            directory: output.clone(),
            format: engine::ExportFormat::Typst,
            page_images: false
        },
        &ctx
    ));
    assert_eq!(app.document.selection, selection);
    app.apply(Command::Italic);
    let edited = app.stored_document();
    let undo = app.document.engine.undo().len();
    let result = app
        .file
        .export_task
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_secs(30))
        .unwrap()
        .unwrap();
    app.file.export_task = None;
    assert_eq!(result["directory"], output.to_str().unwrap());
    assert_eq!(
        std::fs::read_to_string(output.join("contenu.typ")).unwrap(),
        before.source
    );
    assert!(!output.join("courrier.pdf").exists() && !output.join("page-1.png").exists());
    assert!(app.stored_document() == edited);
    assert_eq!(app.document.engine.undo().len(), undo);
    assert_eq!(app.preferences, prefs);
    assert!(!app.export_document(
        engine::ExportParams {
            guard: app.document.engine.target_guard(),
            directory: output.clone(),
            format: engine::ExportFormat::Both,
            page_images: true
        },
        &ctx
    ));
    assert!(app.file.export_task.is_none());
    assert!(app.dialogs.message.as_ref().unwrap().contains("vide"));
}

#[test]
fn recovery_ui_cache_uses_engine_references_and_global_configuration_survives_document_changes() {
    let (ctx, mut app) = demo_app();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("recovery");
    let stored = StoredDocument::demo(DocumentType::Courrier);
    seed_recovery(&root, &stored, Some("original.typsto".into()));
    app.start_recovery(root, ctx);
    let reference = app.recovery.entries[0].reference;
    assert_eq!(
        app.document
            .engine
            .query(Query::RecoveryList(Empty {}))
            .unwrap()["entries"][0]["reference"],
        json!(reference)
    );
    let prefs = app.preferences.clone();
    app.close_document();
    assert_eq!(app.recovery.entries[0].reference, reference);
    app.recover_document(0);
    app.recovery.worker.as_mut().unwrap().wait();
    assert!(app.stored_document() == stored);
    assert_eq!(app.preferences, prefs);
    assert!(app.document.open && app.is_dirty() && app.document.engine.is_recovered());
    assert!(app.document.engine.file_path().map(PathBuf::from).is_none());
    assert!(app.recovery.entries.is_empty());
    assert_eq!(
        app.document
            .engine
            .query(Query::RecoveryList(Empty {}))
            .unwrap()["entries"],
        json!([])
    );
}
