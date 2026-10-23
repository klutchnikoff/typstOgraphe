use super::*;
use crate::document_types::MetadataField;

fn new_params(e: &Engine) -> NewDocumentParams {
    NewDocumentParams {
        guard: e.target_guard(),
        document_type: DocumentType::Courrier,
        model: "courrier.model.personal".into(),
        initial_values: vec![],
        signature: None,
        discard_changes: false,
    }
}
fn changed(e: &mut Engine) {
    call(e, "text.insert", json!({"text":"Été *littéral*."})).unwrap();
}
#[test]
fn empty_creation_uses_explicit_values_and_model_catalog_without_global_defaults() {
    let mut e = Engine::empty();
    assert!(!e.is_open());
    assert_eq!(e.query(Query::Inspect(Empty {})).unwrap()["open"], false);
    assert_eq!(
        e.run(Command::Bold(StyleParams::default()))
            .unwrap_err()
            .reason(),
        "no_document"
    );
    let old = e.target_guard();
    let mut p = new_params(&e);
    p.initial_values = vec![
        InitialValue {
            field: MetadataField::SenderName,
            value: "Zoé #nom".into(),
        },
        InitialValue {
            field: MetadataField::LetterDate,
            value: "2026-10-23".into(),
        },
    ];
    p.signature = Some(ImageInput::from(&crate::assets::ImageData {
        data: include_bytes!("../../../tests/fixtures/signature.svg")
            .as_slice()
            .into(),
        extension: "svg".into(),
    }));
    let out = e.run_call(Command::NewDocument(p).to_call()).unwrap();
    assert!(out.effects.contains(&Effect::DocumentReplaced));
    assert_ne!(old.document_id, e.target_guard().document_id);
    assert!(e.is_open() && !e.is_dirty());
    assert!(e.session.document.source().is_empty());
    assert_eq!(e.session.metadata.courrier().expediteur, "Zoé #nom");
    assert_eq!(e.session.metadata.courrier().date, "2026-10-23");
    assert!(e.session.metadata.courrier().destinataire.is_empty());
    assert!(e.session.metadata.signature_path().is_some());
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    assert_eq!(e.session.template, DocumentType::Courrier.default_model());
}
#[test]
fn replacement_requires_a_current_guard_and_explicit_discard_before_reading_paths() {
    let mut e = engine("Bonjour");
    let old = e.target_guard();
    changed(&mut e);
    let stored = e.stored_document();
    let current = e.target_guard();
    for (guard, discard, expected) in [
        (old, true, "stale_revision"),
        (current, false, "unsaved_changes"),
    ] {
        let error = e
            .run(Command::OpenDocument(OpenDocumentParams {
                guard,
                path: "absent.typsto".into(),
                discard_changes: discard,
            }))
            .unwrap_err();
        assert_eq!(error.reason(), expected);
        assert!(e.stored_document() == stored);
        assert_eq!(e.target_guard(), current);
    }
    let mut p = new_params(&e);
    p.discard_changes = true;
    p.model = "absent".into();
    assert_eq!(
        e.run(Command::NewDocument(p)).unwrap_err().reason(),
        "creation_model"
    );
    assert!(e.stored_document() == stored);
    let outcome = e
        .run(Command::CloseDocument(CloseDocumentParams {
            guard: current,
            discard_changes: true,
        }))
        .unwrap();
    assert!(outcome.changed && !e.is_open());
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    assert_eq!(
        e.run(Command::SaveDocument(SaveDocumentParams {
            guard: e.target_guard(),
            path: None
        }))
        .unwrap_err()
        .reason(),
        "no_document"
    );
}
#[test]
fn new_refuses_unknown_type_fields_dynamic_dates_and_duplicate_initial_values_atomically() {
    for initial_values in [
        vec![InitialValue {
            field: MetadataField::MeetingTitle,
            value: "Titre".into(),
        }],
        vec![InitialValue {
            field: MetadataField::LetterDate,
            value: "today".into(),
        }],
        vec![InitialValue {
            field: MetadataField::SenderName,
            value: "A\nB".into(),
        }],
        vec![
            InitialValue {
                field: MetadataField::SenderName,
                value: "A".into()
            };
            2
        ],
    ] {
        let mut e = Engine::empty();
        let before = e.target_guard();
        let mut p = new_params(&e);
        p.initial_values = initial_values;
        assert!(e.run(Command::NewDocument(p)).is_err());
        assert!(!e.is_open());
        assert_eq!(e.target_guard(), before);
    }
    let mut e = Engine::empty();
    let mut p = new_params(&e);
    p.document_type = DocumentType::CompteRendu;
    assert_eq!(
        e.run(Command::NewDocument(p)).unwrap_err().reason(),
        "creation_model"
    );
}
#[test]
fn saves_are_atomic_preserve_history_and_save_as_leaves_the_old_file_intact() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.typsto");
    let second = directory.path().join("second.typsto");
    let mut e = engine("Avant");
    changed(&mut e);
    let guard = e.target_guard();
    let selection = e.selection();
    let undo = e.session.undo.len();
    assert_eq!(
        e.run(Command::SaveDocument(SaveDocumentParams {
            guard,
            path: None
        }))
        .unwrap_err()
        .reason(),
        "save_path_required"
    );
    let out = e
        .run(Command::SaveDocument(SaveDocumentParams {
            guard,
            path: Some(first.clone()),
        }))
        .unwrap();
    assert!(!out.changed && out.effects.contains(&Effect::Saved));
    assert!(!e.is_dirty());
    assert_eq!(e.session.undo.len(), undo);
    assert_eq!(e.selection(), selection);
    let saved = crate::storage::read(&first).unwrap();
    changed(&mut e);
    let dirty = e.stored_document();
    let guard = e.target_guard();
    assert!(
        e.run(Command::SaveAs(SaveAsParams {
            guard,
            overwrite: false,
            path: directory.path().join("absent/file.typsto")
        }))
        .is_err()
    );
    assert!(e.stored_document() == dirty);
    assert!(e.is_dirty());
    assert_eq!(e.session.file_path, Some(first.clone()));
    e.run_call(
        Command::SaveAs(SaveAsParams {
            guard,
            overwrite: false,
            path: second.clone(),
        })
        .to_call(),
    )
    .unwrap();
    assert!(crate::storage::read(&first).unwrap() == saved);
    assert!(crate::storage::read(&second).unwrap() == dirty);
    assert_eq!(e.session.file_path, Some(second));
    assert!(!e.is_dirty());
    e.run(Command::Undo(Empty {})).unwrap();
    assert!(e.is_dirty());
    e.run(Command::Redo(Empty {})).unwrap();
    assert!(!e.is_dirty());
}
#[test]
fn open_replaces_identity_and_history_but_corrupt_archives_leave_the_session_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let stored = StoredDocument::demo(DocumentType::Courrier);
    crate::storage::save(&path, &stored).unwrap();
    let mut e = engine("Brouillon");
    changed(&mut e);
    let old = e.target_guard();
    e.run(Command::OpenDocument(OpenDocumentParams {
        guard: old,
        path: path.clone(),
        discard_changes: true,
    }))
    .unwrap();
    assert!(e.stored_document() == stored);
    assert_eq!(e.session.file_path, Some(path));
    assert!(!e.is_dirty());
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    assert_eq!(e.selection(), Selection::default());
    assert_ne!(e.target_guard().document_id, old.document_id);
    let guard = e.target_guard();
    let bad = dir.path().join("bad.typsto");
    std::fs::write(&bad, b"bad").unwrap();
    assert!(
        e.run(Command::OpenDocument(OpenDocumentParams {
            guard,
            path: bad,
            discard_changes: false
        }))
        .is_err()
    );
    assert!(e.stored_document() == stored);
    assert_eq!(e.target_guard(), guard);
}
#[test]
fn export_formats_defaults_and_immutable_background_snapshot_share_the_same_writer() {
    let dir = tempfile::tempdir().unwrap();
    let mut e: Engine = Session::new(
        StoredDocument::demo(DocumentType::Courrier),
        Document::from_source(DocumentType::Courrier.demo_source()).unwrap(),
        None,
        0,
    )
    .into();
    let before = e.stored_document();
    let guard = e.target_guard();
    for (format, images) in [
        (ExportFormat::Pdf, false),
        (ExportFormat::Typst, false),
        (ExportFormat::Both, true),
    ] {
        let path = dir.path().join(format!("{format:?}"));
        e.run(Command::Export(ExportParams {
            guard,
            directory: path.clone(),
            format,
            page_images: images,
        }))
        .unwrap();
        assert_eq!(
            path.join("courrier.pdf").exists(),
            format != ExportFormat::Typst
        );
        assert_eq!(path.join("main.typ").exists(), format != ExportFormat::Pdf);
        assert_eq!(path.join("page-1.png").exists(), images);
        if format != ExportFormat::Pdf {
            assert_eq!(
                std::fs::read_to_string(path.join("contenu.typ")).unwrap(),
                before.source
            );
            for name in before.dependencies.keys() {
                assert!(path.join(name).is_file(), "{name}");
            }
        }
        let pdf = std::fs::read_dir(&path).unwrap().count();
        assert!(
            e.run(Command::Export(ExportParams {
                guard,
                directory: path.clone(),
                format,
                page_images: images
            }))
            .is_err()
        );
        assert_eq!(std::fs::read_dir(path).unwrap().count(), pdf);
    }
    let path = dir.path().join("snapshot");
    let params: ExportParams =
        serde_json::from_value(json!({"guard":guard,"directory":path})).unwrap();
    assert_eq!(params.format, ExportFormat::Both);
    assert!(params.page_images);
    let task = e.prepare_export(params).unwrap();
    changed(&mut e);
    task.execute().unwrap();
    assert_eq!(
        std::fs::read_to_string(path.join("contenu.typ")).unwrap(),
        before.source
    );
    assert_eq!(e.session.undo.len(), 1);
    assert!(e.is_dirty());
}
#[test]
fn export_refuses_required_fields_stale_guards_and_invalid_models_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export");
    let mut e = engine("");
    assert_eq!(
        e.run(Command::Export(ExportParams {
            guard: e.target_guard(),
            directory: path.clone(),
            format: ExportFormat::Both,
            page_images: true
        }))
        .unwrap_err()
        .reason(),
        "export_required_fields"
    );
    assert!(!path.exists());
    e.session.metadata = Metadata::demo(DocumentType::Courrier);
    e.session.template = "#let broken = unknown".into();
    assert!(
        e.run(Command::Export(ExportParams {
            guard: e.target_guard(),
            directory: path.clone(),
            format: ExportFormat::Both,
            page_images: true
        }))
        .is_err()
    );
    assert!(!path.exists());
}

