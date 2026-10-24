#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
use assert_cmd::{Command as ProcessCommand, cargo::cargo_bin_cmd};
use serde_json::{Value, json};
use std::path::Path;
use typstographe_core::{
    document::Document,
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
    let assert = command.arg("--json").assert().code(exit);
    assert!(
        assert.get_output().stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&assert.get_output().stderr)
    );
    serde_json::from_slice(&assert.get_output().stdout).unwrap()
}
fn fixture(path: &Path, source: &str) -> StoredDocument {
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.source = source.into();
    storage::save(path, &stored).unwrap();
    stored
}
fn writer(path: &Path) -> Engine {
    let mut engine = Engine::empty();
    engine
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: engine.target_guard(),
            path: path.into(),
            read_only: false,
            discard_changes: false,
        }))
        .unwrap();
    engine
}

#[test]
fn global_queries_use_the_catalogue_and_document_queries_require_a_file() {
    let expected = Engine::empty()
        .query(Query::DocumentTypes(Empty {}))
        .unwrap();
    assert_eq!(
        response(cli().args(["query", "document_types.list"]), 0)["outcome"],
        expected
    );
    for id in [
        "metadata.get",
        "document.structure",
        "models.list",
        "spelling.issues",
        "document.source",
    ] {
        assert_eq!(
            response(cli().args(["query", id]), 1)["error"]["params"]["reason"],
            "no_document"
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let stored = fixture(&path, "Bonjour Élodie.");
    let result = response(cli().args(["query", "metadata.get"]).arg(&path), 0);
    assert_eq!(result["outcome"]["metadata"], json!(stored.metadata));
    let suggestions = response(
        cli().args([
            "query",
            "spelling.suggestions",
            "--params",
            r#"{"target":{"kind":"word","language":"fr","word":"bonjor"}}"#,
        ]),
        0,
    );
    assert_eq!(suggestions["ok"], true);
}

#[test]
fn run_metadata_save_matches_the_engine_and_reopens_as_an_autonomous_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let stored = fixture(&path, "Texte à garder.");
    let mut expected = Engine::from_stored(stored, None).unwrap();
    expected.run(Command::from_call("metadata.set", json!({"guard":expected.target_guard(),"field":"letter.subject","value":"Été 日本"})).unwrap()).unwrap();
    let result = response(
        cli().args(["run", "metadata.set"]).arg(&path).args([
            "--params",
            r#"{"field":"letter.subject","value":"Été 日本"}"#,
            "--save",
        ]),
        0,
    );
    assert_eq!(result["outcome"]["saved"], true);
    let reopened = writer(&path);
    assert_eq!(reopened.stored_document(), expected.stored_document());
    assert!(!reopened.is_dirty());
}

#[test]
fn preview_selects_one_unicode_occurrence_and_does_not_modify_the_source_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path, "Bonjour Élodie. Bonjour Élodie. 日本\n");
    let before = std::fs::read(&path).unwrap();
    let result = response(
        cli().args(["run", "text.bold"]).arg(&path).args([
            "--select",
            "Élodie",
            "--occurrence",
            "2",
        ]),
        0,
    );
    assert_eq!(result["outcome"]["saved"], false);
    assert!(result["outcome"]["path"].is_null());
    let source = result["outcome"]["source"].as_str().unwrap();
    let document = Document::from_source(source).unwrap();
    let first = document.view.text.find("Élodie").unwrap();
    let first = document.view.text[..first].chars().count();
    let second_byte = document.view.text.rfind("Élodie").unwrap();
    let second = document.view.text[..second_byte].chars().count();
    assert!(!document.view.styles[first].bold);
    assert!(
        document.view.styles[second..second + 6]
            .iter()
            .all(|s| s.bold)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let human = cli()
        .args(["--lang", "en", "run", "text.italic"])
        .arg(&path)
        .args(["--select", "日本"])
        .assert()
        .success();
    assert!(
        String::from_utf8_lossy(&human.get_output().stdout).contains("no document file was saved")
    );
    let text = response(
        cli()
            .args(["query", "document.text"])
            .arg(&path)
            .args(["--select", "日本"]),
        0,
    );
    assert_eq!(text["outcome"]["text"], "日本");
    assert!(!dir.path().join(".letter.typsto.lock").exists());
}

