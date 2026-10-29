use super::*;
use std::fs::File;
fn archive() -> Archive {
    let state = super::super::contract::State::new(
        serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Texte à préserver 😀"}]},{"type":"paragraph"}]})).unwrap(),
        Default::default(),
    );
    Archive {
        document: state.document,
        metadata: state.settings.metadata,
        template: state.settings.template.clone(),
        model_id: state.settings.model_id,
        assets: Default::default(),
        dependencies: crate::packages::for_template(&state.settings.template),
        yjs: vec![],
    }
}
fn write(path: &Path, value: &Archive) {
    let mut lease = crate::DocumentLease::acquire(path).unwrap();
    value.save(&mut lease).unwrap();
}
fn rewrite(path: &Path, change: impl FnOnce(&mut Vec<(String, Vec<u8>)>)) {
    let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut entries = vec![];
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        let mut data = vec![];
        entry.read_to_end(&mut data).unwrap();
        entries.push((entry.name().into(), data));
    }
    drop(zip);
    change(&mut entries);
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    for (name, data) in entries {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&data).unwrap();
    }
    zip.finish().unwrap();
}
#[test]
fn corrupt_old_future_incomplete_and_tampered_archives_are_refused_without_extraction() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.typsto");
    std::fs::write(&path, b"not ZIP").unwrap();
    assert!(Archive::read(&path).is_err());
    for case in [
        "old", "future", "missing", "escape", "source", "main", "metadata", "shared", "template",
        "packages",
    ] {
        write(&path, &archive());
        rewrite(&path, |files| {
            let edit = |files: &mut Vec<(String, Vec<u8>)>, name: &str, data: Vec<u8>| {
                files.iter_mut().find(|(n, _)| n == name).unwrap().1 = data;
            };
            match case {
                "old"|"future" => edit(files,"manifest.json",serde_json::to_vec(&serde_json::json!({"format":"typstOgraphe","version":if case=="old" {9}else{999},"model_id":"embedded"})).unwrap()),
                "missing" => files.retain(|(n,_)|n!="document.json"),
                "escape" => files.push(("../outside.typ".into(),b"escape".to_vec())),
                "source" => edit(files,"contenu.typ",b"cache incoherent".to_vec()),
                "main" => edit(files,"main.typ",b"cache incoherent".to_vec()),
                "metadata" => edit(files,"metadonnees.json",b"invalid JSON".to_vec()),
                "shared" => edit(files,"state.yjs",vec![255]),
                "template" => edit(files,"modele.typ",vec![255]),
                "packages" => { let i=files.iter().position(|(n,_)|crate::packages::valid_path(n)).unwrap(); files.remove(i); },
                _ => unreachable!(),
            }
        });
        let bytes = std::fs::read(&path).unwrap();
        assert!(Archive::read(&path).is_err(), "{case}");
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!dir.path().join("outside.typ").exists());
        assert!(!dir.path().join("images").exists());
    }
}
#[test]
fn archive_size_limits_are_checked_before_loading_the_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.typsto");
    let file = File::create(&path).unwrap();
    file.set_len(MAX_DOCUMENT_BYTES + 2 * 1024 * 1024 + 1)
        .unwrap();
    drop(file);
    assert!(matches!(
        Archive::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::FileTooLarge
        )))
    ));
    let mut zip = ZipWriter::new(File::create(&path).unwrap());
    for i in 0..4097 {
        zip.start_file(
            format!("images/entry-{i}.svg"),
            SimpleFileOptions::default(),
        )
        .unwrap();
    }
    zip.finish().unwrap();
    assert!(matches!(
        Archive::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::FileCount
        )))
    ));
    let mut zip = ZipWriter::new(File::create(&path).unwrap());
    zip.start_file(
        "document.json",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
    )
    .unwrap();
    std::io::copy(
        &mut std::io::repeat(b' ').take(MAX_DOCUMENT_BYTES + 1),
        &mut zip,
    )
    .unwrap();
    zip.finish().unwrap();
    assert!(matches!(
        Archive::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::ExpandedTooLarge
        )))
    ));
}