fn seed(root: &std::path::Path, stored: &StoredDocument) {
    let (mut recovery, _) = crate::recovery::Recovery::start(root, Default::default()).unwrap();
    recovery.make_due();
    recovery
        .checkpoint(1, true, || (stored.clone(), Some("original.typsto".into())))
        .unwrap();
    recovery.wait();
    assert!(recovery.poll().iter().all(Result::is_ok));
}
#[test]
fn recovery_references_are_service_scoped_and_restoration_is_a_dirty_copy() {
    let dir = tempfile::tempdir().unwrap();
    let stored = StoredDocument::demo(DocumentType::Courrier);
    seed(dir.path(), &stored);
    let (mut service, entries) =
        crate::recovery::Recovery::start(dir.path(), Default::default()).unwrap();
    let mut e = engine("Brouillon");
    e.configure_recovery(entries);
    let listing = e
        .query_call(Query::RecoveryList(Empty {}).to_call())
        .unwrap();
    let reference: RecoveryRef =
        serde_json::from_value(listing["entries"][0]["reference"].clone()).unwrap();
    assert_eq!(listing["entries"][0]["name"], "original.typsto");
    changed(&mut e);
    let guard = e.target_guard();
    let before = e.stored_document();
    let params = RestoreRecoveryParams {
        guard,
        reference,
        discard_changes: false,
    };
    assert_eq!(
        e.run(Command::RestoreRecovery(params.clone()))
            .unwrap_err()
            .reason(),
        "unsaved_changes"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.recovery_items().len(), 1);
    let mut other = Engine::empty();
    assert_eq!(
        other
            .run(Command::DeleteRecovery(DeleteRecoveryParams { reference }))
            .unwrap_err()
            .reason(),
        "recovery_unconfigured"
    );
    other.configure_recovery(vec![]);
    assert_eq!(
        other
            .run(Command::DeleteRecovery(DeleteRecoveryParams { reference }))
            .unwrap_err()
            .reason(),
        "recovery_not_found"
    );
    let out = e
        .run_call(
            Command::RestoreRecovery(RestoreRecoveryParams {
                discard_changes: true,
                ..params
            })
            .to_call(),
        )
        .unwrap();
    assert!(out.effects.contains(&Effect::DocumentReplaced));
    assert!(e.stored_document() == stored);
    assert!(e.session.recovered && e.is_dirty());
    assert!(e.session.file_path.is_none());
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    assert!(e.recovery_items().is_empty());
    assert_ne!(e.target_guard().document_id, guard.document_id);
    service.adopt(e.take_recovered_entry().unwrap()).unwrap();
    service.wait();
    let copy = dir.path().join("copy.typsto");
    e.run(Command::SaveDocument(SaveDocumentParams {
        guard: e.target_guard(),
        path: Some(copy.clone()),
    }))
    .unwrap();
    service.clear().unwrap();
    service.wait();
    assert!(crate::storage::read(&copy).unwrap() == stored);
    assert!(!e.is_dirty());
}
#[test]
fn deleting_one_recovery_copy_preserves_document_history_and_invalid_refs_fail() {
    let dir = tempfile::tempdir().unwrap();
    let stored = StoredDocument::demo(DocumentType::Courrier);
    seed(dir.path(), &stored);
    let (_service, entries) =
        crate::recovery::Recovery::start(dir.path(), Default::default()).unwrap();
    let mut e = engine("Brouillon");
    changed(&mut e);
    let before = e.stored_document();
    let guard = e.target_guard();
    let undo = e.session.undo.len();
    e.configure_recovery(entries);
    let reference = e.recovery_items()[0].reference;
    e.run_call(Command::DeleteRecovery(DeleteRecoveryParams { reference }).to_call())
        .unwrap();
    assert!(e.stored_document() == before);
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.session.undo.len(), undo);
    assert!(e.recovery_items().is_empty());
    assert_eq!(
        e.run(Command::DeleteRecovery(DeleteRecoveryParams { reference }))
            .unwrap_err()
            .reason(),
        "recovery_not_found"
    );
    assert!(
        Command::from_call(
            "recovery.restore",
            json!({"guard":guard,"path":"arbitrary.recovery"})
        )
        .is_err()
    );
}

