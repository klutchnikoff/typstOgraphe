use super::*;
use std::path::Path;

fn open(engine: &mut Engine, path: &Path, read_only: bool) -> Result<Outcome, CommandError> {
    engine.run(Command::OpenDocument(OpenDocumentParams {
        guard: engine.target_guard(),
        path: path.into(),
        read_only,
        discard_changes: true,
    }))
}
fn save(engine: &mut Engine) -> Result<Outcome, CommandError> {
    engine.run(Command::SaveDocument(SaveDocumentParams {
        guard: engine.target_guard(),
        path: None,
    }))
}
fn fixture(path: &Path) {
    crate::storage::save(path, &StoredDocument::demo(DocumentType::Courrier)).unwrap();
}

#[test]
fn one_writer_and_multiple_snapshot_readers_survive_atomic_saves_and_release_on_close() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Été 日本.typsto");
    fixture(&path);
    let mut writer = Engine::empty();
    let mut reader = Engine::empty();
    let mut second = Engine::empty();
    open(&mut writer, &path, false).unwrap();
    open(&mut reader, &path, true).unwrap();
    let snapshot = reader.stored_document();
    assert_eq!(
        open(&mut second, &path, false).unwrap_err().reason(),
        "file_locked"
    );
    assert!(!second.is_open());
    call(&mut writer, "text.insert", json!({"text":"Ajout "})).unwrap();
    save(&mut writer).unwrap();
    // Le remplacement atomique n’a pas perdu le verrou, et le lecteur est un instantané.
    assert_eq!(
        open(&mut second, &path, false).unwrap_err().reason(),
        "file_locked"
    );
    assert!(reader.stored_document() == snapshot);
    open(&mut reader, &path, true).unwrap();
    assert_ne!(reader.stored_document().source, snapshot.source);
    writer
        .run(Command::CloseDocument(CloseDocumentParams {
            guard: writer.target_guard(),
            discard_changes: true,
        }))
        .unwrap();
    open(&mut second, &path, false).unwrap();
    drop(second);
    // Le compagnon existe encore, sans verrou périmé.
    assert!(dir.path().join(".Été 日本.typsto.lock").exists());
    open(&mut writer, &path, false).unwrap();
}

#[test]
fn read_only_blocks_commands_and_native_edits_but_allows_a_new_editable_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path);
    let mut reader = Engine::empty();
    open(&mut reader, &path, true).unwrap();
    let before = reader.stored_document();
    let draft = reader.begin_native_edit(reader.selection());
    assert_eq!(
        reader.commit_native_edit(
            draft.clone(),
            draft.view().clone(),
            reader.selection(),
            NativeInput::Typing
        ),
        Err(crate::document::edits::EditError::ReadOnly)
    );
    assert_eq!(
        call(&mut reader, "text.insert", json!({"text":"Non"}))
            .unwrap_err()
            .reason(),
        "file_read_only"
    );
    assert_eq!(save(&mut reader).unwrap_err().reason(), "file_read_only");
    assert_eq!(
        reader.query(Query::Inspect(Empty {})).unwrap()["read_only"],
        true
    );
    let specs = reader.query(Query::Commands(Empty {})).unwrap();
    assert_eq!(
        specs["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "metadata.set")
            .unwrap()["context_availability"]["state"],
        "disabled"
    );
    let copy = dir.path().join("copy.typsto");
    reader
        .run(Command::SaveAs(SaveAsParams {
            guard: reader.target_guard(),
            path: copy,
            overwrite: false,
        }))
        .unwrap();
    assert!(!reader.is_read_only());
    call(&mut reader, "text.insert", json!({"text":"Oui "})).unwrap();
    save(&mut reader).unwrap();
    assert_eq!(crate::storage::read(&path).unwrap(), before);
}

#[test]
fn external_change_same_size_or_deletion_refuses_save_without_publishing_or_losing_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path);
    let mut engine = Engine::empty();
    open(&mut engine, &path, false).unwrap();
    call(&mut engine, "text.insert", json!({"text":"Brouillon "})).unwrap();
    let before = engine.stored_document();
    let guard = engine.target_guard();
    let mut external = std::fs::read(&path).unwrap();
    let len = external.len();
    external[len - 1] ^= 1;
    std::fs::write(&path, &external).unwrap();
    assert_eq!(save(&mut engine).unwrap_err().reason(), "file_changed");
    assert_eq!(std::fs::read(&path).unwrap(), external);
    assert_eq!(engine.target_guard(), guard);
    assert_eq!(engine.stored_document(), before);
    assert!(engine.is_dirty());
    std::fs::remove_file(&path).unwrap();
    assert_eq!(save(&mut engine).unwrap_err().reason(), "file_changed");
    assert!(!path.exists());
    engine
        .run(Command::SaveAs(SaveAsParams {
            guard,
            path: dir.path().join("rescued.typsto"),
            overwrite: false,
        }))
        .unwrap();
}

