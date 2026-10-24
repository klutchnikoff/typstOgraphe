#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
use assert_cmd::{Command as ProcessCommand, cargo::cargo_bin_cmd};
use serde_json::Value;
use std::{path::Path, time::Duration};
use typstographe_core::{
    document_types::DocumentType,
    engine::*,
    storage::{self, StoredDocument},
};

fn cli() -> ProcessCommand {
    let mut command = cargo_bin_cmd!("typstographe-cli");
    command.timeout(Duration::from_secs(60));
    command
}
fn json(command: &mut ProcessCommand, exit: i32) -> Value {
    let assertion = command.arg("--json").assert().code(exit);
    let output = assertion.get_output();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON object, no extra output");
    assert_eq!(value["version"], 1);
    value
}
fn fixture(path: &Path) -> StoredDocument {
    let stored = StoredDocument::demo(DocumentType::Courrier);
    storage::save(path, &stored).unwrap();
    stored
}

#[test]
fn discovery_uses_the_registry_and_describe_has_the_exact_input_schema() {
    let value = json(cli().args(["commands", "list"]), 0);
    assert_eq!(
        value["outcome"]["entries"],
        serde_json::to_value(catalogue()).unwrap()
    );
    let queries = json(cli().args(["commands", "list", "--query"]), 0);
    assert_eq!(
        queries["outcome"]["entries"].as_array().unwrap().len(),
        Query::query_specs().len()
    );
    let description = json(cli().args(["commands", "describe", "metadata.set"]), 0);
    assert_eq!(
        description["outcome"],
        serde_json::to_value(
            catalogue()
                .iter()
                .find(|s| s.id.as_str() == "metadata.set")
                .unwrap()
        )
        .unwrap()
    );
    let unknown = json(cli().args(["commands", "describe", "missing.action"]), 1);
    assert_eq!(unknown["error"]["code"], "not_found");
}

