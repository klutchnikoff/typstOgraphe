use super::tests::{confirmed, entries, fixture, version};
use super::*;
use crate::document_types::DocumentType;
use std::{collections::BTreeMap, fs, io::Write};

fn write_zip(path: &Path, files: BTreeMap<String, Vec<u8>>) {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (path, bytes) in files {
        zip.start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn zip_duplicate_names_truncation_directories_and_symlinks_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zip.typsto");
    let project = Project::new(fixture(DocumentType::Courrier, "ZIP safety")).unwrap();
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let original = entries(&path);
    // The writer refuses duplicates, so forge two names of identical length.
    let mut files = original.clone();
    files.insert("aaaa".into(), vec![]);
    files.insert("bbbb".into(), vec![]);
    write_zip(&path, files);
    let mut bytes = fs::read(&path).unwrap();
    let mut replacements = 0;
    for offset in 0..bytes.len().saturating_sub(3) {
        if &bytes[offset..offset + 4] == b"bbbb" {
            bytes[offset..offset + 4].copy_from_slice(b"aaaa");
            replacements += 1;
        }
    }
    assert_eq!(replacements, 2);
    fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        Project::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::DuplicateEntry { .. }
        )))
    ));
    fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    assert!(Project::read(&path).is_err());
    write_zip(&path, original.clone());
    let mut zip = zip::ZipWriter::new_append(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap(),
    )
    .unwrap();
    zip.add_directory("objects/", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.finish().unwrap();
    assert!(Project::read(&path).is_err());
    write_zip(&path, original);
    let mut zip = zip::ZipWriter::new_append(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap(),
    )
    .unwrap();
    zip.add_symlink(
        format!("objects/sha256/{}", "0".repeat(64)),
        "/tmp/escape",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.finish().unwrap();
    assert!(Project::read(&path).is_err());
}

fn central_headers(bytes: &[u8]) -> Vec<usize> {
    bytes
        .windows(4)
        .enumerate()
        .filter_map(|(i, b)| (b == b"PK\x01\x02").then_some(i))
        .collect()
}
#[test]
fn archive_state_checkpoint_image_and_index_budgets_are_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("budgets.typsto");
    let project = Project::new(fixture(DocumentType::Courrier, "Budgets")).unwrap();
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let original = fs::read(&path).unwrap();
    let descriptor: schema::StateDescriptor = project.store.read_json(&project.current_id).unwrap();
    for (name, size) in [
        ("versions.json".into(), schema::MAX_INDEX_BYTES + 1),
        ("manifest.json".into(), schema::MAX_MANIFEST_BYTES + 1),
        (
            descriptor.yjs.object.path(),
            crate::content::shared::MAX_STATE_BYTES as u64 + 1,
        ),
        (
            descriptor.document.path(),
            crate::assets::MAX_DOCUMENT_BYTES + 1,
        ),
    ] {
        let mut bytes = original.clone();
        let mut changed = false;
        for i in central_headers(&bytes) {
            let len = u16::from_le_bytes(bytes[i + 28..i + 30].try_into().unwrap()) as usize;
            if &bytes[i + 46..i + 46 + len] == name.as_bytes() {
                bytes[i + 24..i + 28].copy_from_slice(&(size as u32).to_le_bytes());
                changed = true;
            }
        }
        assert!(changed);
        fs::write(&path, bytes).unwrap();
        assert!(Project::read(&path).is_err(), "{name}");
    }
    // Repeated logical paths count towards the state budget even if bytes deduplicate.
    let mut files = entries_from_bytes(&original);
    let payload = vec![b'x'; 1024 * 1024];
    let payload_id = ObjectId::of(&payload);
    files.insert(payload_id.path(), payload);
    let resources: schema::ResourceMap = (0..65)
        .map(|n| (format!("capabilities/file-{n}.typ"), payload_id.clone()))
        .collect();
    let mut descriptor = descriptor;
    let resources = serde_json::to_vec(&resources).unwrap();
    descriptor.resources = ObjectId::of(&resources);
    files.insert(descriptor.resources.path(), resources);
    let bytes = serde_json::to_vec(&descriptor).unwrap();
    let id = ObjectId::of(&bytes);
    files.insert(id.path(), bytes);
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["current"] = serde_json::to_value(id).unwrap();
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    write_zip(&path, files);
    assert!(matches!(
        Project::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::LetterTooLarge
        )))
    ));
    // A sparse over-limit file is refused before parsing or fingerprinting its contents.
    let file = fs::File::create(&path).unwrap();
    file.set_len(MAX_ARCHIVE_BYTES + 1).unwrap();
    drop(file);
    assert!(matches!(
        Project::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::FileTooLarge
        )))
    ));
    assert!(DocumentLease::acquire(&path).is_err());
}
fn entries_from_bytes(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut e = zip.by_index(i).unwrap();
            let name = e.name().unwrap().into_owned();
            let mut data = vec![];
            e.read_to_end(&mut data).unwrap();
            (name, data)
        })
        .collect()
}

