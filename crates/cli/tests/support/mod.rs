#![allow(dead_code)]
use assert_cmd::{Command, cargo::cargo_bin_cmd};
use serde_json::{Value, json};
use std::path::Path;
use typstographe_core::{
    content::{api::Session, archive::Archive},
    document_types::{DocumentType, Metadata},
};
pub fn cli() -> Command {
    let mut c = cargo_bin_cmd!("typstographe-cli");
    c.timeout(std::time::Duration::from_secs(90));
    c
}
pub fn response(c: &mut Command, code: i32) -> Value {
    let a = c.arg("--json").assert().code(code);
    let o = a.get_output();
    assert!(
        o.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let v: Value = serde_json::from_slice(&o.stdout).expect("one JSON result");
    assert_eq!(v["version"], 1);
    v
}
pub fn target(from: u32, to: u32) -> Value {
    json!({"zone":{"kind":"body","path":[0]},"from":from,"to":to})
}
pub fn insert(text: &str) -> Value {
    json!({"id":"text.insert","params":{"target":target(0,0),"text":text}})
}
pub fn fixture(path: &Path, text: &str) -> Archive {
    let mut s = Session::new();
    s.execute(
        "document.new",
        json!({"type":"courrier","metadata":Metadata::demo(DocumentType::Courrier)}),
    )
    .unwrap();
    if !text.is_empty() {
        s.execute("text.insert", json!({"target":target(0,0),"text":text}))
            .unwrap();
    }
    s.save_as(path).unwrap();
    s.archive().unwrap()
}
pub fn writer(path: &Path) -> Session {
    let mut s = Session::new();
    s.execute("document.open", json!({"path":path})).unwrap();
    s
}
pub fn source(path: &Path) -> String {
    typstographe_core::content::project::Project::read(path)
        .unwrap()
        .current()
        .clone()
        .validate()
        .unwrap()
}
pub fn job(folder: &Path, value: Value) -> std::path::PathBuf {
    let p = folder.join("job.json");
    std::fs::write(&p, serde_json::to_vec(&value).unwrap()).unwrap();
    p
}
pub fn script(steps: Value, save: Value) -> Value {
    json!({"version":1,"document":{"kind":"open","path":"original.typsto"},"steps":steps,"save":save})
}

pub fn has_mark(path: &Path, text: &str, kind: &str) -> bool {
    fn walk(n: &typstographe_core::content::Node, text: &str, kind: &str) -> bool {
        n.text.as_deref().is_some_and(|s| s.contains(text))
            && n.marks.iter().any(|m| m.kind == kind)
            || n.content.iter().any(|n| walk(n, text, kind))
    }
    walk(
        &typstographe_core::content::project::Project::read(path)
            .unwrap()
            .current()
            .clone()
            .document,
        text,
        kind,
    )
}