#[test]
fn copy_and_preview_work_while_a_writer_is_open_but_save_is_refused_and_targets_are_not_overwritten()
 {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path, "Bonjour.");
    let owner = writer(&path);
    let before = std::fs::read(&path).unwrap();
    let copy = dir.path().join("copie.typsto");
    response(
        cli()
            .args(["run", "text.insert"])
            .arg(&path)
            .args(["--params", r#"{"text":"Été "}"#, "--output"])
            .arg(&copy),
        0,
    );
    assert!(storage::read(&copy).unwrap().source.contains("Été"));
    let bytes = std::fs::read(&copy).unwrap();
    let error = response(
        cli().args(["run", "text.insert"]).arg(&path).args([
            "--params",
            r#"{"text":"Non "}"#,
            "--save",
        ]),
        1,
    );
    assert_eq!(error["error"]["params"]["reason"], "file_locked");
    response(
        cli()
            .args(["run", "text.insert"])
            .arg(&path)
            .args(["--params", r#"{"text":"Non "}"#, "--output"])
            .arg(&copy),
        3,
    );
    response(
        cli()
            .args(["run", "text.insert"])
            .arg(&path)
            .args(["--params", r#"{"text":"Non "}"#, "--output"])
            .arg(&path),
        3,
    );
    assert_eq!(std::fs::read(&copy).unwrap(), bytes);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(owner.document().source(), "Bonjour.");
}

#[test]
fn file_operations_history_recovery_and_wrong_families_are_rejected_before_opening_a_document() {
    for id in [
        "document.save",
        "document.save_as",
        "document.export",
        "document.new",
        "document.open",
        "document.batch",
        "edit.undo",
        "recovery.delete",
    ] {
        let result = response(cli().args(["run", id, "missing.typsto"]), 1);
        assert_eq!(result["error"]["params"]["reason"], "cli_call_unavailable");
    }
    assert_eq!(
        response(cli().args(["query", "recovery.list", "missing.typsto"]), 1)["error"]["params"]["reason"],
        "cli_call_unavailable"
    );
    for (verb, id) in [("run", "metadata.get"), ("query", "text.bold")] {
        assert_eq!(
            response(cli().args([verb, id, "missing.typsto"]), 1)["error"]["params"]["reason"],
            "cli_call_family"
        );
    }
    assert_eq!(
        response(cli().args(["run", "unknown.action", "missing.typsto"]), 1)["error"]["params"]["reason"],
        "unknown_command"
    );
}

#[test]
fn selector_errors_and_foreign_references_leave_the_archive_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path, "Été Été.");
    let before = std::fs::read(&path).unwrap();
    for (options, reason) in [
        (vec!["--select", "Été"], "cli_selection_ambiguous"),
        (vec!["--select", "été"], "cli_selection_missing"),
        (
            vec!["--select", "Été", "--occurrence", "9"],
            "cli_selection_missing",
        ),
        (vec!["--block", "99"], "cli_block_missing"),
        (
            vec![
                "--select",
                "Été",
                "--occurrence",
                "1",
                "--params",
                r#"{"target":{"kind":"all"}}"#,
            ],
            "cli_params_conflict",
        ),
        (
            vec!["--params", r#"{"guard":{"document_id":1,"revision":1}}"#],
            "cli_session_reference",
        ),
    ] {
        let result = response(
            cli()
                .args(["run", "text.bold"])
                .arg(&path)
                .args(options)
                .arg("--save"),
            1,
        );
        assert_eq!(result["error"]["params"]["reason"], reason);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    response(
        cli()
            .args(["run", "text.bold"])
            .arg(&path)
            .args(["--block", "0"]),
        2,
    );
    response(
        cli()
            .args(["run", "text.bold"])
            .arg(&path)
            .args(["--occurrence", "1"]),
        2,
    );
    response(
        cli()
            .args(["run", "text.bold"])
            .arg(&path)
            .args(["--save", "--output"])
            .arg(dir.path().join("copy.typsto")),
        2,
    );
}

#[test]
fn block_targets_modify_image_metadata_and_json_asset_paths_use_the_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    let image = include_bytes!("../../../fixtures/signature.svg");
    std::fs::write(dir.path().join("signature.svg"), image).unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path, "Introduction.\n\n");
    let params_dir = dir.path().join("params");
    std::fs::create_dir(&params_dir).unwrap();
    std::fs::write(
        params_dir.join("image.json"),
        r#"{"image":{"kind":"path","path":"signature.svg"},"caption":"Légende 日本"}"#,
    )
    .unwrap();
    response(
        cli().current_dir(dir.path()).args([
            "run",
            "insert.image",
            "letter.typsto",
            "--params-file",
            "params/image.json",
            "--save",
        ]),
        0,
    );
    let structure = response(cli().args(["query", "document.structure"]).arg(&path), 0);
    let number = structure["outcome"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .position(|b| b["kind"] == "image")
        .unwrap()
        + 1;
    response(
        cli().args(["run", "image.set_alt"]).arg(&path).args([
            "--block",
            &number.to_string(),
            "--params",
            r#"{"alt":"Image de test"}"#,
            "--save",
        ]),
        0,
    );
    let stored = storage::read(&path).unwrap();
    assert!(stored.source.contains("Image de test"));
    assert!(stored.source.contains("Légende 日本"));
    assert_eq!(stored.assets.len(), 1);
}

#[test]
fn spelling_and_replacement_targets_are_built_in_the_current_session() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path, "Bonjour xyzzzzzzzz.");
    response(
        cli().args(["run", "spelling.correct"]).arg(&path).args([
            "--select",
            "xyzzzzzzzz",
            "--params",
            r#"{"replacement":"collègues"}"#,
            "--save",
        ]),
        0,
    );
    response(
        cli()
            .args(["run", "edit.replace_current"])
            .arg(&path)
            .args([
                "--select",
                "Bonjour",
                "--params",
                r#"{"query":"Bonjour","replacement":"Bonsoir"}"#,
                "--save",
            ]),
        0,
    );
    assert!(
        storage::read(&path)
            .unwrap()
            .source
            .contains("Bonsoir collègues.")
    );
}

#[test]
fn render_queries_compile_and_bind_a_fresh_reference_without_filesystem_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path, "Bonjour Élodie.");
    let before = std::fs::read(&path).unwrap();
    let boxes = response(
        cli()
            .args(["query", "render.selection_boxes"])
            .arg(&path)
            .args(["--select", "Élodie"]),
        0,
    );
    assert!(!boxes["outcome"]["boxes"].as_array().unwrap().is_empty());
    let locate = response(
        cli().args(["query", "render.locate"]).arg(&path).args([
            "--select",
            "Élodie",
            "--params",
            r#"{"position":{"direction":"to_preview"}}"#,
        ]),
        0,
    );
    assert!(!locate["outcome"]["point"].is_null());
    let compiled = response(cli().args(["query", "render.compile"]).arg(&path), 0);
    assert!(compiled["outcome"]["render"]["pages"].is_array());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