#[test]
fn expanded_archive_and_object_count_limits_are_checked_without_inflating_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("expanded.typsto");
    let project = Project::new(fixture(DocumentType::Courrier, "Expanded")).unwrap();
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let mut bytes = fs::read(&path).unwrap();
    let mut objects = 0;
    for i in central_headers(&bytes) {
        let len = u16::from_le_bytes(bytes[i + 28..i + 30].try_into().unwrap()) as usize;
        if bytes[i + 46..i + 46 + len].starts_with(b"objects/") {
            bytes[i + 24..i + 28]
                .copy_from_slice(&(crate::assets::MAX_DOCUMENT_BYTES as u32).to_le_bytes());
            objects += 1;
        }
    }
    assert!(objects > 8);
    fs::write(&path, bytes).unwrap();
    assert!(matches!(
        Project::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::ExpandedTooLarge
        )))
    ));
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    for n in 0..MAX_OBJECTS + 3 {
        zip.start_file(
            format!("objects/sha256/{n:064x}"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    }
    zip.finish().unwrap();
    assert!(matches!(
        Project::read(&path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::FileCount
        )))
    ));
}

#[test]
fn file_guards_can_fingerprint_projects_larger_than_the_former_current_state_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.typsto");
    let file = fs::File::create(&path).unwrap();
    file.set_len(crate::assets::MAX_DOCUMENT_BYTES + 3 * 1024 * 1024)
        .unwrap();
    drop(file);
    DocumentLease::acquire(&path).unwrap().check().unwrap();
}

