use super::persistence::{open_project, save_project};
use crate::{
    rendering::Renderer,
    session::{Session, Snapshot},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use typstographe_core::{
    DocumentLease,
    assets::Assets,
    content::{
        archive::Archive,
        contract::{Settings, State},
        shared::SharedDocument,
    },
};

struct Fixture {
    session: Mutex<Session>,
    directory: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let state = State::new(
            serde_json::from_value(json!({"type":"doc","content":[
                {"type":"paragraph","content":[{"type":"text","text":"Courrier personnel 😀"}]},
                {"type":"paragraph"}
            ]}))
            .unwrap(),
            Settings::default(),
        );
        let mut session = Session::managed(directory.path().join("recovery")).unwrap();
        session.replace(
            state.document.clone(),
            state.settings.clone(),
            Assets::default(),
            SharedDocument::from_state(&state).unwrap().update(),
        );
        session.join("human");
        session.ensure_owner().unwrap().snapshot("human").unwrap();
        Self {
            session: Mutex::new(session),
            directory,
        }
    }
    fn path(&self) -> PathBuf {
        self.directory.path().join("letter.typsto")
    }
    fn draft(&self) -> (Snapshot, Vec<u8>) {
        let s = self.session.lock().unwrap();
        let owner = s.owner.as_ref().unwrap();
        (
            Snapshot {
                revision: s.revision,
                document: owner.state().document.clone(),
                settings: Some(s.settings.clone()),
            },
            owner.update_bytes(),
        )
    }
    fn recovery_path(&self) -> PathBuf {
        self.session.lock().unwrap().root.join("session.json")
    }
    fn persist_draft(&self) {
        let (snapshot, update) = self.draft();
        self.session
            .lock()
            .unwrap()
            .save(&snapshot, update)
            .unwrap();
    }
    fn edit(&self, text: &str) {
        let mut s = self.session.lock().unwrap();
        let guard = s.guard();
        let calls: Vec<typstographe_core::content::api::Call> =
            serde_json::from_value(json!([{"id":"text.insert","params":{
                "target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":text
            }}]))
            .unwrap();
        let publication = s
            .ensure_owner()
            .unwrap()
            .calls("human", &guard, &calls)
            .unwrap();
        s.record(&publication);
    }
    fn fingerprint(&self) -> Value {
        let mut s = self.session.lock().unwrap();
        let history = s.ensure_owner().unwrap().snapshot("human").unwrap();
        json!({"guard":s.guard(),"updates":s.updates,"state":s.owner.as_ref().unwrap().state(),
            "history":history,"clean":s.clean_revision,"lease":s.file_lease.as_ref().map(|l|l.path())})
    }
    fn save(&self, path: &Path) {
        let (snapshot, update) = self.draft();
        let renderer = Renderer::default();
        save_project(
            &self.session,
            path,
            snapshot,
            update,
            |source, assets, settings| renderer.validate(source, assets, settings),
        )
        .unwrap();
    }
    fn make_dirty(&self) {
        self.edit("Travail à conserver. ");
        self.persist_draft();
    }
}

#[test]
fn real_typst_save_and_reopen_preserve_checkpoint_and_release_recovery_only_after_success() {
    let f = Fixture::new();
    f.persist_draft();
    assert!(f.recovery_path().exists());
    f.save(&f.path());
    assert!(!f.recovery_path().exists());
    assert!(DocumentLease::acquire(&f.path()).is_err());
    f.make_dirty();
    let expected = f.draft();
    f.save(&f.path());
    let archive = Archive::read(&f.path()).unwrap();
    assert_eq!(archive.document, expected.0.document);
    SharedDocument::from_update(&archive.yjs)
        .unwrap()
        .verify(&archive.state())
        .unwrap();
    assert!(!f.recovery_path().exists());
    f.session.lock().unwrap().file_lease = None;
    let target = Mutex::new(Session::managed(f.directory.path().join("reopened")).unwrap());
    let renderer = Renderer::default();
    let response = open_project(&target, &f.path(), 0, |source, assets, settings| {
        renderer.validate(source, assets, settings)
    })
    .unwrap();
    assert_eq!(response["document"], json!(archive.document));
    let s = target.lock().unwrap();
    assert_eq!(s.clean_revision, Some(s.revision));
    assert_eq!(s.updates, vec![archive.yjs.clone()]);
    assert_eq!(s.settings, archive.state().settings);
    assert!(DocumentLease::acquire(&f.path()).is_err());
}

