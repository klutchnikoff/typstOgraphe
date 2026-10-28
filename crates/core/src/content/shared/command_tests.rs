use super::*;
use crate::content::{
    commands::Command,
    contract::{Settings, TargetReason, UnavailableReason},
};
use crate::document_types::{
    DocumentType, Metadata,
    article::{Affiliation, Author},
};
use serde_json::{Value, json};
use yrs::{XmlFragment, types::SharedRef};

pub(super) fn session(kind: DocumentType) -> SharedSession {
    let mut metadata = Metadata::blank(kind);
    if let Metadata::Article(meta) = &mut metadata {
        meta.authors.push(Author {
            id: "camille".into(),
            name: "Camille".into(),
            affiliation_ids: vec!["irmar".into()],
            ..Default::default()
        });
        meta.affiliations.push(Affiliation {
            id: "irmar".into(),
            text: "Univ Rennes, CNRS, IRMAR".into(),
        });
    }
    let state = State::new(serde_json::from_value(json!({"type":"doc","content":[
        {"type":"paragraph","content":[{"type":"text","text":"A😀é日本"}]},
        {"type":"table","content":[{"type":"tableRow","content":[{"type":"tableCell","content":[{"type":"paragraph","content":[{"type":"text","text":"Cellule"}]}]}]}]},
        {"type":"paragraph","content":[{"type":"text","text":"Avant"},{"type":"footnote","attrs":{"text":"Note 😀"}},{"type":"text","text":" après"}]},
        {"type":"paragraph"}
    ]})).unwrap(), Settings { metadata, template:kind.default_model(), model_id:kind.models().next().unwrap().model.id().into() });
    SharedSession::open(
        Guard {
            document: "project".into(),
            revision: 0,
        },
        &SharedDocument::from_state(&state).unwrap().update(),
        Assets::default(),
    )
    .unwrap()
}
pub(super) fn command(id: &str, params: Value) -> Command {
    serde_json::from_value(json!({"id":id,"params":params})).unwrap()
}
pub(super) fn target(zone: Value, from: u32, to: u32) -> Value {
    json!({"zone":zone,"from":from,"to":to})
}
pub(super) fn body(path: Value, from: u32, to: u32) -> Value {
    target(json!({"kind":"body","path":path}), from, to)
}
pub(super) fn execute(session: &mut SharedSession, commands: Vec<Command>) -> Publication {
    let prepared = session
        .prepare_commands(session.guard(), &commands)
        .unwrap();
    session.publish(prepared).unwrap()
}
pub(super) fn fields(session: &SharedSession) -> Value {
    serde_json::to_value(&session.state().settings.metadata).unwrap()["fields"].clone()
}

#[test]
fn text_and_styles_preserve_xml_identity_across_cells_annotations_and_unicode() {
    let mut s = session(DocumentType::Courrier);
    let before = SharedDocument::from_update(&s.update()).unwrap();
    let root = before.doc.transact().get_xml_fragment("default").unwrap();
    let p = root.get(&before.doc.transact(), 0).unwrap();
    let cell = targets::element(root.clone(), &before.doc.transact(), &[1, 0, 0, 0]).unwrap();
    execute(
        &mut s,
        vec![
            command(
                "text.bold",
                json!({"target":body(json!([0]),1,3),"value":"on"}),
            ),
            command(
                "text.insert",
                json!({"target":body(json!([1,0,0,0]),0,7),"text":"Nouvelle cellule"}),
            ),
            command(
                "text.insert",
                json!({"target":target(json!({"kind":"annotation","path":[2,1]}),5,7),"text":"日本"}),
            ),
        ],
    );
    assert_eq!(s.guard().revision, 1);
    assert_eq!(
        s.state.document.content[0].content[1].text.as_deref(),
        Some("😀")
    );
    assert_eq!(s.state.document.content[0].content[1].marks[0].kind, "bold");
    let after = SharedDocument::from_update(&s.update()).unwrap();
    let tx = after.doc.transact();
    let root = tx.get_xml_fragment("default").unwrap();
    assert_eq!(root.get(&tx, 0).unwrap().id(), p.id());
    assert_eq!(
        targets::element(root.clone(), &tx, &[1, 0, 0, 0])
            .unwrap()
            .hook()
            .id(),
        cell.hook().id()
    );
    assert_eq!(
        s.state.document.content[2].content[1]
            .attrs
            .as_ref()
            .unwrap()["text"],
        "Note 日本"
    );
}

