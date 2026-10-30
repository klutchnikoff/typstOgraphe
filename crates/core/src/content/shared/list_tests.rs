use super::command_tests::{body, command, execute, session};
use super::*;
use crate::document_types::DocumentType;
use serde_json::{Value, json};
use yrs::XmlFragment;
fn tree(path: &[usize]) -> Value {
    json!({"root":{"kind":"body"},"path":path})
}
fn p(text: &str) -> Value {
    if text.is_empty() {
        json!({"type":"paragraph"})
    } else {
        json!({"type":"paragraph","content":[{"type":"text","text":text}]})
    }
}
fn list_node(items: &[&str]) -> Value {
    json!({"type":"bulletList","content":items.iter().map(|text|json!({"type":"listItem","content":[p(text)]})).collect::<Vec<_>>()})
}
fn start(node: Value) -> SharedSession {
    let mut s = session(DocumentType::Courrier);
    execute(
        &mut s,
        vec![command(
            "block.replace",
            json!({"target":tree(&[0]),"node":node}),
        )],
    );
    s
}
fn xml(s: &SharedSession) -> SharedDocument {
    SharedDocument::from_update(&s.update()).unwrap()
}
#[test]
fn list_item_split_and_join_preserve_the_survivor_prefix_and_other_items() {
    let mut s = start(list_node(&["A😀BC", "Second", "Voisin"]));
    let before = s.state.document.clone();
    let peer = xml(&s);
    let tx = peer.doc.transact();
    let root = tx.get_xml_fragment("default").unwrap();
    let first = targets::element(root.clone(), &tx, &[0, 0]).unwrap();
    let first_id = yrs::XmlOut::Element(first).id();
    let paragraph = targets::element(root.clone(), &tx, &[0, 0, 0]).unwrap();
    let text_id = paragraph.get(&tx, 0).unwrap().id();
    let neighbor = yrs::XmlOut::Element(targets::element(root, &tx, &[0, 2]).unwrap()).id();
    execute(
        &mut s,
        vec![command(
            "list.item.split",
            json!({"target":body(json!([0,0,0]),3,3)}),
        )],
    );
    assert_eq!(s.state.document.content[0].content.len(), 4);
    assert_eq!(
        s.state.document.content[0].content[0].content[0].content[0]
            .text
            .as_deref(),
        Some("A😀")
    );
    let after = xml(&s);
    let tx = after.doc.transact();
    let root = tx.get_xml_fragment("default").unwrap();
    assert_eq!(
        yrs::XmlOut::Element(targets::element(root.clone(), &tx, &[0, 0]).unwrap()).id(),
        first_id
    );
    assert_eq!(
        targets::element(root.clone(), &tx, &[0, 0, 0])
            .unwrap()
            .get(&tx, 0)
            .unwrap()
            .id(),
        text_id
    );
    assert_eq!(
        yrs::XmlOut::Element(targets::element(root, &tx, &[0, 3]).unwrap()).id(),
        neighbor
    );
    execute(
        &mut s,
        vec![command(
            "list.item.join",
            json!({"left":tree(&[0,0]),"right":tree(&[0,1])}),
        )],
    );
    assert_eq!(
        normalization::node(&s.state.document),
        normalization::node(&before)
    );
    let after = xml(&s);
    let tx = after.doc.transact();
    let root = tx.get_xml_fragment("default").unwrap();
    assert_eq!(
        yrs::XmlOut::Element(targets::element(root.clone(), &tx, &[0, 0]).unwrap()).id(),
        first_id
    );
    assert_eq!(
        targets::element(root, &tx, &[0, 0, 0])
            .unwrap()
            .get(&tx, 0)
            .unwrap()
            .id(),
        text_id
    );
}
#[test]
fn item_split_replaces_a_range_and_moves_following_paragraphs_with_marks_and_badges() {
    let node = json!({"type":"orderedList","attrs":{"start":4},"content":[
        {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Avant après","marks":[{"type":"italic"}]},{"type":"footnote","attrs":{"text":"Note"}}]},p("Deuxième paragraphe")]},
        {"type":"listItem","content":[p("Voisin")]}
    ]});
    let mut s = start(node);
    execute(
        &mut s,
        vec![command(
            "list.item.split",
            json!({"target":body(json!([0,0,0]),5,6)}),
        )],
    );
    let list = &s.state.document.content[0];
    assert_eq!(list.attrs.as_ref().unwrap()["start"], 4);
    assert_eq!(list.content[0].content.len(), 1);
    assert_eq!(list.content[1].content.len(), 2);
    assert_eq!(
        list.content[1].content[0].content[0].text.as_deref(),
        Some("après")
    );
    assert_eq!(
        list.content[1].content[0].content[0].marks[0].kind,
        "italic"
    );
    assert_eq!(list.content[1].content[0].content[1].kind, "footnote");
    execute(
        &mut s,
        vec![command(
            "list.item.join",
            json!({"left":tree(&[0,0]),"right":tree(&[0,1])}),
        )],
    );
    assert_eq!(s.state.document.content[0].content[0].content.len(), 2);
}
#[test]
fn description_split_keeps_term_and_definition_separate_and_exit_keeps_the_continuation() {
    let mut s = start(
        json!({"type":"descriptionList","content":[{"type":"descriptionItem","content":[
            {"type":"descriptionTerm","content":[{"type":"text","text":"Terme"}]},
            {"type":"descriptionDefinition","content":[{"type":"text","text":"Avant après"}]}
        ]}]}),
    );
    execute(
        &mut s,
        vec![command(
            "list.item.split",
            json!({"target":body(json!([0,0,1]),5,6)}),
        )],
    );
    let list = &s.state.document.content[0];
    assert!(list.content[1].content[0].content.is_empty());
    assert_eq!(
        list.content[1].content[1].content[0].text.as_deref(),
        Some("après")
    );
    for bad in [
        command(
            "list.item.split",
            json!({"target":body(json!([0,0,0]),1,1)}),
        ),
        command(
            "list.item.join",
            json!({"left":tree(&[0,0]),"right":tree(&[0,1])}),
        ),
        command("list.item.exit", json!({"target":tree(&[0,0])})),
    ] {
        let before = s.update();
        assert!(s.prepare_commands(s.guard(), &[bad]).is_err());
        assert_eq!(s.update(), before);
    }
    let mut s = start(list_node(&["Un", "", "Deux"]));
    execute(
        &mut s,
        vec![command("list.item.exit", json!({"target":tree(&[0,1])}))],
    );
    assert_eq!(s.state.document.content[0].content.len(), 2);
    assert_eq!(s.state.document.content[1].kind, "paragraph");
    assert!(s.state.document.content[1].content.is_empty());
    let mut s = start(list_node(&[""]));
    execute(
        &mut s,
        vec![command("list.item.exit", json!({"target":tree(&[0,0])}))],
    );
    assert!(
        s.state
            .document
            .content
            .iter()
            .all(|n| n.kind != "bulletList")
    );
}
#[test]
fn list_operations_refuse_bad_boundaries_surrogates_and_false_empty_badges_without_partial_publication()
 {
    let mut s = start(list_node(&["A😀B", "Deux"]));
    for bad in [
        command(
            "list.item.split",
            json!({"target":body(json!([0,0,0]),2,2)}),
        ),
        command(
            "list.item.join",
            json!({"left":tree(&[0,0]),"right":tree(&[1,0])}),
        ),
        command(
            "list.item.join",
            json!({"left":tree(&[0,usize::MAX]),"right":tree(&[0,0])}),
        ),
        command("list.item.exit", json!({"target":tree(&[0,0])})),
    ] {
        let before = s.update();
        let commands = [
            command(
                "text.insert",
                json!({"target":body(json!([0,1,0]),0,0),"text":"Privé"}),
            ),
            bad,
        ];
        assert!(s.prepare_commands(s.guard(), &commands).is_err());
        assert_eq!(s.update(), before);
    }
    execute(
        &mut s,
        vec![command(
            "block.replace",
            json!({"target":tree(&[0,0,0]),"node":{"type":"paragraph","content":[{"type":"footnote","attrs":{"text":"Ne pas perdre"}}]}}),
        )],
    );
    assert!(
        s.prepare_commands(
            s.guard(),
            &[command("list.item.exit", json!({"target":tree(&[0,0])}))]
        )
        .is_err()
    );
}