#[test]
fn failed_validation_bad_extension_and_incoherent_checkpoints_keep_disk_history_and_recovery() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let state = f.fingerprint();
    let disk = std::fs::read(f.path()).unwrap();
    let recovery = std::fs::read(f.recovery_path()).unwrap();
    let (snapshot, update) = f.draft();
    assert!(
        save_project(
            &f.session,
            &f.path(),
            snapshot.clone(),
            update.clone(),
            |_, _, _| Err("Typst refused".into())
        )
        .is_err()
    );
    assert!(
        save_project(
            &f.session,
            &f.path().with_extension("pdf"),
            snapshot.clone(),
            update,
            |_, _, _| panic!("invalid extension must precede compilation")
        )
        .is_err()
    );
    let mut different = f
        .session
        .lock()
        .unwrap()
        .owner
        .as_ref()
        .unwrap()
        .state()
        .clone();
    different.document.content[0].content[0].text = Some("Un autre état".into());
    let incompatible = SharedDocument::from_state(&different).unwrap().update();
    assert!(
        save_project(&f.session, &f.path(), snapshot, incompatible, |_, _, _| Ok(
            ()
        ))
        .is_err()
    );
    assert_eq!(f.fingerprint(), state);
    assert_eq!(std::fs::read(f.path()).unwrap(), disk);
    assert_eq!(std::fs::read(f.recovery_path()).unwrap(), recovery);
}

#[test]
fn a_document_edit_during_save_validation_cannot_be_marked_saved_or_overwritten() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let disk = std::fs::read(f.path()).unwrap();
    let (snapshot, update) = f.draft();
    let mut concurrent = Value::Null;
    let mut recovery = vec![];
    let result = save_project(&f.session, &f.path(), snapshot, update, |_, _, _| {
        f.edit("Modification pendant la compilation. ");
        f.persist_draft();
        concurrent = f.fingerprint();
        recovery = std::fs::read(f.recovery_path()).unwrap();
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(f.fingerprint(), concurrent);
    assert_eq!(std::fs::read(f.path()).unwrap(), disk);
    assert_eq!(std::fs::read(f.recovery_path()).unwrap(), recovery);
}

#[test]
fn empty_or_rebuilt_shared_history_cannot_replace_a_confirmed_project() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let state = f.fingerprint();
    let disk = std::fs::read(f.path()).unwrap();
    let recovery = std::fs::read(f.recovery_path()).unwrap();
    let (snapshot, _) = f.draft();
    let rebuilt =
        SharedDocument::from_state(f.session.lock().unwrap().owner.as_ref().unwrap().state())
            .unwrap()
            .update();
    for update in [vec![], rebuilt] {
        assert!(
            save_project(
                &f.session,
                &f.path(),
                snapshot.clone(),
                update,
                |_, _, _| Ok(())
            )
            .is_err()
        );
        assert_eq!(f.fingerprint(), state);
        assert_eq!(std::fs::read(f.path()).unwrap(), disk);
        assert_eq!(std::fs::read(f.recovery_path()).unwrap(), recovery);
    }
}