#[test]
fn failed_replacement_keeps_old_lock_and_save_as_transfers_it_only_after_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.typsto");
    fixture(&path);
    let other = dir.path().join("other.typsto");
    fixture(&other);
    let mut engine = Engine::empty();
    open(&mut engine, &path, false).unwrap();
    let mut blocker = Engine::empty();
    open(&mut blocker, &other, false).unwrap();
    let guard = engine.target_guard();
    assert_eq!(
        open(&mut engine, &other, false).unwrap_err().reason(),
        "file_locked"
    );
    assert_eq!(engine.target_guard(), guard);
    assert_eq!(
        open(&mut blocker, &path, false).unwrap_err().reason(),
        "file_locked"
    );
    assert_eq!(
        engine
            .run(Command::SaveAs(SaveAsParams {
                guard,
                path: other.clone(),
                overwrite: true
            }))
            .unwrap_err()
            .reason(),
        "file_locked"
    );
    drop(blocker);
    engine
        .run(Command::SaveAs(SaveAsParams {
            guard,
            path: other.clone(),
            overwrite: true,
        }))
        .unwrap();
    let mut next = Engine::empty();
    open(&mut next, &path, false).unwrap();
    assert_eq!(
        open(&mut next, &other, false).unwrap_err().reason(),
        "file_locked"
    );
}

#[test]
fn batch_save_shares_the_lease_and_updates_the_fingerprint_without_releasing_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path);
    let mut engine = Engine::empty();
    open(&mut engine, &path, false).unwrap();
    let params = BatchParams {
        guard: engine.target_guard(),
        steps: vec![
            Command::InsertText(InsertTextParams {
                target: TextTarget::default(),
                text: "Un ".into(),
                input_rules: InputRules::Literal,
            })
            .to_call(),
        ],
        save: Some(BatchSave::Current {}),
    };
    engine.run(Command::Batch(params)).unwrap();
    save(&mut engine).unwrap();
    let mut second = Engine::empty();
    assert_eq!(
        open(&mut second, &path, false).unwrap_err().reason(),
        "file_locked"
    );
    let before = std::fs::read(&path).unwrap();
    let guard = engine.target_guard();
    let params = BatchParams {
        guard,
        steps: vec![
            Command::InsertText(InsertTextParams {
                target: TextTarget::default(),
                text: "Non ".into(),
                input_rules: InputRules::Literal,
            })
            .to_call(),
        ],
        save: Some(BatchSave::As {
            path: path.clone(),
            overwrite: false,
        }),
    };
    assert!(engine.run(Command::Batch(params)).is_err());
    assert_eq!(engine.target_guard(), guard);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        open(&mut second, &path, false).unwrap_err().reason(),
        "file_locked"
    );
}

#[cfg(unix)]
#[test]
fn symlink_and_relative_aliases_use_the_same_lock_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path);
    let alias = dir.path().join("alias.typsto");
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    let mut engine = Engine::empty();
    open(&mut engine, &path, false).unwrap();
    let mut second = Engine::empty();
    assert_eq!(
        open(&mut second, &alias, false).unwrap_err().reason(),
        "file_locked"
    );
    open(&mut engine, &alias, false).unwrap();
    save(&mut engine).unwrap();
    assert!(alias.symlink_metadata().unwrap().is_symlink());
}

// Le même exécutable de tests sert de processus détenteur, sans binaire auxiliaire public.
#[test]
fn process_lock_holder() {
    let Some(path) = std::env::var_os("TYPSTOGRAPHE_LOCK_TEST_PATH") else {
        return;
    };
    let mut engine = Engine::empty();
    open(&mut engine, Path::new(&path), false).unwrap();
    use std::io::Write;
    println!("LOCK_READY");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}

#[test]
fn another_process_holds_the_lock_and_crashing_it_releases_the_os_lock() {
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    fixture(&path);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "engine::tests::locking::process_lock_holder",
            "--nocapture",
        ])
        .env("TYPSTOGRAPHE_LOCK_TEST_PATH", &path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = false;
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        if line.unwrap().contains("LOCK_READY") {
            ready = true;
            break;
        }
    }
    assert!(ready);
    let mut engine = Engine::empty();
    assert_eq!(
        open(&mut engine, &path, false).unwrap_err().reason(),
        "file_locked"
    );
    open(&mut engine, &path, true).unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    open(&mut engine, &path, false).unwrap();
}
