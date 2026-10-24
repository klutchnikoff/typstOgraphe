#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
use assert_cmd::{Command as ProcessCommand, cargo::cargo_bin_cmd};
use serde_json::Value;
use std::path::Path;
use typstographe_core::{
    assets,
    document_types::DocumentType,
    engine::*,
    storage::{self, StoredDocument},
};

fn cli() -> ProcessCommand {
    let mut cli = cargo_bin_cmd!("typstographe-cli");
    cli.timeout(std::time::Duration::from_secs(60));
    cli
}
fn response(command: &mut ProcessCommand, exit: i32) -> Value {
    let assertion = command.arg("--json").assert().code(exit);
    assert!(assertion.get_output().stderr.is_empty());
    serde_json::from_slice(&assertion.get_output().stdout).unwrap()
}
fn fixture(path: &Path) {
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.source = "Première page.".into();
    stored.template = "#let courrier(..args, body) = { set page(width: 120pt, height: 80pt, margin: 5pt); body; pagebreak(); [Seconde page.] }".into();
    storage::save(path, &stored).unwrap();
}

#[test]
fn a_selected_page_matches_the_engine_and_export_while_a_writer_is_active() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lettre.typsto");
    fixture(&input);
    let before = std::fs::read(&input).unwrap();
    let mut engine = Engine::empty();
    engine
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: engine.target_guard(),
            path: input.clone(),
            read_only: false,
            discard_changes: false,
        }))
        .unwrap();
    let compiled = engine
        .compile(GuardParams {
            guard: engine.target_guard(),
        })
        .unwrap()
        .render
        .unwrap();
    let expected = engine.render_page_png(compiled.reference, 1).unwrap();
    // Le lecteur ne voit pas la modification non enregistrée de l’écrivain.
    engine
        .run(Command::InsertText(InsertTextParams {
            target: TextTarget::default(),
            text: "Brouillon privé ".into(),
            input_rules: InputRules::Literal,
        }))
        .unwrap();
    let saved = storage::read(&input).unwrap();
    let output = dir.path().join("résultat 日本.png");
    let result = response(
        cli()
            .arg("render")
            .arg(&input)
            .args(["--page", "2", "--png"])
            .arg(&output),
        0,
    );
    assert_eq!(result["outcome"]["page"], 2);
    assert_eq!(result["outcome"]["pages"], 2);
    assert_eq!(
        result["outcome"]["size_points"],
        serde_json::json!([120.0, 80.0])
    );
    assert_eq!(std::fs::read(&output).unwrap(), expected);
    let image = assets::decode(&expected, "png").unwrap();
    assert_eq!((image.width(), image.height()), (240.0, 160.0));
    assert_eq!(std::fs::read(&input).unwrap(), before);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3); // archive, verrou actif, PNG
    let exported = dir.path().join("export");
    response(
        cli()
            .arg("export")
            .arg(&input)
            .args(["--format", "pdf", "--pages", "--output"])
            .arg(&exported),
        0,
    );
    assert_eq!(
        std::fs::read(exported.join("page-2.png")).unwrap(),
        expected
    );
    assert_eq!(storage::read(&input).unwrap(), saved);
}

#[test]
fn invalid_pages_and_required_arguments_never_create_a_png() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lettre.typsto");
    fixture(&input);
    let output = dir.path().join("page.png");
    for page in ["0", "-1", "4294967296", "abc"] {
        assert_eq!(
            response(
                cli()
                    .arg("render")
                    .arg(&input)
                    .args(["--page", page, "--png"])
                    .arg(&output),
                2
            )["error"]["code"],
            "usage"
        );
        assert!(!output.exists());
    }
    for page in ["3", "4294967295"] {
        let result = response(
            cli()
                .arg("render")
                .arg(&input)
                .args(["--page", page, "--png"])
                .arg(&output),
            1,
        );
        assert_eq!(result["error"]["params"]["reason"], "cli_render_page");
        assert_eq!(result["error"]["params"]["pages"], 2);
        assert!(!output.exists());
    }
    response(cli().arg("render").arg(&input).arg("--png").arg(&output), 2);
    response(cli().arg("render").arg(&input).args(["--page", "1"]), 2);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1); // pas même un verrou de lecture
}

#[test]
fn failed_render_or_publication_preserves_destinations_and_leaves_no_partial_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lettre.typsto");
    fixture(&input);
    let before = std::fs::read(&input).unwrap();
    let output = dir.path().join("existing.png");
    std::fs::write(&output, b"sentinelle").unwrap();
    for target in [&input, &output, dir.path()] {
        assert_eq!(
            response(
                cli()
                    .arg("render")
                    .arg(&input)
                    .args(["--page", "1", "--png"])
                    .arg(target),
                1
            )["error"]["params"]["reason"],
            "file_exists"
        );
    }
    assert_eq!(std::fs::read(&input).unwrap(), before);
    assert_eq!(std::fs::read(&output).unwrap(), b"sentinelle");
    let missing = dir.path().join("absent/page.png");
    assert_eq!(
        response(
            cli()
                .arg("render")
                .arg(&input)
                .args(["--page", "1", "--png"])
                .arg(&missing),
            3
        )["error"]["params"]["reason"],
        "cli_render_io"
    );
    assert!(!missing.parent().unwrap().exists());
    let mut stored = storage::read(&input).unwrap();
    stored.template = "#let courrier(..args, body) = panic(\"Compilation refusée\")".into();
    storage::save(&input, &stored).unwrap();
    let output = dir.path().join("failed.png");
    let error = response(
        cli()
            .arg("render")
            .arg(&input)
            .args(["--page", "1", "--png"])
            .arg(&output),
        1,
    );
    assert_eq!(error["error"]["params"]["reason"], "render_compile");
    assert!(!output.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn relative_output_and_human_messages_follow_the_requested_language() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lettre.typsto");
    fixture(&input);
    for (lang, text) in [("fr", "Page 1 rendue"), ("en", "Page 1 rendered")] {
        let result = cli()
            .current_dir(dir.path())
            .arg("render")
            .arg(&input)
            .args([
                "--page",
                "1",
                "--png",
                &format!("{lang}.png"),
                "--lang",
                lang,
            ])
            .assert()
            .success();
        assert!(
            String::from_utf8_lossy(&result.get_output().stdout)
                .replace(['\u{2068}', '\u{2069}'], "")
                .contains(text)
        );
        assert!(dir.path().join(format!("{lang}.png")).exists());
    }
}

#[cfg(unix)]
#[test]
fn png_publication_refuses_existing_and_dangling_symbolic_links() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lettre.typsto");
    fixture(&input);
    let target = dir.path().join("absent.png");
    let link = dir.path().join("lien.png");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert_eq!(
        response(
            cli()
                .arg("render")
                .arg(&input)
                .args(["--page", "1", "--png"])
                .arg(&link),
            1
        )["error"]["params"]["reason"],
        "file_exists"
    );
    assert!(!target.exists());
    std::fs::write(&target, b"sentinelle").unwrap();
    response(
        cli()
            .arg("render")
            .arg(&input)
            .args(["--page", "1", "--png"])
            .arg(&link),
        1,
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"sentinelle");
}