#[test]
fn save_as_refuses_existing_targets_unless_replacement_is_explicit_and_publication_cannot_clobber()
{
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.typsto");
    let original = StoredDocument::demo(DocumentType::Courrier);
    crate::storage::save(&path, &original).unwrap();
    let original_bytes = std::fs::read(&path).unwrap();
    let mut e = engine("Autre document");
    changed(&mut e);
    let before = e.stored_document();
    let guard = e.target_guard();
    let undo = e.session.undo.len();
    let params = SaveAsParams {
        guard,
        path: path.clone(),
        overwrite: false,
    };
    assert_eq!(
        e.run(Command::SaveAs(params.clone())).unwrap_err().reason(),
        "file_exists"
    );
    assert_eq!(
        e.run(Command::SaveDocument(SaveDocumentParams {
            guard,
            path: Some(path.clone())
        }))
        .unwrap_err()
        .reason(),
        "file_exists"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.session.undo.len(), undo);
    assert!(e.session.file_path.is_none() && e.is_dirty());
    // La primitive de publication refuse aussi une cible apparue après un contrôle préalable.
    assert!(crate::storage::save_new(&path, &before).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original_bytes);
    e.run(Command::SaveAs(SaveAsParams {
        overwrite: true,
        ..params
    }))
    .unwrap();
    assert!(crate::storage::read(&path).unwrap() == before);
    assert!(!e.is_dirty());
    assert_eq!(e.session.undo.len(), undo);
}
#[test]
fn corrupt_recovery_refuses_publication_and_stale_export_refuses_filesystem_effects() {
    let dir = tempfile::tempdir().unwrap();
    let stored = StoredDocument::demo(DocumentType::Courrier);
    seed(dir.path(), &stored);
    let (_service, entries) =
        crate::recovery::Recovery::start(dir.path(), Default::default()).unwrap();
    let mut e = engine("À conserver");
    e.configure_recovery(entries);
    let reference = e.recovery_items()[0].reference;
    let snapshot = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("document.recovery"))
        .find(|p| p.exists())
        .unwrap();
    std::fs::write(snapshot, "broken").unwrap();
    let before = e.stored_document();
    let guard = e.target_guard();
    assert_eq!(
        e.run(Command::RestoreRecovery(RestoreRecoveryParams {
            guard,
            reference,
            discard_changes: true
        }))
        .unwrap_err()
        .reason(),
        "recovery_load"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.recovery_items().len(), 1);
    changed(&mut e);
    let path = dir.path().join("must-not-exist");
    assert_eq!(
        e.run(Command::Export(ExportParams {
            guard,
            directory: path.clone(),
            format: ExportFormat::Both,
            page_images: true
        }))
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    assert!(!path.exists());
}
