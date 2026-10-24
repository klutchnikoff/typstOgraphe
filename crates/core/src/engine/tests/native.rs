use super::*;

#[test]
fn native_edit_owns_its_before_state_and_publishes_only_the_supplied_projection() {
    let mut e = engine("Un *été*.");
    let selection = Selection {
        primary: Caret {
            index: 2,
            ..Default::default()
        },
        ..Default::default()
    };
    let before = e.stored_document();
    let guard = e.target_guard();
    let edit = e.begin_native_edit(selection);
    assert_eq!(edit.guard(), guard);
    assert_eq!(edit.selection(), selection);
    let mut view = edit.view().clone();
    view.insert_text("Élodie", 0);
    assert!(e.stored_document() == before);
    assert_eq!(e.target_guard(), guard);
    let after = e
        .commit_native_edit(edit, view, selection, NativeInput::Composition)
        .unwrap();
    assert_eq!(e.revision(), guard.revision + 1);
    assert!(e.can_undo());
    assert_eq!(e.metadata(), &before.metadata);
    assert_eq!(e.template(), &before.template);
    assert_eq!(e.dependencies(), &before.dependencies);
    assert!(e.document().view.text.starts_with("Élodie"));
    e.run(Command::Undo(Empty {})).unwrap();
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), selection);
    e.run(Command::Redo(Empty {})).unwrap();
    assert_eq!(e.selection(), after);
}
#[test]
fn stale_and_foreign_native_handles_never_publish_their_projection_or_selection() {
    let mut e = engine("Avant");
    let old = e.begin_native_edit(Selection::default());
    e.run(Command::InsertText(InsertTextParams {
        target: TextTarget::default(),
        text: "Nouveau ".into(),
        input_rules: InputRules::Literal,
    }))
    .unwrap();
    let foreign = engine("Autre").begin_native_edit(Selection::default());
    let before = e.stored_document();
    let guard = e.target_guard();
    let selection = e.selection();
    for edit in [old, foreign] {
        let mut view = edit.view().clone();
        view.insert_text("À refuser", 0);
        assert_eq!(
            e.commit_native_edit(edit, view, Selection::default(), NativeInput::Typing)
                .unwrap_err(),
            crate::document::edits::EditError::StaleDraft
        );
        assert!(e.stored_document() == before);
        assert_eq!(e.target_guard(), guard);
        assert_eq!(e.selection(), selection);
    }
    let mut closed = Engine::empty();
    let guard = closed.target_guard();
    let edit = closed.begin_native_edit(Selection::default());
    let mut view = edit.view().clone();
    view.insert_text("À refuser", 0);
    assert_eq!(
        closed
            .commit_native_edit(edit, view, Selection::default(), NativeInput::Typing)
            .unwrap_err(),
        crate::document::edits::EditError::StaleDraft
    );
    assert_eq!(closed.target_guard(), guard);
    assert!(!closed.is_open() && !closed.can_undo());
}
#[test]
fn native_cursor_moves_change_only_selection_and_typing_context_without_destroying_redo() {
    let mut e = engine("*Été* après");
    e.run(Command::InsertText(InsertTextParams {
        target: TextTarget::default(),
        text: "À annuler ".into(),
        input_rules: InputRules::Literal,
    }))
    .unwrap();
    e.run(Command::Undo(Empty {})).unwrap();
    let guard = e.target_guard();
    let before = e.stored_document();
    e.update_adapter_cursor(Selection {
        primary: Caret {
            index: 2,
            ..Default::default()
        },
        secondary: Caret {
            index: 1,
            ..Default::default()
        },
    })
    .unwrap();
    assert!(e.document().view.typing_style.bold);
    assert!(e.can_redo());
    e.update_adapter_cursor(Selection::default()).unwrap();
    assert_eq!(e.document().view.typing_style, InlineStyle::default());
    e.run(Command::Bold(StyleParams {
        target: TextTarget::default(),
        value: StyleValue::On,
    }))
    .unwrap();
    e.set_adapter_selection(Selection::default()).unwrap();
    assert!(e.document().view.typing_style.bold);
    assert!(e.stored_document() == before);
    assert_eq!(e.target_guard(), guard);
    assert!(!e.can_undo() && e.can_redo());
    assert!(
        e.update_adapter_cursor(Selection {
            primary: Caret {
                index: usize::MAX,
                ..Default::default()
            },
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(e.selection(), Selection::default());
}

#[test]
fn validated_constructor_rejects_invalid_data_and_creates_independent_clean_sessions() {
    let stored = StoredDocument::demo(DocumentType::Courrier);
    let left = Engine::from_stored(stored.clone(), Some("courrier.typsto".into())).unwrap();
    let right = Engine::from_stored(stored.clone(), None).unwrap();
    assert!(left.stored_document() == stored);
    assert!(left.is_open() && !left.is_dirty() && !left.can_undo() && !left.can_redo());
    assert_eq!(
        left.file_path(),
        Some(std::path::Path::new("courrier.typsto"))
    );
    assert_ne!(
        left.target_guard().document_id,
        right.target_guard().document_id
    );
    let mut invalid = stored;
    invalid.source = "#inconnue[texte]".into();
    assert!(Engine::from_stored(invalid, None).is_err());
}
