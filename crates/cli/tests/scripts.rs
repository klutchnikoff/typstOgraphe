#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
use assert_cmd::{Command as ProcessCommand, cargo::cargo_bin_cmd};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use typstographe_core::{
    document::{BlockKind, Document},
    document_types::DocumentType,
    engine::*,
    storage::{self, StoredDocument},
};

fn cli() -> ProcessCommand {
    let mut command = cargo_bin_cmd!("typstographe-cli");
    command.timeout(std::time::Duration::from_secs(60));
    command
}
fn response(command: &mut ProcessCommand, exit: i32) -> Value {
    let assertion = command.arg("--json").assert().code(exit);
    let output = assertion.get_output();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn job(folder: &Path, value: Value) -> PathBuf {
    let path = folder.join("job.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    path
}
fn fixture(path: &Path) -> StoredDocument {
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.source = "Avant Élodie.\n".into();
    storage::save(path, &stored).unwrap();
    stored
}
fn script(steps: Value, save: Value) -> Value {
    json!({"version":1,"document":{"kind":"open","path":"original.typsto"},"steps":steps,"save":save})
}
fn assert_bold(source: &str, word: &str) {
    let document = Document::from_source(source).unwrap();
    let at = document.view.text.find(word).unwrap();
    let start = document.view.text[..at].chars().count();
    assert!(
        document.view.styles[start..start + word.chars().count()]
            .iter()
            .all(|style| style.bold)
    );
}

#[test]
fn a_new_letter_with_metadata_postscript_and_enclosure_is_saved_once_and_exported_separately() {
    let dir = tempfile::tempdir().unwrap();
    let initial: Vec<_> = StoredDocument::demo(DocumentType::Courrier)
        .metadata
        .field_values()
        .into_iter()
        .filter(|f| f.required_for_export)
        .map(|f| json!({"field":f.field,"value":f.value}))
        .collect();
    let path = job(
        dir.path(),
        json!({"version":1,"document":{"kind":"new","type":"courrier","params":{"initial_values":initial}},"steps":[
        {"id":"metadata.set","params":{"field":"letter.subject","value":"Essai 日本"}},
        {"id":"text.insert","params":{"text":"Bonjour Élodie.\n\nVoici notre courrier."}},
        {"id":"text.bold","target":{"select":"Élodie"}},
        {"id":"letter.insert_postscript"},
        {"id":"text.insert","params":{"text":"Une précision utile."}},
        {"id":"letter.insert_attachment"},
        {"id":"text.insert","params":{"text":"Formulaire signé"}}
    ],"save":{"kind":"as","path":"courrier.typsto"}}),
    );
    let result = response(cli().arg("script").arg(&path), 0);
    assert_eq!(result["outcome"]["saved"], true);
    assert_eq!(result["outcome"]["batch"]["value"]["steps"], 7);
    let output = dir.path().join("courrier.typsto");
    let stored = storage::read(&output).unwrap();
    let document = Document::from_source(&stored.source).unwrap();
    assert!(document.view.blocks.contains(&BlockKind::Postscript));
    assert!(document.view.blocks.contains(&BlockKind::Attachment));
    assert!(document.view.text.contains("Une précision utile."));
    assert!(document.view.text.contains("Formulaire signé"));
    assert_bold(&stored.source, "Élodie");
    let mut reopened = Engine::empty();
    reopened
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: reopened.target_guard(),
            path: output.clone(),
            read_only: false,
            discard_changes: false,
        }))
        .unwrap();
    assert_eq!(reopened.stored_document(), stored);
    response(cli().arg("check").arg(&output).arg("--for-export"), 0);
    let exported = dir.path().join("export");
    response(
        cli()
            .arg("export")
            .arg(&output)
            .args(["--format", "both", "--output"])
            .arg(&exported),
        0,
    );
    assert!(
        std::fs::read(exported.join("courrier.pdf"))
            .unwrap()
            .starts_with(b"%PDF")
    );
    assert_eq!(
        std::fs::read_to_string(exported.join("contenu.typ")).unwrap(),
        stored.source
    );
}