#[test]
fn inspect_reads_a_snapshot_while_a_writer_holds_the_document_and_matches_engine_queries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Été 日本.typsto");
    let stored = fixture(&path);
    let mut writer = Engine::empty();
    writer
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: writer.target_guard(),
            path: path.clone(),
            read_only: false,
            discard_changes: false,
        }))
        .unwrap();
    writer
        .run(Command::InsertText(InsertTextParams {
            target: TextTarget::default(),
            text: "Non enregistré ".into(),
            input_rules: InputRules::Literal,
        }))
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let value = json(cli().arg("inspect").arg(&path), 0);
    let data = &value["outcome"];
    assert_eq!(data["document"]["read_only"], true);
    assert_eq!(
        data["metadata"]["metadata"],
        serde_json::to_value(&stored.metadata).unwrap()
    );
    let engine = Engine::from_stored(stored, None).unwrap();
    let stats = engine
        .query(Query::Statistics(StatisticsParams::default()))
        .unwrap();
    assert_eq!(data["statistics"]["document"], stats["document"]);
    assert_eq!(
        data["structure"]["blocks"][0]["zones"][0]["text"],
        engine.query(Query::Structure(Empty {})).unwrap()["blocks"][0]["zones"][0]["text"]
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn draft_validity_is_separate_from_export_readiness_and_spelling() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("draft.typsto");
    storage::save(&path, &StoredDocument::blank(DocumentType::Courrier)).unwrap();
    let draft = json(cli().arg("check").arg(&path), 0);
    assert_eq!(draft["outcome"]["passed"], true);
    assert_eq!(draft["outcome"]["validation"]["ready_for_export"], false);
    assert!(draft["outcome"]["spelling"].is_null());
    let not_ready = json(cli().arg("check").arg(&path).arg("--for-export"), 1);
    assert_eq!(not_ready["ok"], true);
    assert_eq!(not_ready["outcome"]["passed"], false);
    let mut stored = fixture(&path);
    stored.source = "Xyzzzzzzzz.\n".into();
    storage::save(&path, &stored).unwrap();
    json(cli().arg("check").arg(&path), 0);
    let spelling = json(cli().arg("check").arg(&path).arg("--spelling"), 1);
    assert_eq!(spelling["outcome"]["spelling"]["available"], true);
    assert!(
        !spelling["outcome"]["spelling"]["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn export_uses_the_shared_engine_without_changing_the_archive_and_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let stored = fixture(&path);
    let before = std::fs::read(&path).unwrap();
    let export = dir.path().join("export");
    let value = json(
        cli()
            .arg("export")
            .arg(&path)
            .args(["--format", "both", "--output"])
            .arg(&export),
        0,
    );
    assert_eq!(value["ok"], true);
    assert!(
        std::fs::read(export.join("courrier.pdf"))
            .unwrap()
            .starts_with(b"%PDF")
    );
    assert_eq!(
        std::fs::read_to_string(export.join("contenu.typ")).unwrap(),
        stored.source
    );
    assert!(!export.join("page-1.png").exists());
    for dependency in stored.dependencies.keys() {
        assert!(export.join(dependency).exists());
    }
    let count = std::fs::read_dir(&export).unwrap().count();
    let failed = json(
        cli().arg("export").arg(&path).arg("--output").arg(&export),
        3,
    );
    assert_eq!(failed["ok"], false);
    assert_eq!(std::fs::read_dir(&export).unwrap().count(), count);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let pdf = dir.path().join("pdf");
    json(
        cli()
            .arg("export")
            .arg(&path)
            .arg("--pages")
            .arg("--output")
            .arg(&pdf),
        0,
    );
    assert!(pdf.join("page-1.png").is_file());
    assert!(!pdf.join("main.typ").exists());
    assert_eq!(
        Engine::from_stored(storage::read(&path).unwrap(), None)
            .unwrap()
            .stored_document(),
        stored
    );
}

#[test]
fn usage_io_and_document_failures_have_distinct_exit_codes_and_clean_json() {
    let usage = json(cli().args(["export", "--unknown"]), 2);
    assert_eq!(usage["error"]["code"], "usage");
    let missing = json(cli().args(["inspect", "does-not-exist.typsto"]), 3);
    assert_eq!(missing["error"]["code"], "io");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.typsto");
    storage::save(&path, &StoredDocument::blank(DocumentType::Courrier)).unwrap();
    let failed = json(
        cli()
            .arg("export")
            .arg(&path)
            .arg("--output")
            .arg(dir.path().join("not-created")),
        1,
    );
    assert_eq!(
        failed["error"]["params"]["reason"],
        "export_required_fields"
    );
    assert!(!dir.path().join("not-created").exists());
    std::fs::write(&path, "broken archive").unwrap();
    assert_eq!(json(cli().arg("inspect").arg(&path), 3)["ok"], false);
    let help = json(cli().arg("--help"), 0);
    assert!(help["outcome"]["help"].is_string());
}

#[test]
fn interface_language_changes_messages_but_not_the_machine_error_or_document_language() {
    let french = json(
        cli().args(["--lang=fr", "commands", "describe", "missing.action"]),
        1,
    );
    let english = json(
        cli().args(["commands", "describe", "missing.action", "--lang", "en"]),
        1,
    );
    assert_ne!(french["error"]["message"], english["error"]["message"]);
    assert_eq!(french["error"]["code"], english["error"]["code"]);
    assert_eq!(french["error"]["params"], english["error"]["params"]);
    for (language, phrase) in [("fr", "Inspecter"), ("en", "Inspect")] {
        let output = cli()
            .args(["--lang", language, "--help"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert!(String::from_utf8(output).unwrap().contains(phrase));
    }
}

#[test]
fn cli_documentation_is_generated_from_the_registry_and_checked_for_drift() {
    let reference = include_str!("../../../docs/cli.md");
    assert_eq!(
        typstographe_cli::documentation(),
        reference.replace("\r\n", "\n")
    );
    cli()
        .args(["commands", "list", "--markdown"])
        .assert()
        .success()
        .stdout(reference.replace("\r\n", "\n"));
}