#[test]
fn metadata_text_and_rich_zones_share_commands_without_replacing_records_or_collections() {
    let mut s = session(DocumentType::Article);
    let bytes = s.update();
    let before = SharedDocument::from_update(&bytes).unwrap();
    let (record, _) = targets::record(
        &before.doc,
        &before.doc.transact(),
        s.state(),
        "authors",
        "camille",
        "name",
    )
    .unwrap();
    execute(
        &mut s,
        vec![
            command(
                "metadata.set",
                json!({"target":{"kind":"record","collection":"authors","id":"camille","field":"email"},"value":"camille@example.org"}),
            ),
            command(
                "text.insert",
                json!({"target":target(json!({"kind":"record","collection":"authors","id":"camille","field":"name"}),7,7),"text":" Martin"}),
            ),
            command(
                "text.insert",
                json!({"target":target(json!({"kind":"record","collection":"localizations","id":"localization-fr","field":"abstract","path":[0]}),0,0),"text":"Un résumé 😀","marks":[{"type":"italic"}]}),
            ),
            command(
                "metadata.set",
                json!({"target":{"kind":"metadata","field":"article.show_outline"},"value":true}),
            ),
        ],
    );
    let f = fields(&s);
    assert_eq!(f["authors"][0]["name"], "Camille Martin");
    assert_eq!(f["show_outline"], true);
    assert_eq!(
        f["localizations"][0]["abstract"]["content"][0]["content"][0]["marks"][0]["type"],
        "italic"
    );
    let after = SharedDocument::from_update(&s.update()).unwrap();
    assert_eq!(
        targets::record(
            &after.doc,
            &after.doc.transact(),
            s.state(),
            "authors",
            "camille",
            "name"
        )
        .unwrap()
        .0
        .hook()
        .id(),
        record.hook().id()
    );
    let mut letter = session(DocumentType::Courrier);
    execute(
        &mut letter,
        vec![
            command(
                "metadata.set",
                json!({"target":{"kind":"metadata","field":"letter.subject"},"value":"Objet"}),
            ),
            command(
                "text.insert",
                json!({"target":target(json!({"kind":"metadata","field":"letter.subject"}),5,5),"text":" précis"}),
            ),
        ],
    );
    assert_eq!(fields(&letter)["objet"], "Objet précis");
}

#[test]
fn target_errors_are_typed_and_availability_is_the_same_nonmutating_validation() {
    let s = session(DocumentType::Courrier);
    let original = s.update();
    for (t, reason) in [
        (body(json!([0]), 2, 2), TargetReason::SplitSurrogate),
        (body(json!([0]), 9, 10), TargetReason::OutOfBounds),
        (body(json!([0]), 3, 1), TargetReason::ReversedRange),
        (body(json!([1]), 0, 0), TargetReason::NotTextZone),
        (body(json!([9]), 0, 0), TargetReason::MissingNode),
    ] {
        let call = command("text.insert", json!({"target":t,"text":"x"}));
        assert_eq!(
            s.availability(s.guard(), &call).reason,
            Some(Error::InvalidTarget {
                reason: reason.clone()
            })
        );
        assert!(
            matches!(s.prepare_commands(s.guard(),&[call]),Err(Error::BatchFailed {index:0,cause}) if *cause==Error::InvalidTarget {reason})
        );
        assert_eq!(s.update(), original);
    }
    let call = command(
        "text.bold",
        json!({"target":target(json!({"kind":"annotation","path":[2,1]}),0,2)}),
    );
    assert_eq!(
        s.availability(s.guard(), &call).reason,
        Some(Error::CommandUnavailable {
            command: "text.bold".into(),
            reason: UnavailableReason::PlainTextOnly
        })
    );
}

#[test]
fn invalid_late_steps_and_obsolete_publications_never_publish_a_partial_batch() {
    let mut s = session(DocumentType::Article);
    let original = s.update();
    let calls = vec![
        command("text.delete", json!({"target":body(json!([0]),0,1)})),
        command(
            "metadata.set",
            json!({"target":{"kind":"record","collection":"authors","id":"missing","field":"name"},"value":"X"}),
        ),
    ];
    assert!(matches!(
        s.prepare_commands(s.guard(), &calls),
        Err(Error::BatchFailed { index: 1, .. })
    ));
    assert_eq!(s.update(), original);
    let calls = vec![command(
        "text.insert",
        json!({"target":body(json!([0]),0,0),"text":"X"}),
    )];
    let a = s.prepare_commands(s.guard(), &calls).unwrap();
    let b = s.prepare_commands(s.guard(), &calls).unwrap();
    s.publish(a).unwrap();
    let confirmed = s.update();
    assert!(matches!(s.publish(b), Err(Error::StaleRevision { .. })));
    assert_eq!(s.update(), confirmed);
    let wrong = Guard {
        document: "another".into(),
        revision: s.guard().revision,
    };
    assert!(matches!(
        s.prepare_commands(&wrong, &calls),
        Err(Error::WrongDocument)
    ));
}

