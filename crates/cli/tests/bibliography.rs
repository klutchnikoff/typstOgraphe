#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod support;
use serde_json::json;
use support::*;
use typstographe_core::content::api::Session;

#[test]
fn cli_can_prepare_notices_under_an_active_writer_without_touching_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("article.typsto");
    let mut owner = Session::new();
    owner
        .execute("document.new", json!({"type":"article"}))
        .unwrap();
    owner.save_as(&path).unwrap();
    let original = std::fs::read(&path).unwrap();
    let cases = [
        ("bibliography.profiles", json!({})),
        (
            "bibliography.prepare_record",
            json!({"id":"new-record","profile":"book","entry":{"title":"Un ouvrage","author":"Durand, Élise"}}),
        ),
        (
            "bibliography.prepare_import",
            json!({"id":"source","name":"in.yaml","format":"hayagriva","contents":"k: {type: book, title: An example}"}),
        ),
    ];
    for (operation, params) in cases {
        let actual = response(
            cli()
                .args(["query", operation])
                .arg(&path)
                .args(["--params", &params.to_string()]),
            0,
        );
        let expected = owner.query(operation, params).unwrap();
        assert_eq!(actual["outcome"], expected);
        if operation != "bibliography.profiles" {
            assert_eq!(actual["outcome"]["active"], false);
        }
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    assert!(!owner.dirty());
}

#[test]
fn cli_reports_invalid_native_field_data_without_publishing_or_losing_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("article.typsto");
    let mut owner = Session::new();
    owner
        .execute("document.new", json!({"type":"article"}))
        .unwrap();
    owner.save_as(&path).unwrap();
    let original = std::fs::read(&path).unwrap();
    let result = response(
        cli()
            .args(["query", "bibliography.prepare_record"])
            .arg(&path)
            .args([
                "--params",
                r#"{"id":"new","profile":"book","entry":{"unknown":"lost"}}"#,
            ]),
        1,
    );
    let text = serde_json::to_string(&result).unwrap();
    assert!(text.contains("unsupported_field"));
    assert!(text.contains("/unknown"));
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn cli_renames_a_cited_notice_under_guards_and_reopens_the_same_record_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("article.typsto");
    let mut owner = Session::new();
    owner
        .execute("document.new", json!({"type":"article"}))
        .unwrap();
    let prepared = owner
        .query(
            "bibliography.prepare_record",
            json!({"id":"book","profile":"book","key":"before","entry":{"title":"An example"}}),
        )
        .unwrap();
    let library = owner.query("bibliography.inspect", json!({})).unwrap();
    owner.batch(&[
        typstographe_core::content::api::Call{id:"bibliography.apply".into(),params:json!({"expected":library["fingerprint"],"change":{"kind":"create","record":prepared["record"]}})},
        typstographe_core::content::api::Call{id:"inline.replace".into(),params:json!({"target":target(0,0),"node":{"type":"citation","attrs":{"items":[{"record_id":"book"}],"form":"normal"}}})},
    ],None).unwrap();
    owner.save_as(&path).unwrap();
    let before = owner.archive().unwrap();
    let library = owner.query("bibliography.inspect", json!({})).unwrap();
    let record = owner
        .query("bibliography.record", json!({"id":"book"}))
        .unwrap();
    drop(owner);
    let params = json!({"expected":library["fingerprint"],"change":{"kind":"rename","id":"book","expected":record["fingerprint"],"key":"after"}});
    response(
        cli().args(["run", "bibliography.apply"]).arg(&path).args([
            "--params",
            &params.to_string(),
            "--save",
        ]),
        0,
    );
    let mut reopened = writer(&path);
    let after = reopened.archive().unwrap();
    assert_eq!(after.document, before.document);
    assert!(after.validate().unwrap().contains("label(\"after\")"));
    assert_eq!(
        reopened
            .query("bibliography.record", json!({"id":"book"}))
            .unwrap()["record"]["key"],
        "after"
    );
    let bytes = std::fs::read(&path).unwrap();
    drop(reopened);
    response(
        cli().args(["run", "bibliography.apply"]).arg(&path).args([
            "--params",
            &params.to_string(),
            "--save",
        ]),
        1,
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
