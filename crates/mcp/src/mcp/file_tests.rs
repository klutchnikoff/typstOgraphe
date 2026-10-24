use super::{host::Host, policy::Policy, roots::Roots};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use typstographe_core::engine::CommandError;

fn dispatch(host: &mut Host, id: &str, args: Value) -> Result<Value, CommandError> {
    host.dispatch(id, args.as_object().unwrap().clone())
        .map(|r| r.value)
}
fn call(host: &mut Host, id: &str, args: Value) -> Value {
    dispatch(host, id, args).unwrap()
}
fn host(read: &[&Path], write: &[&Path], read_only: bool) -> Host {
    Host::with_policy(
        Policy::new(
            read.iter().map(PathBuf::from).collect(),
            write.iter().map(PathBuf::from).collect(),
            read_only,
        )
        .unwrap(),
    )
}
fn create(host: &mut Host) -> Value {
    let mut params: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/capture-letter-params.json"
    ))
    .unwrap();
    params["type"] = json!("courrier");
    params["model"] = json!("courrier.model.personal");
    call(host, "document.new", params)["document"].clone()
}
fn fixture(path: &Path) {
    let mut h = host(&[], &[path.parent().unwrap()], false);
    let doc = create(&mut h);
    call(
        &mut h,
        "document.save_as",
        json!({"document":doc,"path":path}),
    );
}

#[test]
fn roots_are_existing_directories_independent_and_component_aware() {
    let dir = tempfile::tempdir().unwrap();
    let read = dir.path().join("read");
    let write = dir.path().join("write");
    let neighbor = dir.path().join("read-more");
    for p in [&read, &write, &neighbor] {
        std::fs::create_dir(p).unwrap();
    }
    std::fs::write(read.join("a.svg"), "test").unwrap();
    std::fs::write(neighbor.join("a.svg"), "secret").unwrap();
    let roots = Roots::new(vec![read.clone(), read.clone()], vec![write.clone()]).unwrap();
    assert_eq!(roots.describe()["read_roots"].as_array().unwrap().len(), 1);
    assert!(roots.read_file(&read.join("a.svg")).is_ok());
    assert!(roots.read_file(&neighbor.join("a.svg")).is_err());
    assert!(roots.write(&read.join("a.svg")).is_err());
    let directory_target = write.join("directory.typsto");
    std::fs::create_dir(&directory_target).unwrap();
    assert!(roots.document_write(&directory_target).is_err());
    std::fs::create_dir(write.join(".blocked.typsto.lock")).unwrap();
    assert!(roots.document_write(&write.join("blocked.typsto")).is_err());
    assert!(roots.read_file(&write.join("missing")).is_err());
    assert!(roots.write(&write.join("new/nested/export")).is_ok());
    assert!(roots.write(&write.join("../write/x")).is_err());
    assert!(roots.write(Path::new("")).is_err());
    assert!(Roots::new(vec![read.join("a.svg")], vec![]).is_err());
    assert!(Roots::new(vec![dir.path().join("missing")], vec![]).is_err());
    let cwd = std::env::current_dir().unwrap();
    let relative = Roots::new(vec![cwd], vec![]).unwrap();
    assert!(relative.read_file(Path::new("Cargo.toml")).is_ok());
}

#[cfg(unix)]
#[test]
fn symlinks_dangling_links_replaced_roots_and_lock_companions_cannot_escape() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("allowed");
    let other = dir.path().join("outside");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::fs::write(other.join("secret"), "secret").unwrap();
    std::fs::write(root.join("inside"), "inside").unwrap();
    symlink(&other, root.join("out")).unwrap();
    symlink(root.join("inside"), root.join("alias")).unwrap();
    symlink(other.join("missing"), root.join("dangling")).unwrap();
    symlink(other.join("secret"), root.join(".letter.typsto.lock")).unwrap();
    let roots = Roots::new(vec![root.clone()], vec![root.clone()]).unwrap();
    assert!(roots.read_file(&root.join("alias")).is_ok());
    for path in [
        root.join("out/secret"),
        root.join("out/new/nested"),
        root.join("dangling"),
    ] {
        assert!(roots.write(&path).is_err());
        assert!(roots.read_file(&path).is_err());
    }
    assert!(roots.document_write(&root.join("letter.typsto")).is_err());
    use std::os::unix::ffi::OsStringExt;
    let non_utf8 = root.join(std::ffi::OsString::from_vec(vec![0xff]));
    assert!(roots.document_write(&non_utf8).is_err());
    #[cfg(target_os = "linux")]
    {
        // APFS ne permet pas de créer ce nom ; ext4 permet de tester l’alias réel.
        std::fs::write(&non_utf8, "test").unwrap();
        symlink(&non_utf8, root.join("utf8-alias")).unwrap();
        assert!(roots.read_file(&root.join("utf8-alias")).is_err());
    }
    std::fs::rename(&root, dir.path().join("moved")).unwrap();
    symlink(&other, &root).unwrap();
    assert!(roots.write(&root.join("new")).is_err());
    assert_eq!(
        std::fs::read_to_string(other.join("secret")).unwrap(),
        "secret"
    );
}

