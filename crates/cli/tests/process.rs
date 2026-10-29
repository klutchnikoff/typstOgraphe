#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod support;
use serde_json::json;
use support::*;
use typstographe_core::content::api;
#[test]
fn discovery_and_generated_docs_match_the_common_catalogue() {
    assert_eq!(
        response(cli().args(["commands", "list"]), 0)["outcome"],
        api::catalogue()
    );
    assert_eq!(
        response(cli().args(["commands", "describe", "metadata.set"]), 0)["outcome"],
        api::describe("metadata.set").unwrap()
    );
    assert_eq!(
        response(cli().args(["commands", "describe", "missing"]), 1)["error"]["code"],
        "not_found"
    );
    assert_eq!(
        typstographe_cli::documentation(),
        include_str!("../../../docs/cli.md").replace("\r\n", "\n")
    );
}
#[test]
fn inspect_is_a_saved_snapshot_even_under_an_active_writer() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("Été 日本.typsto");
    let a = fixture(&p, "Bonjour");
    let mut w = writer(&p);
    w.execute(
        "text.insert",
        json!({"target":target(0,0),"text":"Unsaved "}),
    )
    .unwrap();
    let before = std::fs::read(&p).unwrap();
    let r = response(cli().arg("inspect").arg(&p), 0);
    assert_eq!(r["outcome"]["document"]["read_only"], true);
    assert_eq!(r["outcome"]["metadata"]["metadata"], json!(a.metadata));
    assert_eq!(r["outcome"]["structure"]["zones"][0]["text"], "Bonjour");
    assert_eq!(std::fs::read(&p).unwrap(), before);
}
#[test]
fn drafts_export_readiness_and_local_spelling_are_separate() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("draft.typsto");
    response(
        cli()
            .args(["new", "--type", "courrier", "--output"])
            .arg(&p),
        0,
    );
    assert_eq!(
        response(cli().arg("check").arg(&p), 0)["outcome"]["validation"]["ready_for_export"],
        false
    );
    assert_eq!(
        response(cli().arg("check").arg(&p).arg("--for-export"), 1)["outcome"]["passed"],
        false
    );
    let p = d.path().join("spelling.typsto");
    fixture(&p, "Xyzzzzzzzz.");
    let r = response(cli().arg("check").arg(&p).arg("--spelling"), 1);
    assert!(
        !r["outcome"]["spelling"]["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn export_is_offline_complete_and_cannot_overwrite() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("letter.typsto");
    let a = fixture(&p, "Bonjour.");
    let before = std::fs::read(&p).unwrap();
    let out = d.path().join("export");
    response(
        cli()
            .arg("export")
            .arg(&p)
            .args(["--format", "both", "--output"])
            .arg(&out),
        0,
    );
    assert!(
        std::fs::read(out.join("courrier.pdf"))
            .unwrap()
            .starts_with(b"%PDF")
    );
    assert_eq!(
        std::fs::read_to_string(out.join("contenu.typ")).unwrap(),
        a.validate().unwrap()
    );
    for dep in a.dependencies.keys() {
        assert!(out.join(dep).exists());
    }
    response(cli().arg("export").arg(&p).arg("--output").arg(&out), 3);
    assert_eq!(std::fs::read(&p).unwrap(), before);
}
#[test]
fn errors_are_clean_json_and_localized_without_changing_machine_codes() {
    assert_eq!(
        response(cli().args(["export", "--unknown"]), 2)["error"]["code"],
        "usage"
    );
    assert_eq!(
        response(cli().args(["inspect", "absent.typsto"]), 3)["ok"],
        false
    );
    let fr = response(
        cli().args(["--lang=fr", "commands", "describe", "missing"]),
        1,
    );
    let en = response(
        cli().args(["--lang=en", "commands", "describe", "missing"]),
        1,
    );
    assert_eq!(fr["error"]["code"], en["error"]["code"]);
    assert_ne!(fr["error"]["message"], en["error"]["message"]);
    assert!(response(cli().arg("--help"), 0)["outcome"]["help"].is_string());
}
