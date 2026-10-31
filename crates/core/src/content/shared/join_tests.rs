use super::command_tests::{command, execute};
use super::*;
use crate::content::contract::Settings;
use serde_json::{Value, json};
use yrs::XmlFragment;

fn p(kind: &str, text: &str) -> Value {
    let mut n = json!({"type":kind,"content":[]});
    if kind == "heading" {
        n["attrs"] = json!({"level":2});
    }
    if !text.is_empty() {
        n["content"] = json!([{"type":"text","text":text}]);
    }
    n
}
fn session(nodes: Value) -> SharedSession {
    let state = State::new(
        serde_json::from_value(json!({"type":"doc","content":nodes})).unwrap(),
        Settings::default(),
    );
    SharedSession::open(
        Guard {
            document: "join".into(),
            revision: 0,
        },
        &SharedDocument::from_state(&state).unwrap().update(),
        Assets::default(),
    )
    .unwrap()
}
fn tree(path: &[usize]) -> Value {
    json!({"root":{"kind":"body"},"path":path})
}
fn join(left: &[usize], right: &[usize]) -> crate::content::commands::Command {
    command("block.join", json!({"left":tree(left),"right":tree(right)}))
}
#[test]
fn heading_pairs_keep_left_kind_and_paragraphs_keep_a_soft_break() {
    for left in ["paragraph", "heading"] {
        for right in ["paragraph", "heading"] {
            let mut s = session(json!([
                p(left, "A😀"),
                p(right, "é日本"),
                p("paragraph", "")
            ]));
            let original = SharedDocument::from_update(&s.update()).unwrap();
            let root = original.doc.transact().get_xml_fragment("default").unwrap();
            let id = root.get(&original.doc.transact(), 0).unwrap().id();
            execute(&mut s, vec![join(&[0], &[1])]);
            let result = &s.state().document.content[0];
            assert_eq!(result.kind, left);
            assert_eq!(
                result
                    .content
                    .iter()
                    .filter(|n| n.kind == "softBreak")
                    .count(),
                usize::from(left == "paragraph" && right == "paragraph")
            );
            assert_eq!(
                result
                    .content
                    .iter()
                    .filter_map(|n| n.text.as_deref())
                    .collect::<String>(),
                "A😀é日本"
            );
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
                id
            );
        }
    }
}
#[test]
fn inline_marks_badges_and_neighbors_survive_and_final_slot_is_a_noop() {
    let mut a = p("paragraph", "Avant");
    a["content"][0]["marks"] = json!([{"type":"bold"}]);
    let b = json!({"type":"paragraph","content":[{"type":"footnote","attrs":{"text":"Note 日本"}}, {"type":"text","text":"Après","marks":[{"type":"italic"},{"type":"link","attrs":{"href":"https://example.org"}}]}]});
    let mut s = session(json!([a, b, p("paragraph", "")]));
    execute(&mut s, vec![join(&[0], &[1])]);
    let nodes = &s.state().document.content[0].content;
    assert_eq!(nodes[0].marks[0].kind, "bold");
    assert_eq!(nodes[1].kind, "softBreak");
    assert_eq!(nodes[2].kind, "footnote");
    assert_eq!(nodes[2].attrs.as_ref().unwrap()["text"], "Note 日本");
    assert_eq!(nodes[3].marks.len(), 2);
    let before = s.update();
    let guard = s.guard().clone();
    assert!(!execute(&mut s, vec![join(&[0], &[1])]).changed);
    assert_eq!(s.update(), before);
    assert_eq!(s.guard(), &guard);
}
#[test]
fn an_empty_neighbor_does_not_demote_a_heading_or_recreate_its_identity() {
    for empty_left in [true, false] {
        let nodes = if empty_left {
            json!([
                p("paragraph", "Préfixe"),
                p("paragraph", ""),
                p("heading", "Titre"),
                p("paragraph", "")
            ])
        } else {
            json!([
                p("paragraph", "Préfixe"),
                p("heading", "Titre"),
                p("paragraph", ""),
                p("paragraph", "Suite"),
                p("paragraph", "")
            ])
        };
        let mut s = session(nodes);
        execute(&mut s, vec![join(&[1], &[2])]);
        assert_eq!(s.state().document.content[1].kind, "heading");
        assert_eq!(
            s.state().document.content[1].content[0].text.as_deref(),
            Some("Titre")
        );
    }
}
#[test]
fn quote_body_and_same_cell_can_join_without_absorbing_annotations_or_other_cells() {
    let quote = json!({"type":"quote","content":[p("paragraph","A"),p("paragraph","B"),{"type":"quoteAttribution","content":[{"type":"text","text":"Source"}]}]});
    let cell = |parts: Value| json!({"type":"tableCell","content":parts});
    let table = json!({"type":"table","content":[{"type":"tableRow","content":[cell(json!([p("paragraph","C"),p("paragraph","D")])),cell(json!([p("paragraph","E")]))]}]});
    let mut s = session(json!([quote, table, p("paragraph", "")]));
    // The table is normalized into a figure by State's public representation.
    let table_path = if s.state().document.content[1].kind == "tableFigure" {
        vec![1, 0, 0, 0]
    } else {
        vec![1, 0, 0]
    };
    let mut left = table_path.clone();
    left.push(0);
    let mut right = table_path.clone();
    right.push(1);
    execute(&mut s, vec![join(&[0, 0], &[0, 1]), join(&left, &right)]);
    assert_eq!(
        s.state().document.content[0].content[0]
            .content
            .iter()
            .filter_map(|n| n.text.as_deref())
            .collect::<String>(),
        "AB"
    );
    let before = s.update();
    assert!(
        s.prepare_commands(s.guard(), &[join(&[0, 0], &[0, 1])])
            .is_err()
    );
    assert_eq!(s.update(), before);
}
#[test]
fn invalid_neighbors_and_cross_roots_refuse_the_entire_lot() {
    let s = session(
        json!([p("paragraph","A"),p("paragraph","B"),{"type":"quote","content":[p("paragraph","C"),{"type":"quoteAttribution"}]},p("paragraph","")]),
    );
    for bad in [
        join(&[1], &[0]),
        join(&[0], &[2]),
        join(&[1], &[2]),
        join(&[2, 0], &[3]),
        join(&[], &[0]),
    ] {
        let before = s.update();
        assert!(
            s.prepare_commands(s.guard(), &[join(&[0], &[1]), bad])
                .is_err()
        );
        assert_eq!(s.update(), before);
    }
}

#[test]
fn an_existing_return_is_reused_instead_of_adding_a_second_one() {
    for (a, b) in [(true, false), (false, true), (true, true)] {
        let mut left = p("paragraph", "A");
        let mut right = p("paragraph", "B");
        if a {
            left["content"]
                .as_array_mut()
                .unwrap()
                .push(json!({"type":"softBreak"}));
        }
        if b {
            right["content"]
                .as_array_mut()
                .unwrap()
                .insert(0, json!({"type":"softBreak"}));
        }
        let mut s = session(json!([left, right, p("paragraph", "")]));
        execute(&mut s, vec![join(&[0], &[1])]);
        let content = &s.state().document.content[0].content;
        assert_eq!(
            content.iter().map(|n| n.kind.as_str()).collect::<Vec<_>>(),
            vec!["text", "softBreak", "text"]
        );
    }
}
