use super::*;
fn target(e: &Engine, range: std::ops::Range<usize>) -> SelectionTarget {
    match explicit(e, range) {
        TextTarget::Selection { guard, selection } => SelectionTarget { guard, selection },
        _ => unreachable!(),
    }
}
#[test]
fn neutral_and_captured_contexts_never_follow_the_user_cursor_or_style() {
    let mut e = engine("*Bonjour* monde");
    let captured = target(&e, 8..13);
    let user = target(&e, 0..7);
    call(
        &mut e,
        "text.select",
        json!({"target":TextTarget::from(user)}),
    )
    .unwrap();
    let primary = e.primary_context().unwrap();
    let guard = e.target_guard();
    let agent = e.create_edit_context(guard, None).unwrap();
    let other = e.create_edit_context(guard, Some(captured)).unwrap();
    assert_eq!(
        e.resolve_target(e.editing_context(&agent).unwrap().selection)
            .unwrap(),
        (0, 0)
    );
    assert_eq!(
        e.editing_context(&agent).unwrap().typing_style,
        InlineStyle::default()
    );
    assert_eq!(e.primary_context(), Some(primary.clone()));
    let commands = e
        .query_in_context(&agent, Query::Commands(Empty {}))
        .unwrap();
    let link = commands["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "text.link")
        .unwrap();
    assert_eq!(link["context_availability"]["state"], "disabled");
    assert!(matches!(
        e.available_intent(CommandIntent::Link, TextTarget::Current {}),
        Availability::Enabled
    ));
    e.select_in_context(&agent, captured).unwrap();
    assert_eq!(e.primary_context(), Some(primary));
    call(&mut e, "text.select", json!({"target": {"kind":"all"}})).unwrap();
    for context in [&agent, &other] {
        let state = e.editing_context(context).unwrap();
        assert_eq!(state.selection, captured);
        assert_eq!(
            e.query_in_context(
                context,
                Query::Text(TextQueryParams {
                    target: Default::default()
                })
            )
            .unwrap()["text"],
            "monde"
        );
        let inspect = e
            .query_in_context(context, Query::Inspect(Empty {}))
            .unwrap();
        assert_eq!(inspect["selection"], json!(captured));
        assert_eq!(inspect["typing_style"]["bold"], false);
        let commands = e
            .query_in_context(context, Query::Commands(Empty {}))
            .unwrap();
        assert_eq!(commands["selection"], json!(captured));
    }
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.history().undo, 0);
}
#[test]
fn retained_contexts_follow_native_edits_metadata_batches_and_history_but_old_calls_fail() {
    let mut e = engine("Avant fin");
    let old = target(&e, 6..9);
    let agent = e.create_edit_context(e.target_guard(), Some(old)).unwrap();
    let before = e.begin_native_edit(e.selection());
    let mut input = crate::document::edits::TextInput {
        view: before.view().clone(),
    };
    input.insert_text("Un ", 0);
    e.commit_native_edit(
        before,
        input.view,
        Selection::default(),
        NativeInput::Typing,
    )
    .unwrap();
    assert_eq!(
        e.resolve_target(e.editing_context(&agent).unwrap().selection)
            .unwrap(),
        (9, 12)
    );
    assert_eq!(
        call(
            &mut e,
            "text.delete",
            json!({"target":TextTarget::from(old)})
        )
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    let guard = e.target_guard();
    e.run(Command::Batch(BatchParams {
        guard,
        save: None,
        steps: vec![
            Command::InsertText(InsertTextParams {
                target: explicit(&e, 0..0),
                text: "Été ".into(),
                input_rules: InputRules::Literal,
            })
            .to_call(),
        ],
    }))
    .unwrap();
    assert_eq!(
        e.resolve_target(e.editing_context(&agent).unwrap().selection)
            .unwrap(),
        (13, 16)
    );
    let guard = e.target_guard();
    call(
        &mut e,
        "metadata.set",
        json!({"guard":guard,"field":"letter.subject","value":"Test"}),
    )
    .unwrap();
    assert_eq!(
        e.editing_context(&agent).unwrap().selection.guard,
        e.target_guard()
    );
    for _ in 0..2 {
        call(&mut e, "edit.undo", json!({})).unwrap();
    }
    assert_eq!(
        e.resolve_target(e.editing_context(&agent).unwrap().selection)
            .unwrap(),
        (9, 12)
    );
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(
        e.resolve_target(e.editing_context(&agent).unwrap().selection)
            .unwrap(),
        (13, 16)
    );
}
#[test]
fn context_handles_are_document_scoped_and_released_without_mutating_content() {
    let mut e = engine("Bonjour");
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let other = engine("Bonjour");
    assert_eq!(
        other.editing_context(&agent).unwrap_err().reason(),
        "foreign_document"
    );
    let stored = e.stored_document();
    let clone = agent.clone();
    drop(agent);
    assert!(e.editing_context(&clone).is_ok());
    drop(clone);
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    assert_eq!(e.stored_document(), stored);
    let guard = e.target_guard();
    call(&mut e, "document.close", json!({"guard":guard})).unwrap();
    assert!(e.editing_context(&agent).is_err());
}