#[test]
fn writable_open_requires_both_roots_and_read_only_server_can_open_and_close() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path);
    let dirs = [dir.path()];
    for (read, write) in [(true, false), (false, true)] {
        let mut h = host(
            if read { &dirs } else { &[] },
            if write { &dirs } else { &[] },
            false,
        );
        assert_eq!(
            dispatch(&mut h, "document.open", json!({"path":path}))
                .unwrap_err()
                .reason(),
            "mcp_file_access"
        );
    }
    let mut h = host(&[dir.path()], &[], true);
    assert_eq!(
        dispatch(&mut h, "document.open", json!({"path":path}))
            .unwrap_err()
            .reason(),
        "mcp_read_only"
    );
    let opened = call(
        &mut h,
        "document.open",
        json!({"path":path,"read_only":true}),
    );
    let doc = opened["document"].clone();
    assert_eq!(
        call(&mut h, "document.inspect", json!({}))["outcome"]["read_only"],
        true
    );
    for id in ["text.insert", "document.save_as", "document.export"] {
        let args = match id {
            "text.insert" => json!({"document":doc,"text":"No"}),
            "document.save_as" => json!({"document":doc,"path":dir.path().join("copy.typsto")}),
            _ => json!({"document":doc,"directory":dir.path().join("export")}),
        };
        assert_eq!(
            dispatch(&mut h, id, args).unwrap_err().reason(),
            "mcp_read_only"
        );
    }
    call(&mut h, "document.close", json!({"document":doc}));
    assert!(!dir.path().join("copy.typsto").exists());
}

#[test]
fn writer_lock_read_snapshot_copy_and_external_change_use_the_common_engine() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path);
    let mut owner = host(&[dir.path()], &[dir.path()], false);
    let opened = call(&mut owner, "document.open", json!({"path":path}));
    let original = opened["document"].clone();
    let edited = call(
        &mut owner,
        "text.insert",
        json!({"document":original,"text":"Not saved"}),
    );
    let mut peer = host(&[dir.path()], &[dir.path()], false);
    assert_eq!(
        dispatch(&mut peer, "document.open", json!({"path":path}))
            .unwrap_err()
            .reason(),
        "file_locked"
    );
    let reader = call(
        &mut peer,
        "document.open",
        json!({"path":path,"read_only":true}),
    );
    assert_eq!(
        call(
            &mut peer,
            "document.text",
            json!({"document":reader["document"]})
        )["outcome"]["text"],
        ""
    );
    assert_eq!(
        dispatch(
            &mut peer,
            "text.insert",
            json!({"document":reader["document"],"text":"No"})
        )
        .unwrap_err()
        .reason(),
        "file_read_only"
    );
    let copy = dir.path().join("copy.typsto");
    let copied = call(
        &mut peer,
        "document.save_as",
        json!({"document":reader["document"],"path":copy}),
    );
    call(
        &mut peer,
        "text.insert",
        json!({"document":copied["document"],"text":"Independent copy"}),
    );
    // Une réécriture externe qui ignore le verrou ne doit jamais être écrasée.
    std::fs::write(&path, b"external edit").unwrap();
    assert_eq!(
        dispatch(
            &mut owner,
            "document.save",
            json!({"document":edited["document"]})
        )
        .unwrap_err()
        .reason(),
        "file_changed"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
    call(
        &mut owner,
        "document.close",
        json!({"document":edited["document"],"discard_changes":true}),
    );
    // La fermeture doit libérer le verrou, même si le contenu n’est plus valide.
    fixture(&dir.path().join("replacement.typsto"));
    std::fs::copy(dir.path().join("replacement.typsto"), &path).unwrap();
    let mut reopened = host(&[dir.path()], &[dir.path()], false);
    call(&mut reopened, "document.open", json!({"path":path}));
}