#[test]
fn external_changes_writer_locks_and_unwritable_targets_keep_the_current_work() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let state = f.fingerprint();
    let recovery = std::fs::read(f.recovery_path()).unwrap();
    let (snapshot, update) = f.draft();
    assert!(
        save_project(&f.session, &f.path(), snapshot, update, |_, _, _| {
            std::fs::write(f.path(), b"external bytes").unwrap();
            Ok(())
        })
        .is_err()
    );
    assert_eq!(std::fs::read(f.path()).unwrap(), b"external bytes");
    let locked = f.directory.path().join("locked.typsto");
    let _lease = DocumentLease::acquire(&locked).unwrap();
    let (snapshot, update) = f.draft();
    assert!(save_project(&f.session, &locked, snapshot, update, |_, _, _| Ok(())).is_err());
    assert!(!locked.exists());
    let parent = f.directory.path().join("not-a-directory");
    std::fs::write(&parent, b"keep").unwrap();
    let (snapshot, update) = f.draft();
    assert!(
        save_project(
            &f.session,
            &parent.join("failed.typsto"),
            snapshot,
            update,
            |_, _, _| Ok(())
        )
        .is_err()
    );
    assert_eq!(std::fs::read(parent).unwrap(), b"keep");
    assert_eq!(f.fingerprint(), state);
    assert_eq!(std::fs::read(f.recovery_path()).unwrap(), recovery);
    assert!(DocumentLease::acquire(&f.path()).is_err());
}

#[test]
fn refused_opens_preserve_current_document_lock_and_recovery() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let state = f.fingerprint();
    let recovery = std::fs::read(f.recovery_path()).unwrap();
    let revision = f.session.lock().unwrap().revision;
    let corrupt = f.directory.path().join("corrupt.typsto");
    std::fs::write(&corrupt, b"broken archive").unwrap();
    assert!(
        open_project(&f.session, &corrupt, revision, |_, _, _| panic!(
            "corrupt input must not reach Typst"
        ))
        .is_err()
    );
    let other = Fixture::new();
    other.save(&other.path());
    assert!(
        open_project(&f.session, &other.path(), revision, |_, _, _| panic!(
            "writer lock must precede reading"
        ))
        .is_err()
    );
    other.session.lock().unwrap().file_lease = None;
    assert!(
        open_project(&f.session, &other.path(), revision, |_, _, _| Err(
            "Typst refused".into()
        ))
        .is_err()
    );
    assert_eq!(f.fingerprint(), state);
    assert_eq!(std::fs::read(f.recovery_path()).unwrap(), recovery);
    assert!(DocumentLease::acquire(&f.path()).is_err());
    assert!(DocumentLease::acquire(&other.path()).is_ok());
}