#[test]
fn image_limit_is_checked_before_decoding_and_old_formats_are_not_migrated_implicitly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("image.typsto");
    let project = Project::new(super::tests::image_fixture("limit")).unwrap();
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let image = ObjectId::of(&project.current().assets["images/limit.svg"]);
    let mut bytes = fs::read(&path).unwrap();
    for i in central_headers(&bytes) {
        let len = u16::from_le_bytes(bytes[i + 28..i + 30].try_into().unwrap()) as usize;
        if bytes[i + 46..i + 46 + len] == *image.path().as_bytes() {
            bytes[i + 24..i + 28]
                .copy_from_slice(&((crate::assets::MAX_IMAGE_BYTES + 1) as u32).to_le_bytes());
        }
    }
    fs::write(&path, bytes).unwrap();
    assert!(
        matches!(Project::read(&path), Err(Error::Core(CoreError::StorageError(StorageError::EntryTooLarge { name }))) if name == "images/limit.svg")
    );
    let old_path = dir.path().join("v10.typsto");
    write_zip(
        &old_path,
        std::collections::BTreeMap::from([(
            "manifest.json".into(),
            br#"{"format":"typstOgraphe","version":10}"#.to_vec(),
        )]),
    );
    assert!(matches!(
        Project::read(&old_path),
        Err(Error::Core(CoreError::StorageError(
            StorageError::Unsupported
        )))
    ));
}
#[test]
fn external_changes_and_failed_history_rewrite_leave_file_and_fingerprint_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("protected.typsto");
    let project = version(
        &Project::new(fixture(DocumentType::Courrier, "Old")).unwrap(),
        "old",
    );
    let project = project
        .with_current(confirmed(fixture(DocumentType::Courrier, "New")))
        .unwrap();
    let mut lease = DocumentLease::acquire(&path).unwrap();
    project.save(&mut lease).unwrap();
    let mut files = entries(&path);
    let old_id = project.versions()[0].id;
    let descriptor: schema::StateDescriptor = project
        .store
        .read_json(&project.versions()[0].state)
        .unwrap();
    files.insert(
        descriptor.sources.body.path(),
        b"corrupt old source".to_vec(),
    );
    write_zip(&path, files);
    assert!(project.save(&mut lease).is_err()); // external change is detected
    lease.record_save().unwrap(); // explicit reopen observes the changed file
    let read = Project::read(&path).unwrap(); // opening does not read old bodies
    assert!(matches!(
        read.version(old_id),
        Err(Error::Core(CoreError::StorageError(
            StorageError::ObjectIntegrity { .. }
        )))
    ));
    let before = fs::read(&path).unwrap();
    let candidate = version(&read, "A new version must not partially publish");
    assert!(candidate.save(&mut lease).is_err()); // streaming rewrite catches the corrupt old object
    assert_eq!(before, fs::read(&path).unwrap());
    assert_eq!(Project::read(&path).unwrap().versions(), read.versions());
    lease.check().unwrap();
    let removed = read.prepare_remove_version(old_id).unwrap();
    removed.save(&mut lease).unwrap();
    assert!(Project::read(&path).unwrap().versions().is_empty());
}
#[test]
fn bad_retained_descriptor_refuses_gc_but_a_valid_current_state_still_opens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("descriptor.typsto");
    let project = version(
        &Project::new(fixture(DocumentType::Article, "Old")).unwrap(),
        "old",
    )
    .with_current(confirmed(fixture(DocumentType::Article, "New")))
    .unwrap();
    let mut lease = DocumentLease::acquire(&path).unwrap();
    project.save(&mut lease).unwrap();
    let mut files = entries(&path);
    files.remove(&project.versions()[0].state.path());
    write_zip(&path, files);
    lease.record_save().unwrap();
    let read = Project::read(&path).unwrap();
    let before = fs::read(&path).unwrap();
    assert!(read.save(&mut lease).is_err());
    assert_eq!(before, fs::read(&path).unwrap());
}
#[test]
fn current_object_corruption_is_never_repaired_from_derived_sources() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.typsto");
    let project = Project::new(fixture(DocumentType::Courrier, "Current")).unwrap();
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let original = entries(&path);
    let descriptor: schema::StateDescriptor = project.store.read_json(&project.current_id).unwrap();
    for target in [
        project.current_id.clone(),
        descriptor.document.clone(),
        descriptor.metadata.clone(),
        descriptor.yjs.object.clone(),
        descriptor.model.source.clone(),
        descriptor.resources.clone(),
    ] {
        let mut files = original.clone();
        files.insert(target.path(), b"corrupted".to_vec());
        write_zip(&path, files);
        assert!(Project::read(&path).is_err(), "{}", target.as_str());
    }
}
#[test]
fn invalid_index_and_envelope_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.typsto");
    let project = version(
        &Project::new(fixture(DocumentType::Courrier, "Text")).unwrap(),
        "v1",
    );
    project
        .save(&mut DocumentLease::acquire(&path).unwrap())
        .unwrap();
    let original = entries(&path);
    for kind in [
        "duplicate-id",
        "date",
        "unknown-field",
        "nil-project",
        "future-version",
        "invalid-hash",
        "path",
        "missing-root",
    ] {
        let mut files = original.clone();
        let mut index: serde_json::Value = serde_json::from_slice(&files["versions.json"]).unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&files["manifest.json"]).unwrap();
        match kind {
            "duplicate-id" => {
                let row = index["snapshots"][0].clone();
                index["snapshots"].as_array_mut().unwrap().push(row);
            }
            "date" => index["snapshots"][0]["created_at"] = "2026-10-31T12:00:00+02:00".into(),
            "unknown-field" => index["unexpected"] = true.into(),
            "nil-project" => manifest["project_id"] = Uuid::nil().to_string().into(),
            "future-version" => manifest["version"] = 12.into(),
            "invalid-hash" => manifest["current"] = "sha256:../escape".into(),
            "path" => {
                files.insert("objects/sha256/../escape".into(), vec![]);
            }
            "missing-root" => {
                files.remove(&project.current_id.path());
            }
            _ => unreachable!(),
        }
        files.insert("versions.json".into(), serde_json::to_vec(&index).unwrap());
        files.insert(
            "manifest.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        write_zip(&path, files);
        assert!(Project::read(&path).is_err(), "{kind}");
    }
}
