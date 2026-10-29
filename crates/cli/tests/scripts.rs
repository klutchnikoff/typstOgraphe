#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod support;
use serde_json::{Value, json};
use support::*;
#[test]
fn a_structured_letter_with_notes_quote_and_enclosure_saves_one_atomic_lot() {
    let d = tempfile::tempdir().unwrap();
    let steps = json!([
        insert("Bonjour Élodie."),
        {"id":"text.bold","target":{"select":"Élodie"}},
        {"id":"inline.replace","params":{"target":target(15,15),"node":{"type":"footnote","attrs":{"text":"Précision"}}}},
        {"id":"block.insert","params":{"parent":{"root":{"kind":"body"}},"index":1,"node":{"type":"attachment","content":[{"type":"text","text":"Formulaire signé"}]}}}
    ]);
    let j = job(
        d.path(),
        json!({"version":1,"document":{"kind":"new","type":"courrier","params":{"metadata":typstographe_core::document_types::Metadata::demo(typstographe_core::document_types::DocumentType::Courrier)}},"steps":steps,"save":{"kind":"as","path":"letter.typsto"}}),
    );
    let r = response(cli().arg("script").arg(&j), 0);
    assert_eq!(r["outcome"]["document"]["context"]["undo_steps"], 1);
    let src = source(&d.path().join("letter.typsto"));
    assert!(has_mark(&d.path().join("letter.typsto"), "Élodie", "bold"));
    assert!(src.contains("Précision"));
    assert!(src.contains("Formulaire signé"));
    response(
        cli()
            .arg("check")
            .arg(d.path().join("letter.typsto"))
            .arg("--for-export"),
        0,
    );
}
#[test]
fn selectors_resolve_after_previous_steps_and_failure_at_any_stage_preserves_disk() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("original.typsto");
    fixture(&p, "Avant");
    let before = std::fs::read(&p).unwrap();
    let j = job(
        d.path(),
        script(
            json!([insert("Élodie "),{"id":"text.bold","target":{"select":"Élodie"}}]),
            json!({"kind":"current"}),
        ),
    );
    response(cli().arg("script").arg(&j), 0);
    assert!(has_mark(&p, "Élodie", "bold"));
    let updated = std::fs::read(&p).unwrap();
    assert_ne!(updated, before);
    let j = job(
        d.path(),
        script(
            json!([insert("Private"),{"id":"text.delete","params":{"target":{"zone":{"kind":"body","path":[99]},"from":0,"to":1}}}]),
            json!({"kind":"current"}),
        ),
    );
    let r = response(cli().arg("script").arg(&j), 1);
    assert_eq!(r["error"]["code"], "batch_failed");
    assert_eq!(r["error"]["index"], 1);
    assert_eq!(std::fs::read(&p).unwrap(), updated);
}
#[test]
fn preview_copy_and_relative_save_work_under_an_active_writer_but_current_save_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("original.typsto");
    fixture(&p, "Base");
    let _w = writer(&p);
    let before = std::fs::read(&p).unwrap();
    for save in [Value::Null, json!({"kind":"as","path":"copy.typsto"})] {
        let j = job(d.path(), script(json!([insert("Aperçu ")]), save));
        response(
            cli().current_dir(std::env::temp_dir()).arg("script").arg(j),
            0,
        );
    }
    assert!(source(&d.path().join("copy.typsto")).contains("Aperçu"));
    let j = job(
        d.path(),
        script(json!([insert("Refusé")]), json!({"kind":"current"})),
    );
    response(cli().arg("script").arg(j), 1);
    assert_eq!(std::fs::read(&p).unwrap(), before);
}
#[test]
fn failed_final_save_never_overwrites_or_creates_parents() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("original.typsto");
    fixture(&p, "Base");
    let before = std::fs::read(&p).unwrap();
    for path in ["original.typsto", "absent/new.typsto"] {
        let j = job(
            d.path(),
            script(json!([insert("Private")]), json!({"kind":"as","path":path})),
        );
        response(
            cli().arg("script").arg(j),
            if path.starts_with("absent") { 3 } else { 1 },
        );
    }
    assert_eq!(std::fs::read(&p).unwrap(), before);
    assert!(!d.path().join("absent").exists());
}
#[test]
fn script_envelopes_are_strict_bounded_and_do_not_allow_session_operations() {
    let d = tempfile::tempdir().unwrap();
    fixture(&d.path().join("original.typsto"), "Base");
    for id in [
        "document.save",
        "document.new",
        "document.batch",
        "context.insert",
        "edit.undo",
        "metadata.get",
        "missing",
    ] {
        let j = job(d.path(), script(json!([{"id":id}]), Value::Null));
        response(cli().arg("script").arg(j), 1);
    }
    for v in [
        json!({"version":2,"document":{"kind":"open","path":"missing"},"steps":[insert("X")]}),
        script(json!([]), Value::Null),
        script(
            json!((0..257).map(|_| insert("x")).collect::<Vec<_>>()),
            Value::Null,
        ),
        json!({"version":1,"document":{"kind":"new","type":"courrier","params":{"type":"article"}},"steps":[insert("x")]}),
        json!({"version":1,"document":{"kind":"new","type":"courrier"},"steps":[insert("x")],"extra":true}),
    ] {
        let j = job(d.path(), v);
        response(cli().arg("script").arg(j), 1);
    }
    let j = d.path().join("broken.json");
    std::fs::write(&j, "{bad").unwrap();
    response(cli().arg("script").arg(&j), 1);
    std::fs::write(&j, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    response(cli().arg("script").arg(&j), 1);
    response(cli().arg("script").arg(d.path().join("missing.json")), 3);
}
#[test]
fn stdin_is_relative_to_cwd_and_uses_the_same_documentary_commands() {
    let d = tempfile::tempdir().unwrap();
    fixture(&d.path().join("original.typsto"), "Base");
    let v = script(
        json!([insert("Depuis stdin ")]),
        json!({"kind":"as","path":"stdin.typsto"}),
    );
    response(
        cli()
            .current_dir(d.path())
            .args(["script", "-"])
            .write_stdin(v.to_string()),
        0,
    );
    assert!(source(&d.path().join("stdin.typsto")).contains("Depuis stdin"));
}
