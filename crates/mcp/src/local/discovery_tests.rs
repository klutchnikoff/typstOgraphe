use super::*;

#[test]
fn fixed_location_resolves_each_new_invitation_and_never_exposes_a_token() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    assert!(resolve(&root).is_err());
    for name in ["first.json", "second.json"] {
        let invitation = temp.path().join(name);
        let advertisement = Advertisement::publish(&root, &invitation).unwrap();
        assert_eq!(resolve(&root).unwrap(), invitation);
        let text = fs::read_to_string(&advertisement.record).unwrap();
        assert!(!text.contains("token") && !text.contains("address"));
        drop(advertisement);
        assert!(resolve(&root).is_err());
    }
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}
#[test]
fn competing_authorizations_are_ambiguous_and_ending_one_never_removes_the_other() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    let first = Advertisement::publish(&root, &temp.path().join("one.json")).unwrap();
    let second = Advertisement::publish(&root, &temp.path().join("two.json")).unwrap();
    assert!(
        resolve(&root)
            .unwrap_err()
            .to_string()
            .contains("Several invitations")
    );
    drop(first);
    assert_eq!(resolve(&root).unwrap(), temp.path().join("two.json"));
    drop(second);
    assert!(resolve(&root).is_err());
}
#[test]
fn crashed_publishers_are_ignored_even_when_the_invitation_file_still_exists() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    let old = temp.path().join("old.json");
    fs::write(&old, "stale").unwrap();
    let mut crashed = Advertisement::publish(&root, &old).unwrap();
    crashed.lock.take(); // Simulate OS lock release on process death, without cleanup.
    assert!(crashed.record.exists());
    assert!(resolve(&root).is_err());
    let fresh = Advertisement::publish(&root, &temp.path().join("new.json")).unwrap();
    assert_eq!(resolve(&root).unwrap(), temp.path().join("new.json"));
    drop(crashed);
    assert_eq!(resolve(&root).unwrap(), temp.path().join("new.json"));
    drop(fresh);
}
#[test]
fn live_malformed_or_oversized_records_are_refused_without_selecting_another_document() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    let record = Advertisement::publish(&root, &temp.path().join("valid.json")).unwrap();
    for data in [
        "{}".into(),
        "x".repeat(4097),
        r#"{"version":2,"invitation":"relative.json"}"#.into(),
    ] {
        fs::write(&record.record, data).unwrap();
        assert!(resolve(&root).is_err());
    }
}
#[cfg(unix)]
#[test]
fn rendezvous_is_private_and_refuses_symlinked_or_shared_directories_and_leases() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    let record = Advertisement::publish(&root, &temp.path().join("valid.json")).unwrap();
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for path in [&record.record, &record.lease] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let alias = temp.path().join("alias");
    symlink(&root, &alias).unwrap();
    assert!(resolve(&alias).is_err());
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(resolve(&root).is_err());
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let moved = record.lease.with_extension("moved");
    fs::rename(&record.lease, &moved).unwrap();
    symlink(&moved, &record.lease).unwrap();
    assert!(resolve(&root).is_err());
    assert!(Advertisement::publish(Path::new("relative"), Path::new("/unused")).is_err());
}
#[test]
fn successful_authentication_removes_discovery_but_keeps_the_existing_connection_scope() {
    use super::super::{LocalSession, transport};
    use std::{sync::Arc, time::Duration};
    use typstographe_core::{
        document_types::DocumentType, engine::Engine, storage::StoredDocument,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    let mut engine =
        Engine::from_stored(StoredDocument::demo(DocumentType::Courrier), None).unwrap();
    let mut session = LocalSession::start(&mut engine, Arc::new(|| {})).unwrap();
    session.advertise(&root).unwrap();
    let path = session.invitation_path().to_path_buf();
    assert_eq!(resolve(&root).unwrap(), path);
    let endpoint = session.invitation.data.clone();
    let (ready, connected) = std::sync::mpsc::channel();
    let (stop, finish) = std::sync::mpsc::channel();
    let client = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let stream = transport::connect(&endpoint).await.unwrap();
                ready.send(()).unwrap();
                finish.recv_timeout(Duration::from_secs(10)).unwrap();
                drop(stream);
            });
    });
    connected.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(session.connected());
    assert!(session.poll(&mut engine));
    assert!(resolve(&root).is_err());
    assert!(path.exists()); // Explicit endpoint still belongs to this connection.
    assert!(session.advertise(&root).is_err());
    drop(session);
    assert!(!path.exists());
    stop.send(()).unwrap();
    client.join().unwrap();
}
