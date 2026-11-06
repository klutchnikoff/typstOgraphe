use super::*;

#[test]
fn literal_formatted_text_survives_json_patches_and_direct_hayagriva_reopening() {
    for title in [
        json!({"value":"aCRonym stays lower","verbatim":true}),
        json!({"value":"aCRonym stays lower","short":"aCRonym","verbatim":true}),
        json!({"value":{"value":"aCRonym stays lower","verbatim":true},"short":"aCRonym"}),
    ] {
        let before = Record::from_native(
            id("r"),
            "literal",
            json!({"type":"book","title":title,
            "publisher":{"name":{"value":"eXample press","verbatim":true},"location":"Paris"},
            "parent":{"type":"anthology","title":{"value":"another LOWER title","verbatim":true}}}),
        )
        .unwrap();
        assert_eq!(
            before
                .entry()
                .title()
                .unwrap()
                .format_title_case(Default::default()),
            "aCRonym stays lower"
        );
        let after = before
            .edited(
                &before.fingerprint(),
                &[FieldPatch {
                    path: "/note".into(),
                    value: Some(json!("Changed field")),
                }],
            )
            .unwrap();
        assert_eq!(before.entry().title(), after.entry().title());
        assert_eq!(before.entry().publisher(), after.entry().publisher());
        assert_eq!(before.entry().parents(), after.entry().parents());
        let restored: Record =
            serde_json::from_slice(&serde_json::to_vec(&after).unwrap()).unwrap();
        assert_eq!(after, restored);
        let library = Library::new(vec![after.clone()]).unwrap();
        let yaml = library.to_hayagriva().unwrap();
        // Read with Hayagriva itself, without our normalizer, as Typst does.
        let reopened = hayagriva::io::from_yaml_str(&yaml).unwrap();
        assert_eq!(reopened.get("literal").unwrap(), after.entry());
    }
}

#[test]
fn yaml_import_preserves_literal_casing_while_the_original_remains_identical() {
    let text = "k:\n  type: book\n  title:\n    value: aCRonym stays lower\n    verbatim: true\n";
    let prepared = import(text, ImportFormat::Hayagriva);
    assert_eq!(prepared.source.contents(), text);
    assert_eq!(
        prepared.records[0]
            .entry()
            .title()
            .unwrap()
            .format_title_case(Default::default()),
        "aCRonym stays lower"
    );
    assert_eq!(
        prepared.fingerprint(),
        import(text, ImportFormat::Hayagriva).fingerprint()
    );
}

#[test]
fn fingerprints_and_identities_refuse_values_outside_their_public_schema() {
    for invalid in ["", "ABC", &"0".repeat(63), &"A".repeat(64), &"g".repeat(64)] {
        assert!(serde_json::from_value::<Fingerprint>(json!(invalid)).is_err());
    }
    for invalid in ["", "has spaces", "../file", &"x".repeat(129)] {
        assert!(serde_json::from_value::<RecordId>(json!(invalid)).is_err());
    }
    assert!(serde_json::from_value::<Fingerprint>(json!("a".repeat(64))).is_ok());
}

#[test]
fn unsupported_nested_members_are_refused_or_reported_without_losing_the_original() {
    for (field, value, path) in [
        (
            "title",
            json!({"value":"Title","unexpected":true}),
            "/title/unexpected",
        ),
        (
            "author",
            json!([{"name":"Martin","unexpected":true}]),
            "/author/0/unexpected",
        ),
        (
            "publisher",
            json!({"name":"Press","unexpected":true}),
            "/publisher/unexpected",
        ),
        (
            "parent",
            json!({"type":"periodical","unexpected":true}),
            "/parent/0/unexpected",
        ),
    ] {
        let mut value_with_unknown = json!({"type":"article","title":"Title"});
        value_with_unknown[field] = value;
        let error = Record::from_native(id("r"), "k", value_with_unknown.clone()).unwrap_err();
        assert_eq!(error.code, Code::UnsupportedField);
        assert_eq!(error.field.as_deref(), Some(path));
        let original = yaml_serde::to_string(&json!({"k":value_with_unknown})).unwrap();
        let prepared = import(&original, ImportFormat::Hayagriva);
        assert_eq!(prepared.source.contents(), original);
        assert!(
            prepared
                .diagnostics
                .iter()
                .any(|d| d.code == Code::UnsupportedField && d.field.as_deref() == Some(path))
        );
    }
}
