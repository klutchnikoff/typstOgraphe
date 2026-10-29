use super::super::persistence::close_project;
use super::*;
#[test]
fn refused_creation_and_closure_keep_the_dirty_document_lease_and_recovery() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let before = f.fingerprint();
    let bytes = std::fs::read(f.recovery_path()).unwrap();
    let revision = f.session.lock().unwrap().revision;
    {
        let mut s = f.session.lock().unwrap();
        assert!(super::super::create(&mut s, Settings::default(), revision - 1, vec![]).is_err());
        assert!(close_project(&mut s, revision - 1).is_err());
        s.join("colleague");
        let generation = s.generation.clone();
        s.connection("colleague", &generation, false).unwrap();
        assert!(super::super::create(&mut s, Settings::default(), revision, vec![]).is_err());
        assert!(close_project(&mut s, revision).is_err());
        s.connection("colleague", &generation, true).unwrap();
    }
    assert_eq!(f.fingerprint(), before);
    assert_eq!(std::fs::read(f.recovery_path()).unwrap(), bytes);
    assert!(DocumentLease::acquire(&f.path()).is_err());
}
#[test]
fn failed_cleanup_blocks_creation_and_closure_then_explicit_closure_releases_the_lease() {
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let before = f.fingerprint();
    let path = f.recovery_path();
    let backup = path.with_extension("backup");
    std::fs::rename(&path, &backup).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut s = f.session.lock().unwrap();
    let revision = s.revision;
    assert!(super::super::create(&mut s, Settings::default(), revision, vec![]).is_err());
    assert!(close_project(&mut s, revision).is_err());
    drop(s);
    assert_eq!(f.fingerprint(), before);
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(backup, &path).unwrap();
    let mut s = f.session.lock().unwrap();
    let generation = s.generation.clone();
    close_project(&mut s, revision).unwrap();
    assert!(!s.opened);
    assert!(s.owner.is_none());
    assert!(s.file_lease.is_none());
    assert_ne!(s.generation, generation);
    assert!(s.check_peer("human", &generation).is_err());
    assert!(!path.exists());
    assert!(DocumentLease::acquire(&f.path()).is_ok());
    let revision = s.revision;
    super::super::create(&mut s, Settings::default(), revision, vec![]).unwrap();
    assert!(s.opened);
    assert!(s.updates.is_empty());
    assert!(s.assets.is_empty());
    assert!(s.fixture.as_ref().unwrap().content[0].content.is_empty());
}

#[test]
fn new_signed_document_requires_valid_resources_before_clearing_the_previous_work() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let f = Fixture::new();
    f.save(&f.path());
    f.make_dirty();
    let before = f.fingerprint();
    let recovery = std::fs::read(f.recovery_path()).unwrap();
    let mut settings = Settings::default();
    let typstographe_core::document_types::Metadata::Courrier(meta) = &mut settings.metadata else {
        unreachable!()
    };
    meta.signature = Some("images/signature.svg".into());
    let mut s = f.session.lock().unwrap();
    let revision = s.revision;
    for resources in [
        vec![],
        vec![crate::session::Resource {
            path: "images/signature.svg".into(),
            data: "not base64".into(),
        }],
    ] {
        assert!(super::super::create(&mut s, settings.clone(), revision, resources).is_err());
    }
    drop(s);
    assert_eq!(f.fingerprint(), before);
    assert_eq!(std::fs::read(f.recovery_path()).unwrap(), recovery);
    let mut s = f.session.lock().unwrap();
    let old = s.generation.clone();
    super::super::create(
        &mut s,
        settings,
        revision,
        vec![crate::session::Resource {
            path: "images/signature.svg".into(),
            data: STANDARD.encode(include_bytes!("../../../../fixtures/signature.svg")),
        }],
    )
    .unwrap();
    assert_ne!(s.generation, old);
    assert_eq!(s.assets.len(), 1);
    assert_eq!(
        s.settings.metadata.signature_path(),
        Some("images/signature.svg")
    );
    assert!(s.file_lease.is_none());
    assert!(s.updates.is_empty());
    assert!(!s.root.join("session.json").exists());
}