#[test]
fn current_save_and_late_failure_preserve_the_previous_file_and_history_is_one_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.typsto");
    fixture(&original);
    let before = std::fs::read(&original).unwrap();
    let path = job(
        dir.path(),
        script(
            json!([
                {"id":"metadata.set","params":{"field":"letter.subject","value":"Réunion"}},
                {"id":"text.insert","params":{"text":"Début "}},
                {"id":"text.bold","target":{"select":"Introuvable"}}
            ]),
            json!({"kind":"current"}),
        ),
    );
    let error = response(cli().arg("script").arg(&path), 1);
    assert_eq!(error["error"]["params"]["step"], 2);
    assert_eq!(error["error"]["params"]["command"], "text.bold");
    assert_eq!(std::fs::read(&original).unwrap(), before);
    let path = job(
        dir.path(),
        script(
            json!([
                {"id":"metadata.set","params":{"field":"letter.subject","value":"Réunion"}},
                {"id":"text.insert","params":{"text":"Début "}},
                {"id":"text.bold","target":{"select":"Élodie"}}
            ]),
            json!({"kind":"current"}),
        ),
    );
    let result = response(cli().arg("script").arg(&path), 0);
    assert_eq!(result["outcome"]["document"]["can_undo"], true);
    assert_bold(&storage::read(&original).unwrap().source, "Élodie");
    let mut engine = Engine::empty();
    engine
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: engine.target_guard(),
            path: original,
            read_only: false,
            discard_changes: false,
        }))
        .unwrap();
}

#[test]
fn targets_and_import_paths_are_resolved_after_previous_steps_relative_to_the_script() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.typsto");
    let before = fixture(&original);
    let jobs = dir.path().join("jobs");
    std::fs::create_dir(&jobs).unwrap();
    std::fs::write(
        jobs.join("image.svg"),
        include_bytes!("../../../fixtures/signature.svg"),
    )
    .unwrap();
    std::fs::write(
        jobs.join("model.typ"),
        format!("{}\n// script model\n", before.template),
    )
    .unwrap();
    let path = job(
        &jobs,
        json!({"version":1,"document":{"kind":"open","path":"../original.typsto"},"steps":[
        {"id":"document.set_model","params":{"model":{"kind":"path","path":"model.typ"}}},
        {"id":"insert.image","params":{"image":{"kind":"path","path":"image.svg"},"caption":"Légende"}},
        {"id":"image.set_alt","params":{"alt":"Une image 日本"},"target":{"block":1}},
        {"id":"text.insert","params":{"text":"Légende révisée"},"target":{"select":"Légende"}},
        {"id":"metadata.set","params":{"field":"letter.subject","value":"image.svg"}}
    ],"save":{"kind":"as","path":"result.typsto"}}),
    );
    response(cli().current_dir(dir.path()).arg("script").arg(&path), 0);
    let stored = storage::read(&jobs.join("result.typsto")).unwrap();
    assert!(stored.source.contains("Une image 日本"));
    assert!(stored.source.contains("Légende révisée"));
    assert!(stored.template.ends_with("// script model\n"));
    assert!(
        stored
            .metadata
            .field_values()
            .iter()
            .any(|f| f.value == "image.svg")
    );
    assert_eq!(stored.assets.len(), 1);
    assert_eq!(storage::read(&original).unwrap(), before);
    assert!(!dir.path().join("result.typsto").exists());
}

