#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod support;
use serde_json::json;
use support::*;
#[test]
fn new_uses_declared_types_models_and_explicit_metadata_without_profile_defaults() {
    let d = tempfile::tempdir().unwrap();
    for kind in ["courrier", "article"] {
        let p = d.path().join(format!("{kind}.typsto"));
        let r = response(cli().args(["new", "--type", kind, "--output"]).arg(&p), 0);
        assert_eq!(r["outcome"]["saved"], true);
        let a = typstographe_core::content::project::Project::read(&p)
            .unwrap()
            .current()
            .clone();
        assert_eq!(
            serde_json::to_value(a.metadata.document_type()).unwrap(),
            kind
        );
        assert!(!a.yjs.is_empty());
    }
}
#[test]
fn parameter_files_and_stdin_are_supported_and_conflicts_are_rejected() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("letter.typsto");
    let f = d.path().join("params.json");
    let data = json!({"metadata":typstographe_core::document_types::Metadata::demo(typstographe_core::document_types::DocumentType::Courrier)});
    std::fs::write(&f, data.to_string()).unwrap();
    response(
        cli()
            .args(["new", "--type", "courrier", "--params-file"])
            .arg(&f)
            .arg("--output")
            .arg(&p),
        0,
    );
    let copy = d.path().join("copy.typsto");
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
            .arg(&copy)
            .write_stdin(data.to_string()),
        0,
    );
    for params in [
        "[]",
        "{broken",
        r#"{"type":"article"}"#,
        r#"{"unknown":true}"#,
    ] {
        response(
            cli()
                .args(["new", "--type", "courrier", "--params", params, "--output"])
                .arg(d.path().join("absent.typsto")),
            1,
        );
    }
}
#[test]
fn unknown_unavailable_types_models_and_oversized_inputs_never_create_files() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("absent.typsto");
    for kind in ["unknown", "compte-rendu"] {
        response(cli().args(["new", "--type", kind, "--output"]).arg(&p), 1);
    }
    response(
        cli()
            .args([
                "new", "--type", "courrier", "--model", "missing", "--output",
            ])
            .arg(&p),
        1,
    );
    let f = d.path().join("params.json");
    response(
        cli()
            .args(["new", "--type", "courrier", "--params-file"])
            .arg(&f)
            .arg("--output")
            .arg(&p),
        3,
    );
    std::fs::write(&f, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    response(
        cli()
            .args(["new", "--type", "courrier", "--params-file"])
            .arg(&f)
            .arg("--output")
            .arg(&p),
        1,
    );
    assert!(!p.exists());
}
