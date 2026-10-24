#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
use assert_cmd::{Command as ProcessCommand, cargo::cargo_bin_cmd};
use serde_json::{Value, json};
use typstographe_core::{engine::*, storage};

fn cli() -> ProcessCommand {
    let mut command = cargo_bin_cmd!("typstographe-cli");
    command.timeout(std::time::Duration::from_secs(60));
    command
}
fn response(command: &mut ProcessCommand, exit: i32) -> Value {
    let assert = command.arg("--json").assert().code(exit);
    assert!(assert.get_output().stderr.is_empty());
    serde_json::from_slice(&assert.get_output().stdout).unwrap()
}

#[test]
fn creation_uses_the_type_catalogue_and_explicit_values_and_reopens_through_document_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Été 日本.typsto");
    let params = json!({"initial_values":[{"field":"sender.name","value":"Élodie 日本"},{"field":"letter.date","value":"2026-10-24"}]}).to_string();
    let created = response(
        cli()
            .args(["new", "--type", "courrier", "--params", &params, "--output"])
            .arg(&path),
        0,
    );
    assert_eq!(created["outcome"]["saved"], true);
    let mut engine = Engine::empty();
    // Exactement l’opération appelée par l’interface graphique.
    engine
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: engine.target_guard(),
            path: path.clone(),
            read_only: false,
            discard_changes: false,
        }))
        .unwrap();
    assert!(engine.document().source().is_empty());
    assert!(!engine.metadata().ready_for_export());
    let fields = engine.metadata().field_values();
    assert_eq!(
        fields
            .iter()
            .find(|f| f.field == typstographe_core::document_types::MetadataField::SenderName)
            .unwrap()
            .value,
        "Élodie 日本"
    );
    assert_eq!(engine.metadata().signature_path(), None);
    assert!(!engine.is_read_only() && !engine.is_dirty());
    let before = std::fs::read(&path).unwrap();
    response(
        cli()
            .args(["new", "--type", "courrier", "--output"])
            .arg(&path),
        3,
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(storage::read(&path).unwrap(), engine.stored_document());
}

#[test]
fn creation_accepts_parameter_files_and_stdin_but_rejects_invalid_or_conflicting_input() {
    let dir = tempfile::tempdir().unwrap();
    let params = dir.path().join("valeurs.json");
    std::fs::write(
        &params,
        r#"{"initial_values":[{"field":"sender.name","value":"Zoé"}]}"#,
    )
    .unwrap();
    let path = dir.path().join("file.typsto");
    response(
        cli()
            .args([
                "new",
                "--type",
                "courrier",
                "--model",
                "courrier.model.personal",
                "--params-file",
            ])
            .arg(&params)
            .arg("--output")
            .arg(&path),
        0,
    );
    let stdin = dir.path().join("stdin.typsto");
    response(
        cli()
            .args([
                "new",
                "--type",
                "courrier",
                "--params-file",
                "-",
                "--output",
            ])
            .arg(&stdin)
            .write_stdin(std::fs::read(&params).unwrap()),
        0,
    );
    assert_eq!(
        storage::read(&path).unwrap(),
        storage::read(&stdin).unwrap()
    );
    for (input, reason) in [
        ("[]", "cli_params_object"),
        ("{broken", "invalid_params"),
        (r#"{"model":"other"}"#, "cli_params_conflict"),
        (r#"{"unexpected":true}"#, "invalid_params"),
    ] {
        let out = dir.path().join("absent.typsto");
        let error = response(
            cli()
                .args(["new", "--type", "courrier", "--params", input, "--output"])
                .arg(&out),
            1,
        );
        assert_eq!(error["error"]["params"]["reason"], reason);
        assert!(!out.exists());
    }
    response(
        cli()
            .args([
                "new",
                "--type",
                "courrier",
                "--params",
                "{}",
                "--params-file",
                "-",
                "--output",
            ])
            .arg(dir.path().join("conflict.typsto")),
        2,
    );
}

#[test]
fn unknown_types_models_and_types_without_models_do_not_create_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.typsto");
    for (kind, model, reason) in [
        ("unknown", None, "cli_document_type"),
        ("compte-rendu", None, "creation_model"),
        ("courrier", Some("missing"), "creation_model"),
    ] {
        let mut command = cli();
        command.args(["new", "--type", kind, "--output"]).arg(&path);
        if let Some(model) = model {
            command.args(["--model", model]);
        }
        assert_eq!(
            response(&mut command, 1)["error"]["params"]["reason"],
            reason
        );
        assert!(!path.exists());
    }
}

#[test]
fn parameter_file_errors_and_oversized_input_are_reported_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.typsto");
    let file = dir.path().join("parameters.json");
    let error = response(
        cli()
            .args(["new", "--type", "courrier", "--params-file"])
            .arg(&file)
            .arg("--output")
            .arg(&path),
        3,
    );
    assert_eq!(error["error"]["params"]["reason"], "cli_params_io");
    std::fs::write(&file, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    let error = response(
        cli()
            .args(["new", "--type", "courrier", "--params-file"])
            .arg(&file)
            .arg("--output")
            .arg(&path),
        1,
    );
    assert_eq!(error["error"]["params"]["reason"], "cli_params_too_large");
    assert!(!path.exists());
}
