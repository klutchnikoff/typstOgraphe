use super::*;
#[test]
fn a_note_selection_follows_its_identity_after_unicode_text_insertion_and_is_lost_on_removal() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    execute(
        &mut s,
        b,
        &[Command::ReplaceInline {
            target: target(&[0], 3, 3),
            node: serde_json::from_value(json!({"type":"footnote","attrs":{"text":"Note 😀"}}))
                .unwrap(),
        }],
    );
    let note = TextTarget {
        zone: TextZone::Annotation { path: vec![0, 1] },
        from: 5,
        to: 7,
    };
    select(&mut s, a, note);
    execute(
        &mut s,
        b,
        &[Command::Insert {
            target: target(&[0], 0, 0),
            text: "Ajout 😀 ".into(),
            marks: vec![],
        }],
    );
    let after = snapshot(&mut s, a);
    assert!(!after.selection_lost);
    assert_eq!(
        after.selection,
        Some(
            TextTarget {
                zone: TextZone::Annotation { path: vec![0, 1] },
                from: 5,
                to: 7
            }
            .into()
        )
    );
    execute(
        &mut s,
        b,
        &[Command::Delete {
            target: target(&[0], 12, 13),
        }],
    );
    let after = snapshot(&mut s, a);
    assert!(after.selection_lost && after.selection.is_none());
    let before = s.update();
    assert!(
        s.insert(&s.guard().clone(), a, "Ne pas insérer ailleurs".into())
            .is_err()
    );
    assert_eq!(s.update(), before);
}
#[test]
fn deleted_cell_and_caption_targets_never_fall_back_to_the_neighbouring_cell_or_body() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    execute(
        &mut s,
        b,
        &[Command::InsertTable {
            parent: TreeTarget {
                root: TreeRoot::Body,
                path: vec![],
            },
            index: 1,
            rows: 2,
            columns: 2,
            header: false,
            row_headers: false,
        }],
    );
    select(&mut s, a, target(&[1, 0, 0, 1, 0], 0, 0));
    execute(
        &mut s,
        b,
        &[Command::RemoveTableColumn {
            target: TreeTarget {
                root: TreeRoot::Body,
                path: vec![1],
            },
            index: 1,
        }],
    );
    let lost = snapshot(&mut s, a);
    assert!(lost.selection_lost && lost.selection.is_none());
    let before = s.update();
    assert!(
        s.insert(&s.guard().clone(), a, "Mauvaise cellule".into())
            .is_err()
    );
    assert_eq!(s.update(), before);
    select(&mut s, a, target(&[1, 1], 0, 0));
    execute(
        &mut s,
        b,
        &[Command::RemoveBlock {
            target: TreeTarget {
                root: TreeRoot::Body,
                path: vec![1],
            },
        }],
    );
    let lost = snapshot(&mut s, a);
    assert!(lost.selection_lost && lost.selection.is_none());
    let before = s.update();
    assert!(
        s.insert(&s.guard().clone(), a, "Mauvaise légende".into())
            .is_err()
    );
    assert_eq!(s.update(), before);
}
#[test]
fn cursor_at_the_gap_between_atomic_inline_nodes_rebases_without_entering_a_badge() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    execute(&mut s,b,&[Command::ReplaceBlock { target:TreeTarget { root:TreeRoot::Body,path:vec![0] },node:serde_json::from_value(json!({"type":"paragraph","content":[
        {"type":"footnote","attrs":{"text":"Première"}},{"type":"footnote","attrs":{"text":"Seconde"}}
    ]})).unwrap() }]);
    select(&mut s, a, target(&[0], 1, 1));
    execute(
        &mut s,
        b,
        &[Command::Insert {
            target: target(&[0], 0, 0),
            text: "😀".into(),
            marks: vec![],
        }],
    );
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(target(&[0], 3, 3).into())
    );
    insert(&mut s, a, "Au milieu");
    let paragraph = &s.state().document.content[0];
    assert_eq!(
        paragraph.content[1].attrs.as_ref().unwrap()["text"],
        "Première"
    );
    assert_eq!(paragraph.content[2].text.as_deref(), Some("Au milieu"));
    assert_eq!(
        paragraph.content[3].attrs.as_ref().unwrap()["text"],
        "Seconde"
    );
}