#[test]
fn edits_and_disconnections_during_open_validation_block_replacement() {
    for disconnect in [false, true] {
        let f = Fixture::new();
        f.save(&f.path());
        f.make_dirty();
        let other = Fixture::new();
        other.save(&other.path());
        other.session.lock().unwrap().file_lease = None;
        let disk = std::fs::read(f.path()).unwrap();
        let target = std::fs::read(other.path()).unwrap();
        let revision = f.session.lock().unwrap().revision;
        let mut expected = Value::Null;
        let result = open_project(&f.session, &other.path(), revision, |_, _, _| {
            if disconnect {
                let mut s = f.session.lock().unwrap();
                s.join("other");
                let generation = s.generation.clone();
                s.connection("other", &generation, false).unwrap();
            } else {
                f.edit("Nouvelle saisie. ");
                f.persist_draft();
            }
            expected = f.fingerprint();
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(f.fingerprint(), expected);
        assert!(f.recovery_path().exists());
        assert_eq!(std::fs::read(f.path()).unwrap(), disk);
        assert_eq!(std::fs::read(other.path()).unwrap(), target);
        assert!(DocumentLease::acquire(&other.path()).is_ok());
    }
}

#[test]
fn failed_recovery_cleanup_cannot_discard_the_open_document_or_original_draft() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let other = Fixture::new();
    other.save(&other.path());
    other.session.lock().unwrap().file_lease = None;
    let state = f.fingerprint();
    let checkpoint = std::fs::read(f.recovery_path()).unwrap();
    let backup = f.recovery_path().with_extension("backup");
    std::fs::rename(f.recovery_path(), &backup).unwrap();
    std::fs::create_dir(f.recovery_path()).unwrap();
    let revision = f.session.lock().unwrap().revision;
    assert!(open_project(&f.session, &other.path(), revision, |_, _, _| Ok(())).is_err());
    assert_eq!(f.fingerprint(), state);
    assert_eq!(std::fs::read(backup).unwrap(), checkpoint);
    assert!(DocumentLease::acquire(&f.path()).is_err());
    assert!(DocumentLease::acquire(&other.path()).is_ok());
}

#[path = "lifecycle_tests.rs"]
mod lifecycle_tests;
#[path = "resource_tests.rs"]
mod resource_tests;

#[test]
fn external_replacement_during_open_validation_cannot_publish_or_clean_the_current_work() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let other = Fixture::new();
    other.save(&other.path());
    other.session.lock().unwrap().file_lease = None;
    let before = f.fingerprint();
    let recovery = std::fs::read(f.recovery_path()).unwrap();
    let revision = f.session.lock().unwrap().revision;
    assert!(
        open_project(&f.session, &other.path(), revision, |_, _, _| {
            std::fs::write(other.path(), b"A newer external version").unwrap();
            Ok(())
        })
        .is_err()
    );
    assert_eq!(f.fingerprint(), before);
    assert_eq!(std::fs::read(f.recovery_path()).unwrap(), recovery);
    assert_eq!(
        std::fs::read(other.path()).unwrap(),
        b"A newer external version"
    );
    assert!(DocumentLease::acquire(&f.path()).is_err());
    assert!(DocumentLease::acquire(&other.path()).is_ok());
}

#[test]
fn failed_save_as_cleanup_keeps_the_original_binding_and_external_change_protection() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let before = f.fingerprint();
    let checkpoint = std::fs::read(f.recovery_path()).unwrap();
    let backup = f.recovery_path().with_extension("backup");
    std::fs::rename(f.recovery_path(), &backup).unwrap();
    std::fs::create_dir(f.recovery_path()).unwrap();
    std::fs::write(f.path(), b"A newer external original").unwrap();
    let copy = f.directory.path().join("copy.typsto");
    let (snapshot, update) = f.draft();
    assert!(save_project(&f.session, &copy, snapshot, update, |_, _, _| Ok(())).is_err());
    assert!(Archive::read(&copy).is_ok()); // The archive was written, but cleanup did not finish.
    let after = f.fingerprint();
    assert_eq!(after["lease"], before["lease"]);
    assert!(after == before, "Refused Save As changed current work");
    assert_eq!(std::fs::read(&backup).unwrap(), checkpoint);
    std::fs::remove_dir(f.recovery_path()).unwrap();
    std::fs::rename(backup, f.recovery_path()).unwrap();
    let (snapshot, update) = f.draft();
    assert!(save_project(&f.session, &f.path(), snapshot, update, |_, _, _| Ok(())).is_err());
    assert_eq!(
        std::fs::read(f.path()).unwrap(),
        b"A newer external original"
    );
}

#[test]
fn retrying_an_in_place_save_after_cleanup_failure_recognizes_its_own_written_file() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let path = f.recovery_path();
    let backup = path.with_extension("backup");
    std::fs::rename(&path, &backup).unwrap();
    std::fs::create_dir(&path).unwrap();
    let (snapshot, update) = f.draft();
    let expected = snapshot.document.clone();
    assert!(save_project(&f.session, &f.path(), snapshot, update, |_, _, _| Ok(())).is_err());
    assert_eq!(Archive::read(&f.path()).unwrap().document, expected);
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(backup, &path).unwrap();
    f.save(&f.path());
    assert!(!path.exists());
    assert!(DocumentLease::acquire(&f.path()).is_err());
}
