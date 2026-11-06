use super::*;
fn classify(record: &Record, profile: Profile) -> Record {
    record.reclassified(&record.fingerprint(), profile).unwrap()
}
fn journal() -> Record {
    record(
        "work",
        "fixed-key",
        json!({"type":"article","title":{"value":"An API study","short":"API"},"serial-number":{"isbn":"9780000000000","custom":"kept"},"note":"Keep this","parent":[{"type":"periodical","title":{"value":"Journal title","short":"JT"},"author":[{"name":"Doe","given-name":"Jane","alias":"JD"}],"date":2026,"publisher":{"name":"Press","location":"Paris"}},{"type":"repository","title":"Deposit"}]}),
    )
}
#[test]
fn profiles_are_derived_and_never_serialized_as_an_independent_classification() {
    for descriptor in profiles() {
        let record = Record::new(id("r"), descriptor.profile.empty_entry("k")).unwrap();
        assert_eq!(record.profile(), descriptor.profile);
        let json = serde_json::to_value(&record).unwrap();
        assert!(json.get("profile").is_none());
        assert!(json.get("genre").is_none());
        assert_eq!(serde_json::from_value::<Record>(json).unwrap(), record);
    }
    for field in ["genre", "profile"] {
        let mut value = serde_json::to_value(journal()).unwrap();
        value[field] = json!("book");
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
}
#[test]
fn type_roundtrips_preserve_native_fields_and_restore_the_right_publication() {
    let original = journal();
    let book = classify(&original, Profile::Book);
    assert_eq!(book.profile(), Profile::Book);
    assert!(book.entry().parents().is_empty());
    assert_eq!(book.retained_parents().len(), 2);
    assert_eq!(book.entry().date().unwrap().year, 2026);
    assert_eq!(
        book.entry().authors().unwrap()[0].alias.as_deref(),
        Some("JD")
    );
    let conference = classify(&book, Profile::ConferencePaper);
    assert_eq!(conference.profile(), Profile::ConferencePaper);
    assert!(
        conference.entry().parents()[0].title().is_none(),
        "Never relabel a journal title as proceedings"
    );
    let conference = conference
        .edited(
            &conference.fingerprint(),
            &[FieldPatch {
                path: "/parent/0/title".into(),
                value: Some(json!("Proceedings title")),
            }],
        )
        .unwrap();
    let restored = classify(&conference, Profile::JournalArticle);
    assert_eq!(restored.entry().parents(), &original.entry().parents()[..1]);
    for field in ["title", "serial-number", "note"] {
        assert_eq!(
            restored.native_value()[field],
            original.native_value()[field]
        );
    }
    let conference = classify(&restored, Profile::ConferencePaper);
    assert_eq!(
        conference.entry().parents()[0]
            .title()
            .unwrap()
            .value
            .to_string(),
        "Proceedings title"
    );
    assert_eq!(conference.id(), original.id());
    assert_eq!(conference.key(), original.key());
    let renamed = conference
        .renamed(&conference.fingerprint(), "new-key")
        .unwrap();
    assert_eq!(
        serde_json::from_value::<Record>(serde_json::to_value(&renamed).unwrap()).unwrap(),
        renamed
    );
    let active = Library::new(vec![book]).unwrap().to_hayagriva().unwrap();
    assert!(!active.contains("Journal title"));
}
#[test]
fn selecting_an_already_matching_profile_preserves_imported_variants_and_multiple_parents() {
    for value in [
        json!({"type":"anthos","title":"Contribution","parent":{"type":"anthology","title":"Collected work"}}),
        json!({"type":"patent","title":"An invention","parent":{"type":"web","title":"Office"}}),
        journal().native_value(),
    ] {
        let before = record("r", "k", value);
        assert_eq!(classify(&before, before.profile()), before);
    }
}
#[test]
fn form_fields_hide_and_restore_values_without_deleting_them() {
    let book = record(
        "r",
        "k",
        json!({"type":"book","title":"A work","edition":"Second","publisher":{"name":{"value":"Press","short":"P"},"location":"Paris"},"genre":"A native descriptive field"}),
    );
    let article = classify(&book, Profile::JournalArticle);
    assert!(
        !crate::form::form_fields(&article)
            .iter()
            .any(|f| f.path == "/edition")
    );
    let restored = classify(&article, Profile::Book);
    assert_eq!(
        restored.native_value()["edition"],
        book.native_value()["edition"]
    );
    assert_eq!(
        restored.native_value()["publisher"],
        book.native_value()["publisher"]
    );
    assert_eq!(
        restored.native_value()["genre"],
        "A native descriptive field"
    );
    assert!(
        book.reclassified(&article.fingerprint(), Profile::Thesis)
            .is_err()
    );
}
#[test]
fn the_native_csl_processor_observes_the_changed_classification() {
    use hayagriva::{BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest};
    let style=hayagriva::citationberg::IndependentStyle::from_xml(r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text"><info><title>Native type test</title><id>https://example.org/native-type-test</id></info><citation><layout><text variable="title"/></layout></citation><bibliography><layout><choose><if type="article-journal"><text value="JOURNAL"/></if><else-if type="paper-conference"><text value="CONFERENCE"/></else-if><else-if type="book"><text value="BOOK"/></else-if><else><text value="OTHER"/></else></choose></layout></bibliography></style>"#).unwrap();
    let record = journal();
    for (record, expected) in [
        (record.clone(), "JOURNAL"),
        (classify(&record, Profile::Book), "BOOK"),
        (classify(&record, Profile::ConferencePaper), "CONFERENCE"),
    ] {
        let mut driver = BibliographyDriver::new();
        driver.citation(CitationRequest::new(
            vec![CitationItem::new(record.entry(), None, None, false, None)],
            &style,
            None,
            &[],
            Some(1),
        ));
        let rendered = driver.finish(BibliographyRequest::new(&style, None, &[]));
        let mut text = String::new();
        rendered.bibliography.unwrap().items[0]
            .content
            .write_buf(&mut text, hayagriva::BufWriteFormat::Plain)
            .unwrap();
        assert_eq!(text, expected);
    }
}

#[test]
fn enrichment_never_turns_an_existing_book_into_a_periodical_by_copying_incompatible_parents() {
    let book = record(
        "book",
        "book",
        json!({"type":"book","title":"Same work","serial-number":{"doi":"10.1000/example"}}),
    );
    let article = record(
        "article",
        "article",
        json!({"type":"article","title":"Same work","serial-number":{"doi":"10.1000/example"},"parent":{"type":"periodical","title":"Journal"}}),
    );
    let library = Library::new(vec![book.clone()]).unwrap();
    let plan = ImportPlan::prepare(&library, &[article]).unwrap();
    assert!(!plan.blocked());
    let after = plan.apply(&library).unwrap();
    let after = after.get(book.id()).unwrap();
    assert_eq!(after.entry().entry_type(), book.entry().entry_type());
    assert!(after.entry().parents().is_empty());
    assert_eq!(
        after.retained_parents()[0]
            .title()
            .unwrap()
            .value
            .to_string(),
        "Journal"
    );
    assert_eq!(
        classify(after, Profile::JournalArticle).entry().parents()[0]
            .title()
            .unwrap()
            .value
            .to_string(),
        "Journal"
    );
}
