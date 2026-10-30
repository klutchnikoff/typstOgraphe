use super::command_tests::{body, command, execute, fields, session};
use super::*;
use crate::{content::contract::TargetReason, document_types::DocumentType};
use serde_json::{Value, json};
use yrs::{Map, XmlFragment, types::SharedRef};

fn tree(path: &[usize]) -> Value {
    json!({"root":{"kind":"body"},"path":path})
}
#[test]
fn inline_objects_split_styled_unicode_text_and_preserve_existing_objects() {
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![command("text.bold", json!({"target":body(json!([0]),0,6)}))],
    );
    let before = SharedDocument::from_update(&s.update()).unwrap();
    let root = before.doc.transact().get_xml_fragment("default").unwrap();
    let paragraph = root.get(&before.doc.transact(), 0).unwrap().id();
    let note = yrs::XmlOut::Element(
        targets::element(root.clone(), &before.doc.transact(), &[2, 1]).unwrap(),
    )
    .id();
    execute(
        &mut s,
        vec![
            command(
                "inline.replace",
                json!({"target":body(json!([0]),3,4),"node":{"type":"footnote","attrs":{"text":"Insérée 日本"},"marks":[{"type":"italic"}]}}),
            ),
            command(
                "node.set_attrs",
                json!({"target":tree(&[2,1]),"attrs":{"text":"Note revue 😀"}}),
            ),
        ],
    );
    let content = &s.state.document.content[0].content;
    assert_eq!(content[0].text.as_deref(), Some("A😀"));
    assert_eq!(content[1].kind, "footnote");
    assert_eq!(content[1].marks[0].kind, "italic");
    assert_eq!(content[2].text.as_deref(), Some("日本"));
    assert_eq!(content[2].marks[0].kind, "bold");
    let after = SharedDocument::from_update(&s.update()).unwrap();
    let root = after.doc.transact().get_xml_fragment("default").unwrap();
    assert_eq!(root.get(&after.doc.transact(), 0).unwrap().id(), paragraph);
    assert_eq!(
        yrs::XmlOut::Element(targets::element(root, &after.doc.transact(), &[2, 1]).unwrap()).id(),
        note
    );
    let publication = execute(
        &mut s,
        vec![command(
            "node.set_attrs",
            json!({"target":tree(&[2,1]),"attrs":{"text":"Note revue 😀"}}),
        )],
    );
    assert!(!publication.changed);
}
#[test]
fn a_boundary_splice_repairs_once_and_preserves_unaffected_neighbors() {
    let mut s = session(DocumentType::Courrier);
    let before = SharedDocument::from_update(&s.update()).unwrap();
    let root = before.doc.transact().get_xml_fragment("default").unwrap();
    let neighbor = root.get(&before.doc.transact(), 2).unwrap().id();
    execute(
        &mut s,
        vec![command(
            "block.splice",
            json!({"parent":tree(&[]),"index":0,"delete_count":2,"nodes":[{"type":"quote","content":[paragraph("Citation"),{"type":"quoteAttribution"}]}]}),
        )],
    );
    assert_eq!(s.state.document.content[0].kind, "quote");
    assert_eq!(
        s.state.document.content[1].content[0].text.as_deref(),
        Some("Avant")
    );
    let after = SharedDocument::from_update(&s.update()).unwrap();
    assert_eq!(
        after
            .doc
            .transact()
            .get_xml_fragment("default")
            .unwrap()
            .get(&after.doc.transact(), 1)
            .unwrap()
            .id(),
        neighbor
    );
    execute(
        &mut s,
        vec![command(
            "block.splice",
            json!({"parent":tree(&[]),"index":0,"delete_count":3,"nodes":[]}),
        )],
    );
    assert_eq!(s.state.document.content.len(), 1);
    assert_eq!(s.state.document.content[0].kind, "paragraph");
}
#[test]
fn object_commands_refuse_invalid_boundaries_private_attributes_and_bibliography_atomically() {
    let s = session(DocumentType::Courrier);
    for step in [
        command(
            "inline.replace",
            json!({"target":body(json!([0]),2,2),"node":{"type":"footnote","attrs":{"text":"Note"}}}),
        ),
        command(
            "inline.replace",
            json!({"target":body(json!([0]),1,1),"node":paragraph("Interdit")}),
        ),
        command(
            "node.set_attrs",
            json!({"target":tree(&[2,1]),"attrs":{"__marks":[]}}),
        ),
        command(
            "node.set_attrs",
            json!({"target":tree(&[2,1]),"attrs":{"text":42}}),
        ),
        command(
            "node.set_attrs",
            json!({"target":tree(&[1,0,0]),"attrs":{"colspan":2}}),
        ),
        command(
            "block.splice",
            json!({"parent":tree(&[]),"index":1,"delete_count":99,"nodes":[]}),
        ),
    ] {
        assert_rollback(
            &s,
            vec![
                command(
                    "node.set_attrs",
                    json!({"target":tree(&[2,1]),"attrs":{"text":"Ne pas publier"}}),
                ),
                step,
            ],
        );
    }
    let article = session(DocumentType::Article);
    assert_rollback(
        &article,
        vec![command(
            "inline.replace",
            json!({"target":body(json!([0]),1,1),"node":{"type":"citation","attrs":{"items":[{"file_id":"absent","key":"missing","supplement":""}],"form":"normal"}}}),
        )],
    );
}
fn abstract_tree(s: &SharedSession, path: &[usize]) -> Value {
    json!({"root":{"kind":"record","collection":"localizations","id":fields(s)["localizations"][0]["id"],"field":"abstract"},"path":path})
}
fn paragraph(text: &str) -> Value {
    json!({"type":"paragraph","content":[{"type":"text","text":text}]})
}
fn assert_rollback(s: &SharedSession, steps: Vec<crate::content::commands::Command>) -> Error {
    let before = s.update();
    let err = s
        .prepare_commands(s.guard(), &steps)
        .err()
        .expect("refused batch");
    assert_eq!(s.update(), before);
    assert_eq!(s.guard().revision, 0);
    err
}
#[test]
fn insert_replace_remove_and_convert_keep_neighbors_and_use_current_paths() {
    let mut s = session(DocumentType::Courrier);
    let original = SharedDocument::from_update(&s.update()).unwrap();
    let root = original.doc.transact().get_xml_fragment("default").unwrap();
    let neighbor = root.get(&original.doc.transact(), 1).unwrap().id();
    execute(
        &mut s,
        vec![
            command(
                "block.insert",
                json!({"parent":tree(&[]),"index":1,"node":paragraph("Nouveau")}),
            ),
            command("block.heading_2", json!({"target":tree(&[1])})),
            command(
                "block.replace",
                json!({"target":tree(&[1]),"node":paragraph("Remplacé")}),
            ),
            command("block.remove", json!({"target":tree(&[0])})),
        ],
    );
    assert_eq!(
        s.state.document.content[0].content[0].text.as_deref(),
        Some("Remplacé")
    );
    assert_eq!(s.guard().revision, 1);
    let after = SharedDocument::from_update(&s.update()).unwrap();
    assert_eq!(
        after
            .doc
            .transact()
            .get_xml_fragment("default")
            .unwrap()
            .get(&after.doc.transact(), 1)
            .unwrap()
            .id(),
        neighbor
    );
    let command = command("block.paragraph", json!({"target":tree(&[0])}));
    assert!(!execute(&mut s, vec![command]).changed);
}
#[test]
fn split_keeps_prefix_identity_marks_unicode_and_inline_atoms() {
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![command("text.bold", json!({"target":body(json!([0]),0,6)}))],
    );
    let original = SharedDocument::from_update(&s.update()).unwrap();
    let root = original.doc.transact().get_xml_fragment("default").unwrap();
    let first = root.get(&original.doc.transact(), 0).unwrap().id();
    execute(
        &mut s,
        vec![command(
            "block.split",
            json!({"target":body(json!([0]),3,3)}),
        )],
    );
    assert_eq!(
        s.state.document.content[0].content[0].text.as_deref(),
        Some("A😀")
    );
    assert_eq!(
        s.state.document.content[1].content[0].text.as_deref(),
        Some("é日本")
    );
    assert_eq!(s.state.document.content[1].content[0].marks[0].kind, "bold");
    let after = SharedDocument::from_update(&s.update()).unwrap();
    assert_eq!(
        after
            .doc
            .transact()
            .get_xml_fragment("default")
            .unwrap()
            .get(&after.doc.transact(), 0)
            .unwrap()
            .id(),
        first
    );
    execute(
        &mut s,
        vec![command(
            "block.split",
            json!({"target":body(json!([3]),5,5)}),
        )],
    );
    assert_eq!(s.state.document.content[4].content[0].kind, "footnote");
    assert_eq!(
        s.state.document.content[4].content[0]
            .attrs
            .as_ref()
            .unwrap()["text"],
        "Note 😀"
    );
}
#[test]
fn insertion_at_caret_uses_split_then_boundary_insertion_without_extra_leading_line() {
    for offset in [0, 3, 6] {
        let mut s = session(DocumentType::Courrier);
        let widget =
            json!({"type":"quote","content":[paragraph("Citation"),{"type":"quoteAttribution"}]});
        let mut steps = Vec::new();
        if offset != 0 {
            steps.push(command(
                "block.split",
                json!({"target":body(json!([0]),offset,offset)}),
            ));
        }
        steps.push(command(
            "block.insert",
            json!({"parent":tree(&[]),"index":if offset==0 {0} else {1},"node":widget}),
        ));
        execute(&mut s, steps);
        let index = usize::from(offset != 0);
        assert_eq!(s.state.document.content[index].kind, "quote");
        assert_eq!(s.state.document.content[index + 1].kind, "paragraph");
        if offset == 0 {
            assert_eq!(
                s.state.document.content[1].content[0].text.as_deref(),
                Some("A😀é日本")
            );
        }
    }
}
#[test]
fn cells_and_record_fragments_share_structure_but_keep_their_boundaries() {
    let mut s = session(DocumentType::Article);
    let root = abstract_tree(&s, &[]);
    execute(
        &mut s,
        vec![
            command("block.split", json!({"target":body(json!([1,0,0,0]),3,3)})),
            command(
                "block.insert",
                json!({"parent":root,"index":1,"node":paragraph("Deuxième paragraphe")}),
            ),
        ],
    );
    assert_eq!(
        s.state.document.content[1].content[0].content[0]
            .content
            .len(),
        2
    );
    assert_eq!(
        fields(&s)["localizations"][0]["abstract"]["content"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let before = s.update();
    let denied = command("block.remove", json!({"target":tree(&[1,0,0])}));
    assert!(matches!(
        s.availability(s.guard(), &denied).reason,
        Some(Error::InvalidTarget {
            reason: TargetReason::NotBlockContainer | TargetReason::NotBlock
        })
    ));
    assert_eq!(s.update(), before);
}
#[test]
fn invalid_shapes_links_assets_targets_and_surrogates_never_publish() {
    for step in [
        command(
            "block.insert",
            json!({"parent":tree(&[]),"index":99,"node":paragraph("Texte")}),
        ),
        command(
            "block.insert",
            json!({"parent":tree(&[]),"index":0,"node":{"type":"bulletList"}}),
        ),
        command(
            "block.replace",
            json!({"target":tree(&[0]),"node":{"type":"figure","attrs":{"src":"images/missing.png","alt":"Image"}}}),
        ),
        command(
            "block.replace",
            json!({"target":tree(&[0]),"node":{"type":"paragraph","content":[{"type":"text","text":"Lien","marks":[{"type":"link","attrs":{"href":"javascript:x"}}]}]}}),
        ),
        command("block.split", json!({"target":body(json!([0]),2,2)})),
        command("block.heading_1", json!({"target":tree(&[1])})),
        command(
            "block.insert",
            json!({"parent":tree(&[1,0,0]),"index":0,"node":{"type":"quote","content":[paragraph("X"),{"type":"quoteAttribution"}]}}),
        ),
    ] {
        assert_rollback(&session(DocumentType::Courrier), vec![step]);
    }
}
#[test]
fn removing_last_text_keeps_an_insertion_point_in_body_cell_and_abstract() {
    let mut s = session(DocumentType::Article);
    let abstract_target = abstract_tree(&s, &[0]);
    execute(
        &mut s,
        vec![
            command("block.remove", json!({"target":tree(&[1,0,0,0])})),
            command("block.remove", json!({"target":abstract_target})),
            command("block.remove", json!({"target":tree(&[3])})),
            command("block.remove", json!({"target":tree(&[2])})),
            command("block.remove", json!({"target":tree(&[1])})),
            command("block.remove", json!({"target":tree(&[0])})),
        ],
    );
    assert_eq!(s.state.document.content.len(), 1);
    assert_eq!(s.state.document.content[0].kind, "paragraph");
    assert!(s.state.document.content[0].content.is_empty());
    assert_eq!(
        fields(&s)["localizations"][0]["abstract"]["content"][0]["type"],
        "paragraph"
    );
}
#[test]
fn ordered_records_keep_existing_maps_texts_and_references() {
    let mut s = session(DocumentType::Article);
    let original = SharedDocument::from_update(&s.update()).unwrap();
    let tx = original.doc.transact();
    let (record, _) = targets::record(
        &original.doc,
        &tx,
        &original.state().unwrap(),
        "authors",
        "camille",
        "name",
    )
    .unwrap();
    let original_hook = record.hook();
    let original_id = original_hook.id();
    let Some(yrs::Out::YText(text)) = record.get(&tx, "name") else {
        panic!()
    };
    let text_hook = text.hook();
    let text_id = text_hook.id();
    drop(tx);
    execute(
        &mut s,
        vec![
            command(
                "record.add",
                json!({"collection":"authors","id":"alex","after":"camille","values":{"name":"Alex 日本","affiliation-ids":["irmar"]}}),
            ),
            command(
                "record.move",
                json!({"collection":"authors","id":"camille"}),
            ),
        ],
    );
    assert_eq!(fields(&s)["authors"][0]["id"], "alex");
    assert_eq!(fields(&s)["authors"][1]["id"], "camille");
    let after = SharedDocument::from_update(&s.update()).unwrap();
    let tx = after.doc.transact();
    let (record, _) =
        targets::record(&after.doc, &tx, s.state(), "authors", "camille", "name").unwrap();
    assert_eq!(record.hook().id(), original_id);
    let Some(yrs::Out::YText(text)) = record.get(&tx, "name") else {
        panic!()
    };
    assert_eq!(text.hook().id(), text_id);
    drop(tx);
    assert!(
        !execute(
            &mut s,
            vec![command(
                "record.move",
                json!({"collection":"authors","id":"camille"})
            )]
        )
        .changed
    );
}
#[test]
fn removing_referenced_records_requires_an_explicit_atomic_repair() {
    let mut s = session(DocumentType::Article);
    assert_rollback(
        &s,
        vec![command(
            "record.remove",
            json!({"collection":"affiliations","id":"irmar"}),
        )],
    );
    execute(
        &mut s,
        vec![
            command(
                "record.remove",
                json!({"collection":"affiliations","id":"irmar"}),
            ),
            command(
                "metadata.set",
                json!({"target":{"kind":"record","collection":"authors","id":"camille","field":"affiliation-ids"},"value":[]}),
            ),
        ],
    );
    assert_eq!(fields(&s)["affiliations"], json!([]));
    assert_eq!(fields(&s)["authors"][0]["affiliation-ids"], json!([]));
}
#[test]
fn record_defaults_whitelist_order_and_unique_languages_are_checked() {
    let s = session(DocumentType::Article);
    for step in [
        command("record.add", json!({"collection":"authors","id":"camille"})),
        command(
            "record.add",
            json!({"collection":"authors","id":"new","values":{"id":"forged"}}),
        ),
        command(
            "record.add",
            json!({"collection":"authors","id":"new","values":{"corresponding":"yes"}}),
        ),
        command(
            "record.add",
            json!({"collection":"authors","id":"new","after":"absent"}),
        ),
        command(
            "record.move",
            json!({"collection":"authors","id":"camille","before":"absent"}),
        ),
        command("record.add", json!({"collection":"private","id":"new"})),
        command(
            "record.add",
            json!({"collection":"localizations","id":"new","values":{"language":"fr"}}),
        ),
    ] {
        assert_rollback(&s, vec![step]);
    }
}

#[test]
fn a_draft_can_drop_its_last_localization_without_becoming_exportable() {
    let mut s = session(DocumentType::Article);
    let id = fields(&s)["localizations"][0]["id"].clone();
    execute(
        &mut s,
        vec![command(
            "record.remove",
            json!({"collection":"localizations","id":id}),
        )],
    );
    assert_eq!(fields(&s)["localizations"], json!([]));
    assert!(!s.state().settings.metadata.ready_for_export());
}

#[test]
fn structural_batches_normalize_whitespace_before_publication_without_touching_neighbors() {
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![
            command(
                "block.insert",
                json!({"parent":tree(&[]),"index":0,"node":{"type":"paragraph"}}),
            ),
            command(
                "block.insert",
                json!({"parent":tree(&[]),"index":5,"node":{"type":"paragraph"}}),
            ),
            command(
                "block.insert",
                json!({"parent":tree(&[]),"index":6,"node":paragraph("Fin")}),
            ),
        ],
    );
    let content = &s.state.document.content;
    assert_eq!(content[0].content[0].text.as_deref(), Some("A😀é日本"));
    assert_eq!(content.len(), 6);
    assert_eq!(content[4].content[0].text.as_deref(), Some("Fin"));
    assert!(content[5].content.is_empty());
    assert!(!execute(&mut s, vec![]).changed);
}
#[test]
fn lists_descriptions_and_rectangular_tables_can_be_inserted_but_not_nested_lists_or_ragged_tables()
{
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![
            command(
                "block.insert",
                json!({"parent":tree(&[]),"index":1,"node":{"type":"orderedList","attrs":{"start":3},"content":[{"type":"listItem","content":[paragraph("Item")]},{"type":"listItem","content":[paragraph("Autre")]}]}}),
            ),
            command("block.remove", json!({"target":tree(&[1,0])})),
            command(
                "block.insert",
                json!({"parent":tree(&[]),"index":2,"node":{"type":"descriptionList","content":[{"type":"descriptionItem","content":[{"type":"descriptionTerm"},{"type":"descriptionDefinition"}]}]}}),
            ),
        ],
    );
    assert_eq!(
        s.state.document.content[1].content[0].content[0].content[0]
            .text
            .as_deref(),
        Some("Autre")
    );
    assert_eq!(
        s.state.document.content[1].attrs.as_ref().unwrap()["start"],
        3
    );
    let fresh = session(DocumentType::Courrier);
    for node in [
        json!({"type":"bulletList","content":[{"type":"listItem","content":[{"type":"bulletList","content":[{"type":"listItem","content":[paragraph("X")]}]}]}]}),
        json!({"type":"table","content":[{"type":"tableRow","content":[{"type":"tableCell","content":[paragraph("X")]}]},{"type":"tableRow","content":[{"type":"tableCell","content":[paragraph("Y")]},{"type":"tableCell","content":[paragraph("Z")]}]}]}),
    ] {
        assert_rollback(
            &fresh,
            vec![command(
                "block.insert",
                json!({"parent":tree(&[]),"index":1,"node":node}),
            )],
        );
    }
    let command = command("block.remove", json!({"target":tree(&[1,0])}));
    assert!(!s.availability(s.guard(), &command).enabled); // last item cannot leave an empty list
}
#[test]
fn malformed_raw_shared_updates_use_the_same_final_structure_validation() {
    let s = session(DocumentType::Courrier);
    let peer = SharedDocument::from_update(&s.update()).unwrap();
    let before = peer.doc.transact().state_vector();
    let root = peer.doc.transact().get_xml_fragment("default").unwrap();
    let cell = targets::element(root, &peer.doc.transact(), &[1, 0, 0]).unwrap();
    cell.remove(&mut peer.doc.transact_mut(), 0);
    let delta = peer.doc.transact().encode_diff_v1(&before);
    let confirmed = s.update();
    assert!(matches!(
        s.prepare(s.guard(), &[delta]),
        Err(Error::InvalidDocument { .. })
    ));
    assert_eq!(s.update(), confirmed);
}
#[test]
fn inline_header_fields_cannot_split_into_two_paragraphs_or_accept_widgets() {
    let s = session(DocumentType::Article);
    let id = fields(&s)["localizations"][0]["id"].clone();
    assert_rollback(
        &s,
        vec![command(
            "block.split",
            json!({"target":{"zone":{"kind":"record","collection":"localizations","id":id,"field":"title","path":[0]},"from":0,"to":0}}),
        )],
    );
    assert_rollback(
        &s,
        vec![command(
            "block.insert",
            json!({"parent":{"root":{"kind":"record","collection":"localizations","id":id,"field":"abstract"},"path":[]},"index":0,"node":{"type":"heading","content":[{"type":"text","text":"Titre"}]}}),
        )],
    );
}

