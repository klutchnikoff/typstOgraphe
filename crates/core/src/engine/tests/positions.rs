use super::*;
use crate::document::edits::TextInput;
fn engine(source: &str) -> Engine {
    let mut stored =
        crate::storage::StoredDocument::blank(crate::document_types::DocumentType::Courrier);
    stored.source = source.into();
    Engine::from_stored(stored, None).unwrap()
}
fn span(e: &Engine, a: usize, h: usize) -> SelectionTarget {
    e.selection_target(Selection {
        secondary: Caret {
            index: a,
            prefer_next_row: false,
        },
        primary: Caret {
            index: h,
            prefer_next_row: true,
        },
    })
    .unwrap()
}
fn insert(e: &mut Engine, a: usize, b: usize, text: &str) {
    let target = span(e, a, b).into();
    e.run(Command::InsertText(InsertTextParams {
        target,
        text: text.into(),
        input_rules: InputRules::Literal,
    }))
    .unwrap();
}
fn mapped(e: &Engine, old: SelectionTarget) -> (usize, usize) {
    e.resolve_target(e.position_map().unwrap().map_retained(old).unwrap())
        .unwrap()
}
#[test]
fn unicode_repeated_text_and_reversed_selection_follow_actual_replacement() {
    let mut e = engine("été été été");
    let last = span(&e, 11, 8);
    let inside = span(&e, 5, 5);
    insert(&mut e, 4, 7, "にほん");
    assert_eq!(mapped(&e, last), (11, 8));
    assert_eq!(mapped(&e, inside), (7, 7));
    assert!(e.resolve_target(last).is_err());
    let last = e.position_map().unwrap().map_retained(last).unwrap();
    insert(&mut e, 0, 0, "👩‍💻 ");
    assert_eq!(mapped(&e, last), (15, 12));
}
#[test]
fn replace_all_maps_each_range_without_treating_the_whole_document_as_replaced() {
    let mut e = engine("été - été - fin");
    let dash = span(&e, 4, 5);
    let tail = span(&e, 12, 15);
    e.run(Command::ReplaceAll(ReplaceAllParams {
        guard: e.target_guard(),
        query: "été".into(),
        case_sensitive: true,
        replacement: "Noël".into(),
    }))
    .unwrap();
    assert_eq!(mapped(&e, dash), (5, 6));
    assert_eq!(mapped(&e, tail), (14, 17));
    let middle = e.position_map().unwrap().map_retained(dash).unwrap();
    e.run(Command::Undo(Empty {})).unwrap();
    assert_eq!(mapped(&e, middle), (4, 5));
    let old = e.position_map().unwrap().map_retained(middle).unwrap();
    e.run(Command::Redo(Empty {})).unwrap();
    assert_eq!(mapped(&e, old), (5, 6));
}
#[test]
fn split_normalization_and_removed_annotation_map_back_to_valid_zones() {
    let mut e = engine("Terme");
    let old = span(&e, 2, 2);
    e.run(Command::Description(TargetParams {
        target: TextTarget::All {},
    }))
    .unwrap();
    let moved = e.position_map().unwrap().map_retained(old).unwrap();
    assert!(e.resolve_target(moved).is_ok());
    let mut e = engine("#quote(block: true, attribution: [Auteur])[Texte]\n\nFin");
    let text = e.document().view.text.clone();
    let start = text.chars().count() - 3;
    let tail = span(&e, start, start + 3);
    insert(&mut e, 0, start - 1, "");
    assert_eq!(mapped(&e, tail), (1, 4));
}
#[test]
fn native_edits_are_tracked_but_preedit_and_invalid_drafts_never_publish_a_map() {
    let mut e = engine("é é 日本");
    let old = span(&e, 5, 7);
    let before = e.begin_native_edit(e.selection());
    let mut input = TextInput {
        view: before.view().clone(),
    };
    input.insert_text("œ", 0);
    assert!(e.position_map().is_none());
    assert_eq!(e.resolve_target(old).unwrap(), (5, 7));
    e.commit_native_edit(
        before,
        input.view,
        Selection::default(),
        NativeInput::Composition,
    )
    .unwrap();
    assert_eq!(mapped(&e, old), (6, 8));
    let guard = e.target_guard();
    let before = e.begin_native_edit(e.selection());
    let mut bad = before.view().clone();
    bad.text.push('x');
    bad.styles.push(Default::default());
    assert!(
        e.commit_native_edit(before, bad, Selection::default(), NativeInput::Typing)
            .is_err()
    );
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.position_map().unwrap().after, guard);
}
#[test]
fn grouped_typing_history_and_literal_shortcuts_keep_valid_inverse_maps() {
    let mut e = engine("Fin");
    for text in ["a", "b"] {
        let before = e.begin_native_edit(e.selection());
        let mut input = TextInput {
            view: before.view().clone(),
        };
        let at = e.selection().primary.index;
        input.insert_text(text, at);
        let selection = Selection {
            primary: Caret {
                index: at + 1,
                ..Default::default()
            },
            secondary: Caret {
                index: at + 1,
                ..Default::default()
            },
        };
        e.commit_native_edit(before, input.view, selection, NativeInput::Typing)
            .unwrap();
    }
    let after = span(&e, 3, 4);
    assert_eq!(e.history().undo, 1);
    e.run(Command::Undo(Empty {})).unwrap();
    assert_eq!(mapped(&e, after), (1, 2));
    assert_eq!(e.document().view.text, "Fin");
    let mut e = engine(r"\=");
    e.set_adapter_selection(Selection {
        primary: Caret {
            index: 1,
            ..Default::default()
        },
        secondary: Caret {
            index: 1,
            ..Default::default()
        },
    })
    .unwrap();
    let old = span(&e, 1, 1);
    e.run(Command::InsertText(InsertTextParams {
        target: Default::default(),
        text: " ".into(),
        input_rules: InputRules::Typing,
    }))
    .unwrap();
    assert_eq!(mapped(&e, old), (0, 0));
    let empty = span(&e, 0, 0);
    e.run(Command::Undo(Empty {})).unwrap();
    assert_eq!(e.document().view.text, "= ");
    assert_eq!(mapped(&e, empty), (2, 2));
    let literal = span(&e, 2, 2);
    e.run(Command::Redo(Empty {})).unwrap();
    assert_eq!(mapped(&e, literal), (0, 0));
}
#[test]
fn batch_composes_splices_and_errors_leave_the_previous_publication_intact() {
    let mut e = engine("aaa BBB ccc");
    let middle = span(&e, 4, 7);
    let mut calls = Vec::new();
    for (a, b, text) in [(8, 11, "fin"), (0, 3, "début")] {
        calls.push(
            Command::InsertText(InsertTextParams {
                target: span(&e, a, b).into(),
                text: text.into(),
                input_rules: InputRules::Literal,
            })
            .to_call(),
        );
    }
    // Explicit targets are guarded: use the first edit then an adapter-resolved target on its draft.
    let guard = e.target_guard();
    e.run_resolved_batch(
        BatchParams {
            guard,
            steps: calls,
            save: None,
        },
        |index, draft, call| {
            let command = if index == 1 {
                Command::InsertText(InsertTextParams {
                    target: span(draft, 0, 3).into(),
                    text: "début".into(),
                    input_rules: InputRules::Literal,
                })
            } else {
                Command::from_call(call.id.as_str(), call.params.clone())?
            };
            Ok(ResolvedBatchStep {
                command,
                selection: None,
            })
        },
    )
    .unwrap();
    assert_eq!(mapped(&e, middle), (6, 9));
    assert_eq!(e.history().undo, 1);
    let guard = e.target_guard();
    let stored = e.stored_document();
    assert!(
        e.run(Command::InsertText(InsertTextParams {
            target: middle.into(),
            text: "X".into(),
            input_rules: InputRules::Literal
        }))
        .is_err()
    );
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.stored_document(), stored);
}
