use super::*;

#[test]
fn native_fixture_roundtrips_with_incomplete_records_and_immutable_source() {
    let text = include_str!("../../../../fixtures/bibliography/native.yaml");
    let prepared = import(text, ImportFormat::Hayagriva);
    assert_eq!(prepared.records.len(), 6);
    assert_eq!(prepared.source.contents(), text);
    assert!(
        prepared
            .diagnostics
            .iter()
            .any(|d| d.key == "minimal" && d.code == Code::Incomplete)
    );
    for record in &prepared.records {
        assert_eq!(
            record.provenance()[0].source_digest,
            *prepared.source.digest()
        );
        assert_eq!(record.provenance()[0].original_key, record.key());
    }
    let original = parse_native(text, ImportFormat::Hayagriva).unwrap();
    let library = Library::new(prepared.records).unwrap();
    let restored = parse_native(&library.to_hayagriva().unwrap(), ImportFormat::Hayagriva).unwrap();
    for entry in original {
        assert_eq!(&entry, restored.get(entry.key()).unwrap());
    }
}

#[test]
fn biblatex_conversion_reports_losses_and_keeps_software_intent_and_original() {
    let text = include_str!("../../../../fixtures/bibliography/imports.bib");
    let prepared = import(text, ImportFormat::Biblatex);
    assert_eq!(prepared.records.len(), 6);
    assert_eq!(prepared.source.contents(), text);
    for field in ["subtitle", "titleaddon", "customfield", "date"] {
        assert!(
            prepared.diagnostics.iter().any(|d| d.key == "journal-paper"
                && d.field.as_deref() == Some(field)
                && d.code == Code::ConversionLoss),
            "{field}"
        );
    }
    let software = prepared
        .records
        .iter()
        .find(|r| r.key() == "software")
        .unwrap();
    assert_eq!(software.profile(), Profile::Software);
    assert_eq!(
        software.entry().entry_type(),
        &hayagriva::types::EntryType::Repository
    );
    assert!(
        prepared
            .diagnostics
            .iter()
            .any(|d| d.key == "software" && d.code == Code::TypeMapping)
    );
    let chapter = prepared
        .records
        .iter()
        .find(|r| r.key() == "chapter")
        .unwrap();
    assert_eq!(chapter.profile(), Profile::BookChapter);
    assert!(!chapter.entry().parents().is_empty());
    assert!(prepared.diagnostics.iter().all(|d| !d.detail.is_empty()));
}

#[test]
fn unknown_native_fields_are_reported_instead_of_silently_claiming_lossless_import() {
    let text = "k:\n  type: article\n  custom: retained only in source\n  parent:\n    type: periodical\n    extra: also retained\n";
    let prepared = import(text, ImportFormat::Hayagriva);
    let fields = prepared
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::UnsupportedField)
        .map(|d| d.field.as_deref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(fields, vec!["/custom", "/parent/0/extra"]);
    assert_eq!(prepared.source.contents(), text);
}

#[test]
fn malformed_sources_and_duplicate_allocations_publish_no_partial_import() {
    for text in [
        "k: {title: No type}",
        "k: {type: invented}",
        "k: {type: article, author: {given-name: No surname}}",
        "k: {type: article}\nk: {type: book}",
    ] {
        assert!(
            parse_native(text, ImportFormat::Hayagriva).is_err(),
            "{text}"
        );
    }
    assert!(
        parse_native(
            "@article{k,title={a}}\n@article{k,title={b}}",
            ImportFormat::Biblatex
        )
        .is_err()
    );
    assert!(parse_native("@article{broken", ImportFormat::Biblatex).is_err());
    let source = ImportSource::new(
        id("s"),
        "original.yaml",
        ImportFormat::Hayagriva,
        "a: {type: article}\nb: {type: book}".into(),
    )
    .unwrap();
    assert_eq!(
        prepare_import(source, |_| id("same")).unwrap_err().code,
        Code::DuplicateIdentity
    );
    assert!(
        ImportSource::new(id("s"), "../escape", ImportFormat::Hayagriva, String::new()).is_err()
    );
    assert_eq!(
        parse_native(&"x".repeat(MAX_SOURCE_BYTES + 1), ImportFormat::Hayagriva)
            .unwrap_err()
            .code,
        Code::LimitExceeded
    );
}

#[test]
fn immutable_source_transport_refuses_changed_bytes_or_digest() {
    let prepared = import(
        "key: {type: book, title: A book}\r\n",
        ImportFormat::Hayagriva,
    );
    let encoded = serde_json::to_value(&prepared.source).unwrap();
    let restored: ImportSource = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(restored, prepared.source);
    let mut changed = encoded.clone();
    changed["contents"] = json!("key: {type: book, title: Changed}");
    assert!(serde_json::from_value::<ImportSource>(changed).is_err());
    let mut changed = encoded;
    changed["digest"] = json!("0".repeat(64));
    assert!(serde_json::from_value::<ImportSource>(changed).is_err());
}
