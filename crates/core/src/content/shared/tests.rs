use super::*;
use crate::{
    content::contract::Settings,
    document_types::{DocumentType, Metadata},
};
use serde_json::json;
use yrs::{Map, Text, XmlFragment};

fn state(kind: DocumentType) -> State {
    let metadata = Metadata::blank(kind);
    let template = kind.default_model();
    let model_id = kind
        .models()
        .next()
        .map(|m| m.model.id().to_owned())
        .unwrap_or_else(|| "embedded".into());
    let document = serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"A😀é日本"}]},{"type":"paragraph"}]})).unwrap();
    State::new(
        document,
        Settings {
            metadata,
            template,
            model_id,
        },
    )
}
fn guard() -> Guard {
    Guard {
        document: "test-document".into(),
        revision: 0,
    }
}
fn open() -> SharedSession {
    SharedSession::open(
        guard(),
        &SharedDocument::from_state(&state(DocumentType::Courrier))
            .unwrap()
            .update(),
        Assets::default(),
    )
    .unwrap()
}
fn peer(session: &SharedSession) -> SharedDocument {
    SharedDocument::from_update(&session.update()).unwrap()
}
fn text(doc: &SharedDocument) -> yrs::XmlTextRef {
    let tx = doc.doc.transact();
    let body = tx.get_xml_fragment("default").unwrap();
    let paragraph = body.get(&tx, 0).unwrap().into_xml_element().unwrap();
    paragraph.get(&tx, 0).unwrap().into_xml_text().unwrap()
}
fn insert(doc: &SharedDocument, value: &str) -> Vec<u8> {
    let before = doc.doc.transact().state_vector();
    text(doc).insert(&mut doc.doc.transact_mut(), 3, value);
    doc.doc.transact().encode_diff_v1(&before)
}

#[test]
fn all_document_types_bootstrap_and_roundtrip_without_an_interface() {
    for kind in DocumentType::ALL {
        let expected = state(kind);
        expected.validate(&Assets::default()).unwrap();
        let bytes = verify_or_initialize(&expected, &[]).unwrap();
        let shared = SharedDocument::from_update(&bytes).unwrap();
        shared.verify(&expected).unwrap();
        shared
            .state()
            .unwrap()
            .validate(&Assets::default())
            .unwrap();
    }
    fn send_sync<T: Send + Sync>() {}
    send_sync::<SharedSession>();
}

#[test]
fn publication_is_guarded_atomic_and_idempotent_with_utf16_positions() {
    let mut session = open();
    let a = peer(&session);
    let b = peer(&session);
    let update_a = insert(&a, " Premier ");
    let update_b = insert(&b, " Second ");
    let pa = session
        .prepare(&guard(), std::slice::from_ref(&update_a))
        .unwrap();
    let pb = session.prepare(&guard(), &[update_b]).unwrap();
    let original = session.update();
    assert_eq!(session.guard().revision, 0);
    let outcome = session.publish(pa).unwrap();
    assert!(outcome.changed);
    assert_eq!(outcome.guard.revision, 1);
    assert_eq!(
        outcome.state.document.content[0].content[0].text.as_deref(),
        Some("A😀 Premier é日本")
    );
    let confirmed = session.update();
    assert_ne!(confirmed, original);
    assert!(matches!(
        session.publish(pb),
        Err(Error::StaleRevision {
            expected: 0,
            actual: 1
        })
    ));
    assert_eq!(session.update(), confirmed);
    let repeated = session.prepare(session.guard(), &[update_a]).unwrap();
    let unchanged = session.publish(repeated).unwrap();
    assert!(!unchanged.changed);
    assert!(unchanged.update.is_empty());
    assert_eq!(unchanged.guard.revision, 1);
    let mut foreign = session.guard().clone();
    foreign.document = "another-document".into();
    assert!(matches!(
        session.prepare(&foreign, &[]),
        Err(Error::WrongDocument)
    ));
}

#[test]
fn invalid_or_incomplete_updates_leave_the_confirmed_state_untouched() {
    let session = open();
    let before = session.update();
    assert!(session.prepare(&guard(), &[vec![255]]).is_err());
    let remote = peer(&session);
    insert(&remote, " dependency ");
    let dependent = insert(&remote, " later ");
    assert!(matches!(
        SharedDocument::from_update(&dependent),
        Err(Error::IncompleteSharedState)
    ));
    assert!(matches!(
        session.prepare(&guard(), &[dependent]),
        Err(Error::IncompleteSharedState)
    ));
    assert_eq!(session.update(), before);
    let root = remote.doc.get_or_insert_xml_fragment("default");
    root.insert(
        &mut remote.doc.transact_mut(),
        0,
        yrs::XmlElementPrelim::empty("script"),
    );
    assert!(matches!(
        session.prepare(&guard(), &[remote.update()]),
        Err(Error::InvalidDocument { .. })
    ));
    assert_eq!(session.update(), before);
}

