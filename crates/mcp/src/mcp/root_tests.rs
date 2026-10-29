use super::roots::Roots;
use std::path::Path;
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
