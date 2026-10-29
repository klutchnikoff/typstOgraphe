#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod support;
use serde_json::json;
use support::*;
#[test]
fn queries_share_the_contract_and_require_a_document_when_appropriate() {
    assert_eq!(
        response(cli().args(["query", "document_types.list"]), 0)["outcome"],
        typstographe_core::content::api::types()
    );
    response(cli().args(["query", "metadata.get"]), 1);
    assert_eq!(
        response(
            cli().args([
                "query",
                "spelling.suggestions",
                "--params",
                r#"{"word":"bonjor","language":"fr"}"#
            ]),
            0
        )["ok"],
        true
    );
}
#[test]
fn typed_metadata_and_unicode_range_edits_save_and_reopen() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("letter.typsto");
    fixture(&p, "Bonjour Élodie. Bonjour Élodie. 日本");
    response(
        cli().args(["run", "metadata.set"]).arg(&p).args([
            "--params",
            r#"{"target":{"kind":"metadata","field":"letter.subject"},"value":"Été 日本"}"#,
            "--save",
        ]),
        0,
    );
    response(
        cli().args(["run", "text.bold"]).arg(&p).args([
            "--select",
            "Élodie",
            "--occurrence",
            "2",
            "--save",
        ]),
        0,
    );
    assert!(has_mark(&p, "Élodie", "bold"));
    assert!(!writer(&p).dirty());
    assert_eq!(
        response(
            cli().args(["query", "document.text"]).arg(&p).args([
                "--select",
                "日本",
                "--occurrence",
                "1"
            ]),
            0
        )["outcome"]["text"],
        "日本"
    );
}
#[test]
fn preview_copy_and_locks_preserve_the_active_writer() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("letter.typsto");
    fixture(&p, "Élodie Élodie");
    let _w = writer(&p);
    let before = std::fs::read(&p).unwrap();
    let r = response(
        cli().args(["run", "text.italic"]).arg(&p).args([
            "--select",
            "Élodie",
            "--occurrence",
            "1",
        ]),
        0,
    );
    assert_eq!(r["outcome"]["saved"], false);
    let copy = d.path().join("copy.typsto");
    response(
        cli()
            .args(["run", "text.italic"])
            .arg(&p)
            .args(["--select", "Élodie", "--occurrence", "1", "--output"])
            .arg(&copy),
        0,
    );
    response(
        cli().args(["run", "text.italic"]).arg(&p).args([
            "--select",
            "Élodie",
            "--occurrence",
            "1",
            "--save",
        ]),
        1,
    );
    response(
        cli()
            .args(["run", "text.italic"])
            .arg(&p)
            .args(["--select", "Élodie", "--occurrence", "1", "--output"])
            .arg(&copy),
        1,
    );
    assert_eq!(std::fs::read(&p).unwrap(), before);
}
#[test]
fn invalid_selectors_targets_families_and_foreign_references_leave_files_untouched() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("letter.typsto");
    fixture(&p, "Élodie Élodie");
    let before = std::fs::read(&p).unwrap();
    for args in [
        vec!["--select", "Élodie"],
        vec!["--select", "missing"],
        vec!["--params", r#"{"guard":{},"target":{}}"#],
        vec![
            "--params",
            r#"{"target":{"zone":{"kind":"body","path":[0]},"from":99,"to":100}}"#,
        ],
    ] {
        response(cli().args(["run", "text.bold"]).arg(&p).args(args), 1);
    }
    for id in [
        "document.save",
        "document.open",
        "edit.undo",
        "document.text",
        "unknown",
    ] {
        response(cli().args(["run", id]).arg(&p), 1);
    }
    assert_eq!(std::fs::read(&p).unwrap(), before);
}
#[test]
fn render_queries_use_typed_body_targets_and_do_not_write_files() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("letter.typsto");
    fixture(&p, "Bonjour Élodie");
    let q = response(cli().args(["query", "render.compile"]).arg(&p), 0);
    assert!(!q["outcome"]["pages"].as_array().unwrap().is_empty());
    let loc = response(
        cli()
            .args(["query", "render.locate"])
            .arg(&p)
            .arg("--params")
            .arg(json!({"target":target(0,7)}).to_string()),
        0,
    );
    assert!(loc["outcome"]["point"].is_object());
    assert!(!d.path().join("page.png").exists());
}