#[test]
fn final_save_is_atomic_with_the_batch_and_export_is_separate() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("letter.typsto");
    let mut h = host(&[dir.path()], &[dir.path()], false);
    let doc = create(&mut h);
    let before = call(&mut h, "document.inspect", json!({}));
    std::fs::write(&out, "preserve").unwrap();
    let args = json!({"document":doc,"steps":[{"id":"text.insert","params":{"text":"Publication atomique"}}],"save":{"kind":"as","path":out}});
    assert!(dispatch(&mut h, "document.batch", args.clone()).is_err());
    assert_eq!(call(&mut h, "document.inspect", json!({})), before);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "preserve");
    let mut args = args;
    args["save"]["overwrite"] = json!(true);
    let saved = call(&mut h, "document.batch", args);
    assert!(
        saved["summary"]
            .as_str()
            .is_some_and(|s| !s.contains("no file"))
    );
    let disk = typstographe_core::storage::read(&out).unwrap();
    assert!(disk.source.contains("Publication atomique"));
    assert_eq!(
        call(&mut h, "document.inspect", json!({}))["outcome"]["dirty"],
        false
    );
    let bytes_before = std::fs::read(&out).unwrap();
    let export = dir.path().join("nested/export");
    call(
        &mut h,
        "document.export",
        json!({"document":saved["document"],"directory":export,"format":"both","page_images":false}),
    );
    assert!(export.join("courrier.pdf").is_file());
    assert!(export.join("main.typ").is_file());
    assert_eq!(std::fs::read(&out).unwrap(), bytes_before);
    let undo = call(&mut h, "edit.undo", json!({"document":saved["document"]}));
    assert_eq!(
        call(
            &mut h,
            "document.text",
            json!({"document":undo["document"]})
        )["outcome"]["text"],
        ""
    );
    assert!(
        dispatch(
            &mut h,
            "document.export",
            json!({"document":undo["document"],"directory":export})
        )
        .is_err()
    );
}

#[test]
fn imports_resolve_only_read_roots_and_are_embedded_before_save() {
    let dir = tempfile::tempdir().unwrap();
    let read = dir.path().join("inputs");
    let write = dir.path().join("outputs");
    std::fs::create_dir(&read).unwrap();
    std::fs::create_dir(&write).unwrap();
    let image = read.join("signature.svg");
    std::fs::write(&image, include_bytes!("../../../../fixtures/parc.svg")).unwrap();
    let model = read.join("model.typ");
    std::fs::write(
        &model,
        include_str!("../../../core/templates/courrier-personnel.typ"),
    )
    .unwrap();
    let mut h = host(&[&read], &[&write], false);
    let mut params: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/capture-letter-params.json"
    ))
    .unwrap();
    params["type"] = json!("courrier");
    params["model"] = json!("courrier.model.personal");
    params["signature"] = json!({"kind":"path","path":image});
    let new = call(&mut h, "document.new", params);
    let edited = call(
        &mut h,
        "document.batch",
        json!({"document":new["document"],"steps":[
        {"id":"insert.image","params":{"image":{"kind":"path","path":image},"caption":"Figure"}},
        {"id":"image.replace","params":{"selector":{"block":1},"image":{"kind":"path","path":image}}},
        {"id":"document.set_signature","params":{"image":{"kind":"path","path":image}}},
        {"id":"document.set_model","params":{"model":{"kind":"path","path":model}}}
    ],"save":{"kind":"as","path":write.join("letter.typsto")}}),
    );
    std::fs::remove_file(&image).unwrap();
    std::fs::remove_file(&model).unwrap();
    let disk = typstographe_core::storage::read(&write.join("letter.typsto")).unwrap();
    assert!(!disk.assets.is_empty());
    let validation = call(
        &mut h,
        "document.validate",
        json!({"document":edited["document"]}),
    );
    assert_eq!(validation["outcome"]["valid"], true);
    call(
        &mut h,
        "document.export",
        json!({"document":edited["document"],"directory":write.join("autonomous"),"format":"typst","page_images":false}),
    );
    assert!(write.join("autonomous/images").is_dir());
}

