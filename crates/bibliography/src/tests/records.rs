use super::*;

#[test]
fn record_edit_preserves_native_parents_roles_and_formatted_fields() {
    let entry = json!({"type":"article","title":{"value":"{DNA} and $x^2$","short":"{DNA}","verbatim":true},
        "author":[{"name":"Durand","given-name":"Élise"}],
        "affiliated":[{"role":"translator","names":"Martin, Camille"}],
        "parent":{"type":"periodical","title":"Native journal","publisher":{"name":"Press","location":"Paris"}},
        "serial-number":{"doi":"10.5555/example","custom":"kept"}});
    let original = record("a", "readable", entry);
    let changed = original
        .edited(
            &original.fingerprint(),
            &[FieldPatch {
                path: "/note".into(),
                value: Some(json!("A note")),
            }],
        )
        .unwrap();
    assert_eq!(original.entry().title(), changed.entry().title());
    assert_eq!(original.entry().parents(), changed.entry().parents());
    assert_eq!(original.entry().affiliated(), changed.entry().affiliated());
    assert_eq!(
        original.entry().serial_number(),
        changed.entry().serial_number()
    );
    assert!(original.entry().note().is_none());
    assert!(changed.entry().note().is_some());
    let roundtrip: Record = serde_json::from_slice(&serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_eq!(changed, roundtrip);
    let renamed = changed
        .renamed(&changed.fingerprint(), "another-key")
        .unwrap();
    assert_eq!(original.id(), renamed.id());
    assert_eq!(renamed.key(), "another-key");
    assert_eq!(renamed.entry().title(), original.entry().title());
    assert_eq!(
        renamed
            .edited(&original.fingerprint(), &[])
            .unwrap_err()
            .code,
        Code::StaleBase
    );
}

#[test]
fn invalid_edits_are_atomic_and_cannot_target_an_unrelated_nested_object() {
    let original = record(
        "a",
        "k",
        json!({"type":"article","title":{"value":"Title","short":"Short"}}),
    );
    for (path, value) in [
        ("/date", json!("invalid date")),
        ("/author", json!({"given-name":"No surname"})),
        ("/title/date", json!(2025)),
        ("/unknown", json!(true)),
        ("/parent/9/title", json!("Missing")),
        ("/type", json!("book")),
    ] {
        assert!(
            original
                .edited(
                    &original.fingerprint(),
                    &[FieldPatch {
                        path: path.into(),
                        value: Some(value)
                    }]
                )
                .is_err(),
            "{path}"
        );
    }
    assert_eq!(original.entry().title().unwrap().value.to_string(), "Title");
    assert_eq!(original.key(), "k");
    let mut payload = serde_json::to_value(&original).unwrap();
    payload["entry"]["custom"] = json!("Would be lost by a permissive parser");
    assert!(serde_json::from_value::<Record>(payload).is_err());
}

#[test]
fn library_rejects_duplicate_ids_or_keys_and_projects_only_native_data() {
    let one = record("a", "one", json!({"type":"article","title":"Title"}));
    let same_id = record("a", "two", json!({"type":"book"}));
    let same_key = record("b", "one", json!({"type":"report"}));
    assert_eq!(
        Library::new(vec![one.clone(), same_id]).unwrap_err().code,
        Code::DuplicateIdentity
    );
    assert_eq!(
        Library::new(vec![one.clone(), same_key]).unwrap_err().code,
        Code::DuplicateKey
    );
    let library = Library::new(vec![one.clone()]).unwrap();
    let yaml = library.to_hayagriva().unwrap();
    assert_eq!(
        parse_native(&yaml, ImportFormat::Hayagriva)
            .unwrap()
            .get("one")
            .unwrap(),
        one.entry()
    );
    assert!(!yaml.contains("provenance"));
    let restored: Library = serde_json::from_slice(&serde_json::to_vec(&library).unwrap()).unwrap();
    assert_eq!(restored.fingerprint(), library.fingerprint());
    assert_eq!(Library::default().to_hayagriva().unwrap().trim(), "{}");
    assert!(RecordId::new("../../escape").is_err());
    assert!(serde_json::from_value::<RecordId>(json!("")).is_err());
}

#[test]
fn genre_descriptors_are_common_native_paths_not_type_specific_requirements() {
    for descriptor in profiles() {
        let entry = descriptor.profile.empty_entry("new");
        let record = Record::new(id("new"), entry).unwrap();
        assert!(!completeness(&record).is_empty());
        for field in descriptor.fields {
            assert!(field.path.starts_with('/'));
        }
        assert!(Library::new(vec![record]).is_ok());
    }
    let repository = hayagriva::Entry::new("repo", hayagriva::types::EntryType::Repository);
    assert_eq!(Profile::infer(&repository), Profile::Software);
}

#[test]
fn fuzzy_search_is_unicode_aware_and_does_not_mutate_records() {
    let a = record(
        "a",
        "a",
        json!({"type":"article","title":"Une enquête collective","author":"Durand, Élise"}),
    );
    let b = record(
        "b",
        "b",
        json!({"type":"book","title":"日本語の本","author":"田中"}),
    );
    let records = vec![a, b];
    let before = records.clone();
    assert_eq!(search(&records, "enquete DURAN", 80), vec![id("a")]);
    assert_eq!(search(&records, "日本語", 80), vec![id("b")]);
    assert!(search(&records, "absent", 80).is_empty());
    assert!(search(&records, &"x".repeat(513), 80).is_empty());
    assert!(search(&records, "", 0).is_empty());
    assert_eq!(records, before);
}

#[test]
fn key_suggestions_avoid_existing_names_without_renaming_or_inventing_identity() {
    let one = record(
        "a",
        "stable-key",
        json!({"type":"article","title":"Une enquête collective","author":"Durand, Élise","date":2025}),
    );
    let occupied = ["durand_une_2025".into(), "durand_une_2025_1".into()]
        .into_iter()
        .collect();
    assert_eq!(suggest_key(one.entry(), &occupied), "durand_une_2025_2");
    assert_eq!(one.key(), "stable-key");
    assert_eq!(
        suggest_key(&Profile::Other.empty_entry("pending"), &Default::default()),
        "anonymous_untitled_nd"
    );
}