#[test]
fn final_validation_can_repair_an_intermediate_reference_but_cannot_leave_it_broken() {
    let mut s = session(DocumentType::Article);
    let original = s.update();
    let broken = command(
        "metadata.set",
        json!({"target":{"kind":"record","collection":"authors","id":"camille","field":"affiliation-ids"},"value":["missing"]}),
    );
    assert!(
        s.prepare_commands(s.guard(), std::slice::from_ref(&broken))
            .is_err()
    );
    assert_eq!(s.update(), original);
    let repaired = command(
        "metadata.set",
        json!({"target":{"kind":"record","collection":"authors","id":"camille","field":"affiliation-ids"},"value":["irmar"]}),
    );
    execute(&mut s, vec![broken, repaired]);
    assert_eq!(
        fields(&s)["authors"][0]["affiliation-ids"],
        json!(["irmar"])
    );
}

#[test]
fn style_changes_cover_atoms_and_remove_marks_without_changing_the_atom_identity() {
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![
            command("text.bold", json!({"target":body(json!([2]),0,12)})),
            command("text.italic", json!({"target":body(json!([2]),5,6)})),
        ],
    );
    assert_eq!(s.state.document.content[2].content[1].marks.len(), 2);
    execute(
        &mut s,
        vec![command(
            "text.clear",
            json!({"target":body(json!([2]),5,6)}),
        )],
    );
    assert!(s.state.document.content[2].content[1].marks.is_empty());
    assert!(!s.state.document.content[2].content[0].marks.is_empty());
}

#[test]
fn noops_do_not_increment_revision_and_plain_insertion_does_not_inherit_marks() {
    let mut s = session(DocumentType::Courrier);
    let out = execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([0]),0,1),"text":"A"}),
        )],
    );
    assert!(!out.changed);
    assert!(out.update.is_empty());
    assert_eq!(s.guard().revision, 0);
    execute(
        &mut s,
        vec![command("text.bold", json!({"target":body(json!([0]),0,1)}))],
    );
    let guard = s.guard().clone();
    let out = execute(
        &mut s,
        vec![command("text.bold", json!({"target":body(json!([0]),0,1)}))],
    );
    assert!(!out.changed);
    assert_eq!(s.guard(), &guard);
    execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([0]),1,1),"text":"x"}),
        )],
    );
    let nodes = &s.state.document.content[0].content;
    assert_eq!(nodes[1].text.as_deref(), Some("x😀é日本"));
    assert!(nodes[1].marks.is_empty());
}

#[test]
fn unsupported_operations_and_unknown_parameters_are_explicit_not_silently_reinterpreted() {
    let s = session(DocumentType::Article);
    for (call, reason) in [
        (
            command(
                "text.insert",
                json!({"target":body(json!([0]),0,0),"text":"one\n\ntwo"}),
            ),
            UnavailableReason::StructuralEditRequired,
        ),
        (
            command("text.bold", json!({"target":body(json!([0]),0,0)})),
            UnavailableReason::EmptySelection,
        ),
        (
            command(
                "metadata.set",
                json!({"target":{"kind":"metadata","field":"article.authors"},"value":[]}),
            ),
            UnavailableReason::UnsupportedFieldKind,
        ),
    ] {
        assert!(
            matches!(s.availability(s.guard(),&call).reason,Some(Error::CommandUnavailable {reason:actual,..}) if actual==reason)
        );
    }
    assert!(serde_json::from_value::<Command>(json!({"id":"text.insert","params":{"target":body(json!([0]),0,0),"text":"x","unexpected":true}})).is_err());
    assert!(serde_json::from_value::<Command>(json!({"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":-1,"to":0},"text":"x"}})).is_err());
}

#[test]
fn replacement_spans_separate_xml_texts_and_atoms_without_deleting_surrounding_blocks() {
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([2]),3,8),"text":"X"}),
        )],
    );
    assert_eq!(
        s.inspect_zone(
            s.guard(),
            &crate::content::commands::TextZone::Body { path: vec![2] }
        )
        .unwrap()
        .text,
        "AvaXprès"
    );
    assert_eq!(s.state.document.content.len(), 4);
    execute(
        &mut s,
        vec![command(
            "text.delete",
            json!({"target":body(json!([2]),0,8)}),
        )],
    );
    assert!(
        s.inspect_zone(
            s.guard(),
            &crate::content::commands::TextZone::Body { path: vec![2] }
        )
        .unwrap()
        .text
        .is_empty()
    );
    assert_eq!(s.state.document.content[1].kind, "table");
}