#[test]
fn forbidden_import_or_final_destination_never_publishes_edits_or_writes() {
    let dir = tempfile::tempdir().unwrap();
    let allowed = dir.path().join("allowed");
    std::fs::create_dir(&allowed).unwrap();
    let secret = dir.path().join("outside.svg");
    std::fs::write(&secret, include_bytes!("../../../../fixtures/parc.svg")).unwrap();
    let model = dir.path().join("outside.typ");
    std::fs::write(
        &model,
        include_str!("../../../core/templates/courrier-personnel.typ"),
    )
    .unwrap();
    let mut h = host(&[&allowed], &[&allowed], false);
    let doc = create(&mut h);
    let before = call(&mut h, "document.inspect", json!({}));
    for (id, mut args) in [
        (
            "document.new",
            json!({"type":"courrier","model":"courrier.model.personal","discard_changes":true,"signature":{"kind":"path","path":secret}}),
        ),
        (
            "insert.image",
            json!({"image":{"kind":"path","path":secret}}),
        ),
        (
            "image.replace",
            json!({"selector":{"block":1},"image":{"kind":"path","path":secret}}),
        ),
        (
            "document.set_signature",
            json!({"image":{"kind":"path","path":secret}}),
        ),
        (
            "document.set_model",
            json!({"model":{"kind":"path","path":model}}),
        ),
        (
            "document.save",
            json!({"path":dir.path().join("outside.typsto")}),
        ),
        (
            "document.save_as",
            json!({"path":dir.path().join("outside.typsto"),"overwrite":true}),
        ),
        (
            "document.export",
            json!({"directory":dir.path().join("outside-export")}),
        ),
        (
            "document.batch",
            json!({"steps":[{"id":"text.insert","params":{"text":"No"}},{"id":"insert.image","params":{"image":{"kind":"path","path":secret}}}],"save":{"kind":"as","path":allowed.join("late.typsto")}}),
        ),
        (
            "document.batch",
            json!({"steps":[{"id":"text.insert","params":{"text":"No"}}],"save":{"kind":"as","path":dir.path().join("outside.typsto")}}),
        ),
    ] {
        args["document"] = doc.clone();
        let error = dispatch(&mut h, id, args).unwrap_err();
        assert!(
            matches!(
                error.reason(),
                "mcp_file_access" | "batch_step" | "image_target"
            ),
            "{id}: {error}"
        );
        assert_eq!(call(&mut h, "document.inspect", json!({})), before);
    }
    assert_eq!(std::fs::read_dir(&allowed).unwrap().count(), 0);
    assert!(!dir.path().join("outside.typsto").exists());
    assert!(!dir.path().join("outside-export").exists());
}

#[test]
fn export_does_not_save_and_save_current_rechecks_the_destination() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let path = root.join("letter.typsto");
    let mut h = host(&[&root], &[&root], false);
    let doc = create(&mut h);
    let saved = call(
        &mut h,
        "document.save_as",
        json!({"document":doc,"path":path}),
    );
    let original = std::fs::read(&path).unwrap();
    let edited = call(
        &mut h,
        "text.insert",
        json!({"document":saved["document"],"text":"Unsaved changes"}),
    );
    call(
        &mut h,
        "document.export",
        json!({"document":edited["document"],"directory":root.join("pdf"),"format":"pdf","page_images":false}),
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(
        call(&mut h, "document.inspect", json!({}))["outcome"]["dirty"],
        true
    );
    let saved = call(
        &mut h,
        "document.batch",
        json!({"document":edited["document"],"steps":[{"id":"text.insert","params":{"text":" final"}}],"save":{"kind":"current"}}),
    );
    assert!(
        typstographe_core::storage::read(&path)
            .unwrap()
            .source
            .contains("final")
    );
    // Échec final de sauvegarde : le moteur conserve la session publiée et l’historique.
    let before = call(&mut h, "document.inspect", json!({}));
    std::fs::write(&path, b"external").unwrap();
    assert_eq!(dispatch(&mut h,"document.batch",json!({"document":saved["document"],"steps":[{"id":"text.insert","params":{"text":"not published"}}],"save":{"kind":"current"}})).unwrap_err().reason(),"file_changed");
    assert_eq!(call(&mut h, "document.inspect", json!({})), before);
    assert_eq!(std::fs::read(&path).unwrap(), b"external");
}
