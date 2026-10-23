use super::*;
use crate::document::BlockKind;

#[test]
fn description_navigation_does_not_write_a_separator_or_add_history() {
    let mut e = engine("/ Échéance: Vendredi");
    let target = explicit(&e, 0..0);
    let before = e.stored_document();
    let zones = e.query(Query::Structure(Empty {})).unwrap();
    for part in ["definition", "definition", "term", "definition", "term"] {
        let outcome = call(&mut e, "text.select", json!({"target":target,"part":part})).unwrap();
        assert_eq!(
            e.selection().range(),
            if part == "term" { 0..0 } else { 9..9 }
        );
        assert!(!outcome.changed);
        assert!(outcome.effects.is_empty());
        assert!(e.stored_document() == before);
        assert_eq!(e.query(Query::Structure(Empty {})).unwrap(), zones);
        assert!(e.undo.is_empty());
    }
}

#[test]
fn structured_enter_splits_and_exits_empty_items_with_exact_history() {
    for source in [
        "- Élément",
        "+ Élément",
        "/ Terme: Définition",
        "#postscript[Merci]",
        "#attachment[Rapport]",
    ] {
        let mut e = engine(source);
        let end = e.document.view.len();
        e.set_adapter_selection(e.resolve(explicit(&e, end..end)).unwrap())
            .unwrap();
        let before = e.document.clone();
        let selection = e.selection();
        call(&mut e, "block.split", json!({})).unwrap();
        let after_split = e.document.clone();
        assert_eq!(e.undo.len(), 1);
        call(&mut e, "block.split", json!({})).unwrap();
        assert_eq!(
            *e.document.view.blocks.last().unwrap(),
            BlockKind::Paragraph
        );
        assert_eq!(e.undo.len(), 2);
        call(&mut e, "edit.undo", json!({})).unwrap();
        assert_eq!(e.document, after_split);
        call(&mut e, "edit.undo", json!({})).unwrap();
        assert_eq!(e.document, before);
        assert_eq!(e.selection(), selection);
    }
}

#[test]
fn whitespace_only_list_and_note_rows_keep_their_kind_when_split() {
    for source in ["-", "+", "#postscript[]", "#attachment[]"] {
        let mut e = engine(source);
        call(&mut e, "text.insert", json!({"text":" "})).unwrap();
        let kind = e.document.view.blocks[0];
        call(&mut e, "block.split", json!({})).unwrap();
        assert_eq!(e.document.view.blocks, vec![kind, kind]);
    }
}

#[test]
fn leaving_letter_notes_preserves_selected_text_and_styles() {
    for source in [
        "#postscript[Merci *encore*]",
        "#attachment[Rapport]\n#attachment[Formulaire *signé*]",
    ] {
        let mut e = engine(source);
        let before = e.document.clone();
        let target = explicit(&e, 1..4);
        let outcome = call(&mut e, "block.exit", json!({"target":target})).unwrap();
        assert_eq!(outcome.effects, [Effect::Render, Effect::RevealSelection]);
        assert_eq!(e.selection().primary.index, before.view.len() + 1);
        assert!(e.document.view.text.starts_with(&before.view.text));
        assert_eq!(
            e.document.view.styles[..before.view.len()],
            before.view.styles
        );
        call(&mut e, "edit.undo", json!({})).unwrap();
        assert_eq!(e.document, before);
        assert_eq!(e.selection().range(), 1..4);
    }
}

#[test]
fn typing_rules_are_explicit_and_undo_restores_complete_literal_input() {
    for (source, text, literal, kind) in [
        ("\\=", " ", "= ", Some(BlockKind::Heading(1))),
        ("\\==", " ", "== ", Some(BlockKind::Heading(2))),
        ("\\===", " ", "=== ", Some(BlockKind::Heading(3))),
        ("\\-", " ", "- ", Some(BlockKind::Bullet)),
        ("\\+", " ", "+ ", Some(BlockKind::Numbered)),
        ("1\\.", " ", "1. ", Some(BlockKind::Numbered)),
        ("\\/", " ", "/ ", Some(BlockKind::Description)),
        ("\\>", " ", "> ", Some(BlockKind::Quote)),
        ("\\*Été 日本", "*", "*Été 日本*", None),
        ("\\_Été 日本", "_", "_Été 日本_", None),
        (
            "/ Échéance\\:: Vendredi",
            " ",
            "Échéance: \tVendredi",
            Some(BlockKind::Description),
        ),
    ] {
        let mut e = engine(source);
        let at = if source.starts_with("/ Échéance") {
            9
        } else {
            e.document.view.len()
        };
        let target = explicit(&e, at..at);
        let mut plain = engine(source);
        let plain_target = explicit(&plain, at..at);
        call(
            &mut plain,
            "text.insert",
            json!({"target":plain_target,"text":text}),
        )
        .unwrap();
        assert_eq!(plain.document.view.text, literal);
        call(
            &mut e,
            "text.insert",
            json!({"target":target,"text":text,"input_rules":"typing"}),
        )
        .unwrap();
        if source.starts_with("/ Échéance") {
            assert_eq!(e.document.view.text, "Échéance\tVendredi");
            assert_eq!(e.selection().range(), 9..9);
        }
        if let Some(kind) = kind {
            assert_eq!(e.document.view.blocks[0], kind);
        } else {
            assert!(!e.document.view.text.starts_with(['*', '_']));
        }
        assert_eq!(e.undo.len(), 1);
        call(&mut e, "edit.undo", json!({})).unwrap();
        assert_eq!(e.document.view.text, literal);
        assert_eq!(e.selection().range(), at + 1..at + 1);
    }
}

#[test]
fn typed_structure_rules_validate_guards_and_boundaries_before_any_change() {
    let mut e = engine("/ Terme: Définition\n\nSuite.");
    let stale = explicit(&e, 0..0);
    let target = explicit(&e, 0..3);
    call(&mut e, "text.bold", json!({"target":target})).unwrap();
    let boundary = explicit(&e, 2..10);
    let outside = explicit(&e, e.document.view.len()..e.document.view.len());
    for (id, params, reason) in [
        (
            "text.select",
            json!({"target":stale,"part":"term"}),
            "stale_revision",
        ),
        (
            "text.insert",
            json!({"target":stale,"text":" ","input_rules":"typing"}),
            "stale_revision",
        ),
        (
            "block.split",
            json!({"target":boundary}),
            "protected_boundary",
        ),
        (
            "text.select",
            json!({"target":outside,"part":"definition"}),
            "description_required",
        ),
    ] {
        let before = e.stored_document();
        let selection = e.selection();
        let state = (e.revision, e.undo.len(), e.redo.len(), e.changed_at);
        assert_eq!(call(&mut e, id, params).unwrap_err().reason(), reason);
        assert!(e.stored_document() == before);
        assert_eq!(e.selection(), selection);
        assert_eq!(
            (e.revision, e.undo.len(), e.redo.len(), e.changed_at),
            state
        );
    }
}