#[test]
fn zone_queries_and_field_errors_are_guarded_and_cannot_address_private_or_identity_fields() {
    let s = session(DocumentType::Article);
    for (zone, reason) in [
        (
            json!({"kind":"record","collection":"__private","id":"a","field":"name"}),
            TargetReason::MissingCollection,
        ),
        (
            json!({"kind":"record","collection":"authors","id":"missing","field":"name"}),
            TargetReason::MissingRecord,
        ),
        (
            json!({"kind":"record","collection":"authors","id":"camille","field":"id"}),
            TargetReason::MissingField,
        ),
        (
            json!({"kind":"record","collection":"authors","id":"camille","field":"name","path":[0]}),
            TargetReason::UnexpectedPath,
        ),
        (
            json!({"kind":"metadata","field":"sender.name"}),
            TargetReason::MissingField,
        ),
    ] {
        let zone = serde_json::from_value(zone).unwrap();
        assert_eq!(
            s.inspect_zone(s.guard(), &zone),
            Err(Error::InvalidTarget { reason })
        );
    }
    let zone = crate::content::commands::TextZone::Body { path: vec![0] };
    let info = s.inspect_zone(s.guard(), &zone).unwrap();
    assert_eq!(info.length, 6);
    assert_eq!(info.text, "A😀é日本");
    assert!(info.supports_styles);
    let mut stale = s.guard().clone();
    stale.revision += 1;
    assert!(matches!(
        s.inspect_zone(&stale, &zone),
        Err(Error::StaleRevision { .. })
    ));
    for value in [json!(null), json!(["name"]), json!(true)] {
        let call = command(
            "metadata.set",
            json!({"target":{"kind":"record","collection":"authors","id":"camille","field":"name"},"value":value}),
        );
        assert_eq!(
            s.availability(s.guard(), &call).reason,
            Some(Error::InvalidParameters {
                parameter: "value".into()
            })
        );
    }
}

#[test]
fn link_removal_clears_hashed_binding_keys_and_repeated_atom_styles_are_noops() {
    let mut s = session(DocumentType::Courrier);
    let shared = SharedDocument::from_update(&s.update()).unwrap();
    {
        let mut tx = shared.doc.transact_mut();
        let paragraph = tx
            .get_xml_fragment("default")
            .unwrap()
            .get(&tx, 0)
            .unwrap()
            .into_xml_element()
            .unwrap();
        let text = paragraph.get(&tx, 0).unwrap().into_xml_text().unwrap();
        use yrs::Text;
        text.format(
            &mut tx,
            0,
            1,
            yrs::types::Attrs::from([(
                "link--AbCd1234".into(),
                yrs::Any::from_json(r#"{"href":"https://example.org"}"#).unwrap(),
            )]),
        );
    }
    s = SharedSession::open(s.guard().clone(), &shared.update(), Assets::default()).unwrap();
    execute(
        &mut s,
        vec![command(
            "text.remove_link",
            json!({"target":body(json!([0]),0,1)}),
        )],
    );
    assert!(
        s.state.document.content[0]
            .content
            .iter()
            .all(|node| node.marks.is_empty())
    );
    execute(
        &mut s,
        vec![
            command("text.bold", json!({"target":body(json!([2]),5,6)})),
            command("text.italic", json!({"target":body(json!([2]),5,6)})),
        ],
    );
    let outcome = execute(
        &mut s,
        vec![command("text.bold", json!({"target":body(json!([2]),5,6)}))],
    );
    assert!(!outcome.changed);
}

#[test]
fn bad_link_parameters_duplicate_styles_and_oversized_batches_never_mutate_state() {
    let s = session(DocumentType::Courrier);
    let before = s.update();
    for (call, parameter) in [
        (
            command(
                "text.link",
                json!({"target":body(json!([0]),0,1),"url":"javascript:alert(1)"}),
            ),
            "url",
        ),
        (
            command(
                "text.insert",
                json!({"target":body(json!([0]),0,0),"text":"x","marks":[{"type":"bold"},{"type":"bold"}]}),
            ),
            "marks",
        ),
    ] {
        assert_eq!(
            s.availability(s.guard(), &call).reason,
            Some(Error::InvalidParameters {
                parameter: parameter.into()
            })
        );
    }
    let call = command("text.delete", json!({"target":body(json!([0]),0,0)}));
    assert!(matches!(
        s.prepare_commands(s.guard(), &vec![call; 257]),
        Err(Error::StateLimitExceeded)
    ));
    assert_eq!(s.update(), before);
}
