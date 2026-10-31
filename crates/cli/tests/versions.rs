#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod support;
use serde_json::json;
use support::*;
#[test]
fn cli_version_creation_and_reading_share_the_common_index_and_never_bypass_writer_locks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path, "Original text");
    let index = response(cli().args(["query", "versions.list"]).arg(&path), 0)["outcome"].clone();
    let params = json!({"history":index["history"],"action":{"kind":"create","details":{"name":"v1","tags":["draft"]}}});
    let created = response(
        cli()
            .arg("versions")
            .arg(&path)
            .arg("--params")
            .arg(params.to_string()),
        0,
    );
    let id = created["outcome"]["versions"][0]["id"].clone();
    assert!(id.is_string());
    let read = response(
        cli()
            .args(["query", "versions.read"])
            .arg(&path)
            .arg("--params")
            .arg(json!({"id":id}).to_string()),
        0,
    );
    assert_eq!(
        read["outcome"]["state"]["document"]["content"][0]["content"][0]["text"],
        "Original text"
    );
    let before = std::fs::read(&path).unwrap();
    response(
        cli()
            .arg("versions")
            .arg(&path)
            .arg("--params")
            .arg(params.to_string()),
        1,
    );
    assert_eq!(before, std::fs::read(&path).unwrap());
    let _writer = writer(&path);
    response(cli().args(["query", "versions.list"]).arg(&path), 0);
    cli()
        .arg("versions")
        .arg(&path)
        .arg("--params")
        .arg(params.to_string())
        .assert()
        .failure();
    assert_eq!(before, std::fs::read(&path).unwrap());
}