#[test]
fn a_batch_validates_its_final_state_and_publishes_one_revision() {
    let mut session = open();
    let remote = peer(&session);
    let body = remote.doc.get_or_insert_xml_fragment("default");
    let before = remote.doc.transact().state_vector();
    body.insert(
        &mut remote.doc.transact_mut(),
        0,
        yrs::XmlElementPrelim::empty("script"),
    );
    let invalid = remote.doc.transact().encode_diff_v1(&before);
    assert!(
        session
            .prepare(&guard(), std::slice::from_ref(&invalid))
            .is_err()
    );
    let middle = remote.doc.transact().state_vector();
    body.remove_range(&mut remote.doc.transact_mut(), 0, 1);
    insert(&remote, " validated ");
    let fixed = remote.doc.transact().encode_diff_v1(&middle);
    let prepared = session.prepare(&guard(), &[invalid, fixed]).unwrap();
    let outcome = session.publish(prepared).unwrap();
    assert_eq!(outcome.guard.revision, 1);
    assert!(
        outcome.state.document.content[0].content[0]
            .text
            .as_ref()
            .unwrap()
            .contains("validated")
    );
}

#[test]
fn metadata_and_rich_frontmatter_are_projected_from_shared_types() {
    let expected = state(DocumentType::Article);
    let shared = SharedDocument::from_state(&expected).unwrap();
    let id = expected.settings.metadata.clone();
    let Metadata::Article(article) = id else {
        unreachable!()
    };
    let tx = shared.doc.transact();
    let settings = tx.get_map("settings").unwrap();
    let yrs::Out::YMap(collections) = settings.get(&tx, "collections").unwrap() else {
        unreachable!()
    };
    let yrs::Out::YMap(records) = collections.get(&tx, "localizations").unwrap() else {
        unreachable!()
    };
    let yrs::Out::YMap(record) = records.get(&tx, &article.localizations[0].id).unwrap() else {
        unreachable!()
    };
    let yrs::Out::YXmlFragment(fragment) = record.get(&tx, "abstract").unwrap() else {
        unreachable!()
    };
    let p = fragment.get(&tx, 0).unwrap().into_xml_element().unwrap();
    drop(tx);
    let text = p.insert(
        &mut shared.doc.transact_mut(),
        0,
        yrs::XmlTextPrelim::new("Résumé 😀 にほん"),
    );
    text.format(
        &mut shared.doc.transact_mut(),
        0,
        6,
        [("italic".into(), yrs::Any::Map(Default::default()))]
            .into_iter()
            .collect(),
    );
    settings.insert(&mut shared.doc.transact_mut(), "article.show_outline", true);
    let projected = shared.state().unwrap();
    projected.validate(&Assets::default()).unwrap();
    let value = serde_json::to_value(&projected.settings.metadata).unwrap();
    assert_eq!(value["fields"]["show_outline"], true);
    assert_eq!(
        value["fields"]["localizations"][0]["abstract"]["content"][0]["content"][0]["marks"][0]["type"],
        "italic"
    );
    assert!(
        matches!(shared.verify(&expected), Err(Error::InconsistentState { field }) if field == "settings")
    );
}

#[test]
fn editor_defaults_do_not_hide_changes_to_content_or_significant_attributes() {
    let mut expected = state(DocumentType::Courrier);
    expected.document.content.push(serde_json::from_value(json!({"type":"table","content":[{"type":"tableRow","content":[{"type":"tableCell","attrs":{"colspan":1,"rowspan":1,"colwidth":null,"align":null},"content":[{"type":"paragraph"}]}]}]})).unwrap());
    let shared = SharedDocument::from_state(&expected).unwrap();
    shared.verify(&expected).unwrap();
    expected.document.content[2].content[0].content[0]
        .attrs
        .as_mut()
        .unwrap()["colspan"] = json!(2);
    assert!(
        matches!(shared.verify(&expected), Err(Error::InconsistentState { field }) if field == "document")
    );
}

#[test]
fn hidden_binding_roots_and_relative_positions_survive_publication() {
    use yrs::types::SharedRef;
    let mut session = open();
    let remote = peer(&session);
    let root = remote
        .doc
        .get_or_insert_map("__tiptap_position_migrations:root:default");
    root.insert(&mut remote.doc.transact_mut(), "kept", true);
    let text_hook = text(&remote).hook();
    insert(&remote, " retained ");
    let prepared = session.prepare(&guard(), &[remote.update()]).unwrap();
    session.publish(prepared).unwrap();
    let restored = peer(&session);
    let tx = restored.doc.transact();
    assert!(text_hook.get(&tx).is_some());
    assert_eq!(
        tx.get_map("__tiptap_position_migrations:root:default")
            .unwrap()
            .get(&tx, "kept"),
        Some(yrs::Out::Any(yrs::Any::Bool(true)))
    );
}
