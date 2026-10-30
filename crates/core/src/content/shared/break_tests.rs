use super::command_tests::{body, command, execute, fields, session, target};
use super::*;
use crate::document_types::DocumentType;
use serde_json::json;
use yrs::XmlFragment;

#[test]
fn inline_breaks_preserve_paragraph_neighbor_and_prefix_identities() {
    for kind in ["softBreak", "hardBreak"] {
        let mut s = session(DocumentType::Courrier);
        let before = SharedDocument::from_update(&s.update()).unwrap();
        let root = before.doc.transact().get_xml_fragment("default").unwrap();
        let paragraph = targets::element(root.clone(), &before.doc.transact(), &[0]).unwrap();
        let paragraph_id = yrs::XmlOut::Element(paragraph.clone()).id();
        let prefix_id = paragraph.get(&before.doc.transact(), 0).unwrap().id();
        let neighbor_id = root.get(&before.doc.transact(), 1).unwrap().id();
        execute(
            &mut s,
            vec![command(
                "inline.replace",
                json!({
                    "target":body(json!([0]),3,4), "node":{"type":kind}
                }),
            )],
        );
        let nodes = &s.state.document.content[0].content;
        assert_eq!(nodes[0].text.as_deref(), Some("A😀"));
        assert_eq!(nodes[1].kind, kind);
        assert_eq!(nodes[2].text.as_deref(), Some("日本"));
        let after = SharedDocument::from_update(&s.update()).unwrap();
        let root = after.doc.transact().get_xml_fragment("default").unwrap();
        let p = targets::element(root.clone(), &after.doc.transact(), &[0]).unwrap();
        assert_eq!(yrs::XmlOut::Element(p.clone()).id(), paragraph_id);
        assert_eq!(p.get(&after.doc.transact(), 0).unwrap().id(), prefix_id);
        assert_eq!(
            root.get(&after.doc.transact(), 1).unwrap().id(),
            neighbor_id
        );
    }
}

#[test]
fn inline_breaks_accept_multiline_zones_and_refuse_plain_or_single_line_fields_atomically() {
    let mut s = session(DocumentType::Article);
    let id = fields(&s)["localizations"][0]["id"].clone();
    for kind in ["softBreak", "hardBreak"] {
        for zone in [
            json!({"kind":"annotation","path":[2,1]}),
            json!({"kind":"record","collection":"authors","id":"camille","field":"name"}),
            json!({"kind":"record","collection":"localizations","id":id,"field":"title","path":[0]}),
        ] {
            let before = s.update();
            let batch = [
                command(
                    "text.insert",
                    json!({"target":body(json!([0]),0,0),"text":"Private"}),
                ),
                command(
                    "inline.replace",
                    json!({"target":target(zone,0,0),"node":{"type":kind}}),
                ),
            ];
            assert!(s.prepare_commands(s.guard(), &batch).is_err());
            assert_eq!(s.update(), before);
        }
        let zone = json!({"kind":"record","collection":"localizations","id":id,"field":"abstract","path":[0]});
        execute(
            &mut s,
            vec![command(
                "text.insert",
                json!({"target":target(zone.clone(),0,0),"text":"Avant après"}),
            )],
        );
        execute(
            &mut s,
            vec![command(
                "inline.replace",
                json!({"target":target(zone,5,5),"node":{"type":kind}}),
            )],
        );
        assert_eq!(
            fields(&s)["localizations"][0]["abstract"]["content"][0]["content"][1]["type"],
            kind
        );
    }
    let bad = command(
        "inline.replace",
        json!({"target":body(json!([0]),2,2),"node":{"type":"softBreak"}}),
    );
    let before = s.update();
    assert!(s.prepare_commands(s.guard(), &[bad]).is_err());
    assert_eq!(s.update(), before);
}

#[test]
fn single_line_header_validation_also_refuses_subtree_replacement_bypasses() {
    let s = session(DocumentType::Article);
    let id = fields(&s)["localizations"][0]["id"].clone();
    for field in ["title", "short-title", "subtitle"] {
        for node in [
            json!({"type":"paragraph","content":[{"type":"text","text":"Avant"},{"type":"softBreak"},{"type":"text","text":"Après"}]}),
            json!({"type":"paragraph","content":[{"type":"text","text":"Avant"},{"type":"hardBreak"},{"type":"text","text":"Après"}]}),
            json!({"type":"paragraph","content":[{"type":"text","text":"Avant\nAprès"}]}),
        ] {
            let bad = command(
                "block.replace",
                json!({"target":{"root":{"kind":"record","collection":"localizations","id":id,"field":field},"path":[0]},"node":node}),
            );
            let before = s.update();
            assert!(s.prepare_commands(s.guard(), &[bad]).is_err());
            assert_eq!(s.update(), before);
        }
    }
}
