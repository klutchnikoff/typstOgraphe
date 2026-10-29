use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
fn personal() -> Preferences {
    Preferences {
        profile: BTreeMap::from([("name".into(), "Camille Martin".into())]),
        personal: BTreeMap::from([("fr".into(), vec!["typstOgraphe".into()])]),
        signature: Some(Resource {
            path: "images/signature.svg".into(),
            data: STANDARD.encode(include_bytes!("../../../fixtures/signature.svg")),
        }),
        ..Default::default()
    }
}
#[test]
fn corrupt_oversized_and_invalid_preferences_are_reported_without_rewriting_them() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Preferences::load(dir.path()).unwrap().signature.is_none());
    let path = dir.path().join("preferences.json");
    for bytes in [
        b"broken JSON".to_vec(),
        br#"{"appearance":"invented"}"#.to_vec(),
        br#"{"signature":{"path":"../escape.svg","data":"AA=="}}"#.to_vec(),
    ] {
        std::fs::write(&path, &bytes).unwrap();
        assert!(Preferences::load(dir.path()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();
    assert!(Preferences::load(dir.path()).is_err());
    assert_eq!(file.metadata().unwrap().len(), 16 * 1024 * 1024 + 1);
}
#[test]
fn invalid_signature_and_io_failure_preserve_the_last_saved_personal_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let original = personal();
    original.save(dir.path()).unwrap();
    let path = dir.path().join("preferences.json");
    let before = std::fs::read(&path).unwrap();
    for data in ["not base64", "bm90IGFuIGltYWdl"] {
        let mut invalid = original.clone();
        invalid.signature.as_mut().unwrap().data = data.into();
        assert!(invalid.save(dir.path()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let backup = dir.path().join("original.json");
    std::fs::rename(&path, &backup).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut changed = original.clone();
    changed
        .profile
        .insert("name".into(), "Autre personne".into());
    assert!(changed.save(dir.path()).is_err());
    assert_eq!(std::fs::read(&backup).unwrap(), before);
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(backup, &path).unwrap();
    assert_eq!(
        Preferences::load(dir.path()).unwrap().profile,
        original.profile
    );
}
#[test]
fn recent_projects_are_deduplicated_and_bounded_without_losing_profile_signature_or_dictionary() {
    let dir = tempfile::tempdir().unwrap();
    let mut preferences = personal();
    for n in 0..15 {
        preferences.remember_project(Path::new(&format!("projet-{n}.typsto")));
    }
    preferences.remember_project(Path::new("projet-7.typsto"));
    preferences.save(dir.path()).unwrap();
    let restored = Preferences::load(dir.path()).unwrap();
    assert_eq!(restored.recent.len(), 12);
    assert_eq!(restored.recent[0], "projet-7.typsto");
    assert_eq!(
        restored
            .recent
            .iter()
            .filter(|p| *p == "projet-7.typsto")
            .count(),
        1
    );
    assert_eq!(restored.profile, preferences.profile);
    assert_eq!(restored.personal, preferences.personal);
    assert_eq!(
        restored.signature.unwrap().decoded().unwrap(),
        preferences.signature.unwrap().decoded().unwrap()
    );
}