#[test]
fn signature_import_and_inserted_then_deleted_images_use_the_common_resource_policy() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("signature.svg"),
        include_bytes!("../../../fixtures/signature.svg"),
    )
    .unwrap();
    let path = job(
        dir.path(),
        json!({"version":1,"document":{"kind":"new","type":"courrier","params":{"signature":{"kind":"path","path":"signature.svg"}}},"steps":[
        {"id":"insert.image","params":{"image":{"kind":"path","path":"signature.svg"}}},
        {"id":"image.remove","target":{"block":1}},
        {"id":"text.insert","params":{"text":"Corps du courrier."}}
    ],"save":{"kind":"as","path":"signed.typsto"}}),
    );
    response(cli().arg("script").arg(&path), 0);
    let stored = storage::read(&dir.path().join("signed.typsto")).unwrap();
    assert!(
        stored
            .assets
            .contains_key(stored.metadata.signature_path().unwrap())
    );
    assert_eq!(stored.assets.len(), 1);
    assert!(stored.source.contains("Corps du courrier."));
}

#[test]
fn preview_and_copy_can_read_an_open_document_but_in_place_script_saving_respects_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.typsto");
    fixture(&original);
    let before = std::fs::read(&original).unwrap();
    let mut owner = Engine::empty();
    owner
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: owner.target_guard(),
            path: original.clone(),
            read_only: false,
            discard_changes: false,
        }))
        .unwrap();
    owner
        .run(Command::InsertText(InsertTextParams {
            target: TextTarget::default(),
            text: "Non enregistré ".into(),
            input_rules: InputRules::Literal,
        }))
        .unwrap();
    let steps = json!([{"id":"text.insert","params":{"text":"Aperçu "}}]);
    let path = job(dir.path(), script(steps.clone(), Value::Null));
    let preview = response(cli().arg("script").arg(&path), 0);
    assert_eq!(preview["outcome"]["saved"], false);
    assert!(
        !preview["outcome"]["source"]
            .as_str()
            .unwrap()
            .contains("Non enregistré")
    );
    assert_eq!(std::fs::read(&original).unwrap(), before);
    let path = job(
        dir.path(),
        script(steps.clone(), json!({"kind":"as","path":"copy.typsto"})),
    );
    response(cli().arg("script").arg(&path), 0);
    assert!(
        storage::read(&dir.path().join("copy.typsto"))
            .unwrap()
            .source
            .starts_with("Aperçu ")
    );
    let path = job(dir.path(), script(steps, json!({"kind":"current"})));
    assert_eq!(
        response(cli().arg("script").arg(&path), 1)["error"]["params"]["reason"],
        "file_locked"
    );
    assert_eq!(std::fs::read(&original).unwrap(), before);
}

#[test]
fn final_save_failure_cannot_overwrite_a_destination_or_publish_an_intermediate_document() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.typsto");
    fixture(&original);
    let before = std::fs::read(&original).unwrap();
    let target = dir.path().join("existing.typsto");
    std::fs::write(&target, b"sentinel").unwrap();
    let steps = json!([{"id":"text.insert","params":{"text":"Modification "}}]);
    for destination in ["existing.typsto", "absent/result.typsto", "original.typsto"] {
        let path = job(
            dir.path(),
            script(steps.clone(), json!({"kind":"as","path":destination})),
        );
        assert_eq!(response(cli().arg("script").arg(&path), 3)["ok"], false);
        assert_eq!(std::fs::read(&original).unwrap(), before);
        assert_eq!(std::fs::read(&target).unwrap(), b"sentinel");
        assert!(!dir.path().join("absent").exists());
    }
    let path = job(
        dir.path(),
        json!({"version":1,"document":{"kind":"new","type":"courrier"},"steps":[
        {"id":"text.insert","params":{"text":"Non publié"}},
        {"id":"metadata.set","params":{"field":"meeting.title","value":"Invalide"}}
    ],"save":{"kind":"as","path":"not-created.typsto"}}),
    );
    response(cli().arg("script").arg(&path), 1);
    assert!(!dir.path().join("not-created.typsto").exists());
}