#[test]
fn adding_an_id_with_a_stale_order_entry_uses_the_explicit_anchor() {
    use yrs::Array;
    let original = session(DocumentType::Article);
    let peer = SharedDocument::from_update(&original.update()).unwrap();
    let tx = peer.doc.transact();
    let settings = tx.get_map("settings").unwrap();
    let Some(yrs::Out::YArray(order)) = settings.get(&tx, "order:authors") else {
        panic!()
    };
    drop(tx);
    order.insert(&mut peer.doc.transact_mut(), 0, "alex");
    let mut s =
        SharedSession::open(original.guard().clone(), &peer.update(), Assets::default()).unwrap();
    execute(
        &mut s,
        vec![command(
            "record.add",
            json!({"collection":"authors","id":"alex","after":"camille"}),
        )],
    );
    assert_eq!(fields(&s)["authors"][0]["id"], "camille");
    assert_eq!(fields(&s)["authors"][1]["id"], "alex");
}

#[test]
fn joins_use_the_same_rich_root_without_crossing_body_record_or_cell_boundaries() {
    let mut s = session(DocumentType::Article);
    let root = abstract_tree(&s, &[]);
    execute(
        &mut s,
        vec![
            command(
                "block.insert",
                json!({"parent":root,"index":0,"node":paragraph("Avant ")}),
            ),
            command(
                "block.insert",
                json!({"parent":root,"index":1,"node":paragraph("Après")}),
            ),
        ],
    );
    let left = abstract_tree(&s, &[0]);
    let right = abstract_tree(&s, &[1]);
    execute(
        &mut s,
        vec![command("block.join", json!({"left":left,"right":right}))],
    );
    let content = fields(&s)["localizations"][0]["abstract"]["content"][0]["content"].clone();
    assert_eq!(
        content
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["text"].as_str())
            .collect::<String>(),
        "Avant Après"
    );
    for (a, b) in [
        (left.clone(), tree(&[1])),
        (tree(&[0]), right),
        (tree(&[1, 0, 0, 0]), tree(&[1, 0, 1, 0])),
    ] {
        let before = s.update();
        assert!(
            s.prepare_commands(
                s.guard(),
                &[command("block.join", json!({"left":a,"right":b}))]
            )
            .is_err()
        );
        assert_eq!(s.update(), before);
    }
    let saved = s.guard().clone();
    let prepared = s
        .prepare_commands(
            &saved,
            &[command(
                "block.join",
                json!({"left":abstract_tree(&s,&[0]),"right":abstract_tree(&s,&[1])}),
            )],
        )
        .unwrap();
    execute(
        &mut s,
        vec![command(
            "text.insert",
            json!({"target":body(json!([0]),0,0),"text":"Concurrent"}),
        )],
    );
    let before = s.update();
    assert!(s.publish(prepared).is_err());
    assert_eq!(s.update(), before);
}