#[test]
fn forbidden_operations_and_bad_target_options_are_rejected_before_opening_files_or_importing_assets()
 {
    let dir = tempfile::tempdir().unwrap();
    for id in [
        "document.save",
        "document.save_as",
        "document.export",
        "document.new",
        "document.open",
        "document.close",
        "document.batch",
        "metadata.get",
        "recovery.delete",
        "edit.undo",
        "unknown.action",
    ] {
        let path = job(
            dir.path(),
            json!({"version":1,"document":{"kind":"new","type":"courrier","params":{"signature":{"kind":"path","path":"missing.svg"}}},"steps":[{"id":id,"params":{}}]}),
        );
        let result = response(cli().arg("script").arg(&path), 1);
        assert_eq!(result["error"]["params"]["step"], 0);
        assert_eq!(
            result["error"]["params"]["error"]["params"]["reason"],
            if id == "unknown.action" {
                "unknown_command"
            } else {
                "batch_step_forbidden"
            }
        );
    }
    for target in [
        json!({"block":0}),
        json!({"select":"x","block":1}),
        json!({"occurrence":1}),
    ] {
        let path = job(
            dir.path(),
            script(json!([{"id":"text.bold","target":target}]), Value::Null),
        );
        assert_eq!(
            response(cli().arg("script").arg(&path), 1)["error"]["params"]["error"]["params"]["reason"],
            "cli_target_options"
        );
    }
    let path = job(
        dir.path(),
        script(
            json!([{"id":"metadata.set","params":{"guard":{"document_id":1,"revision":1}}}]),
            Value::Null,
        ),
    );
    assert_eq!(
        response(cli().arg("script").arg(&path), 1)["error"]["params"]["error"]["params"]["reason"],
        "cli_session_reference"
    );
}

#[test]
fn the_envelope_is_versioned_strict_and_bounded_and_stdin_uses_the_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.typsto");
    fixture(&original);
    let steps = json!([{"id":"text.insert","params":{"text":"Depuis stdin "}}]);
    let payload = script(steps.clone(), json!({"kind":"as","path":"stdin.typsto"}));
    response(
        cli()
            .current_dir(dir.path())
            .args(["script", "-"])
            .write_stdin(payload.to_string()),
        0,
    );
    assert!(dir.path().join("stdin.typsto").is_file());
    for (payload, reason) in [
        (
            json!({"version":2,"document":{"kind":"open","path":"missing"},"steps":steps}),
            "cli_script_version",
        ),
        (script(json!([]), Value::Null), "batch_step_limit"),
        (
            script(
                json!(
                    (0..257)
                        .map(|_| json!({"id":"text.bold"}))
                        .collect::<Vec<_>>()
                ),
                Value::Null,
            ),
            "batch_step_limit",
        ),
        (
            json!({"version":1,"document":{"kind":"new","type":"courrier"},"steps":steps,"save":{"kind":"current"}}),
            "cli_script_save",
        ),
        (
            json!({"version":1,"document":{"kind":"open","path":"missing"},"steps":steps,"unknown":true}),
            "invalid_params",
        ),
        (
            script(json!([{"id":"text.bold","extra":true}]), Value::Null),
            "invalid_params",
        ),
        (
            script(
                steps.clone(),
                json!({"kind":"as","path":"copy.typsto","overwrite":true}),
            ),
            "invalid_params",
        ),
    ] {
        let path = job(dir.path(), payload);
        assert_eq!(
            response(cli().arg("script").arg(&path), 1)["error"]["params"]["reason"],
            reason
        );
    }
    let path = dir.path().join("job.json");
    std::fs::write(&path, b"{broken").unwrap();
    assert_eq!(
        response(cli().arg("script").arg(&path), 1)["error"]["params"]["reason"],
        "invalid_params"
    );
    std::fs::write(&path, vec![b' '; MAX_BATCH_BYTES + 1]).unwrap();
    assert_eq!(
        response(cli().arg("script").arg(&path), 1)["error"]["params"]["reason"],
        "cli_script_too_large"
    );
    assert_eq!(
        response(cli().arg("script").arg(dir.path().join("missing.json")), 3)["error"]["params"]["reason"],
        "cli_script_io"
    );
}
