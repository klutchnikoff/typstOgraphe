use super::*;

#[test]
fn acceptance_maps_the_latest_user_context_and_publishes_one_undoable_transaction() {
    let mut e = engine("Avant fin");
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let observer_target = e
        .selection_target(e.resolve(explicit(&e, 6..9)).unwrap())
        .unwrap();
    let observer = e
        .create_edit_context(e.target_guard(), Some(observer_target))
        .unwrap();
    let proposal = e
        .prepare_proposal(
            &agent,
            params(
                &e,
                vec![
                    ("text.bold", json!({"value":"on"})),
                    ("text.insert", json!({"text":"Été にほん "})),
                    ("text.italic", json!({"value":"on"})),
                ],
            ),
        )
        .unwrap();
    // Review does not freeze the user's cursor or typing style.
    let target = explicit(&e, 9..9);
    call(&mut e, "text.select", json!({"target":target})).unwrap();
    call(&mut e, "text.italic", json!({"value":"on"})).unwrap();
    let user = e.primary_context().unwrap();
    let before = e.stored_document();
    let original_guard = e.target_guard();
    let outcome = e.accept_proposal(&agent, &proposal).unwrap();
    assert_eq!(outcome.guard.revision, original_guard.revision + 1);
    assert_eq!(outcome.effects, [Effect::Render]);
    assert_eq!(e.document().view.text, "Été にほん Avant fin");
    assert_eq!(e.selection().range(), 17..17);
    assert_eq!(e.primary_context().unwrap().typing_style, user.typing_style);
    let participant = e.editing_context(&agent).unwrap();
    assert_eq!(participant.selection.selection, proposal.selection());
    assert!(participant.typing_style.bold && participant.typing_style.italic);
    assert!(!e.primary_context().unwrap().typing_style.bold);
    let observer = e.editing_context(&observer).unwrap();
    assert_eq!(e.resolve_target(observer.selection).unwrap(), (14, 17));
    assert_eq!(outcome.selection, e.primary_context().map(|s| s.selection));
    assert_eq!(e.history().undo, 1);
    let accepted = e.stored_document();
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.stored_document(), before);
    assert_eq!(e.primary_context().unwrap().typing_style, user.typing_style);
    assert_eq!(e.selection().range(), 9..9);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(e.stored_document(), accepted);
    assert_eq!(e.selection().range(), 17..17);
}

#[test]
fn stale_wrong_context_and_read_only_acceptance_leave_the_published_state_untouched() {
    let mut e = engine("Texte");
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let other = e.create_edit_context(e.target_guard(), None).unwrap();
    let proposal = e
        .prepare_proposal(
            &agent,
            params(&e, vec![("text.insert", json!({"text":"Proposition "}))]),
        )
        .unwrap();
    assert_eq!(
        e.accept_proposal(&other, &proposal).unwrap_err().reason(),
        "edit_context_closed"
    );
    e.session.read_only = true;
    assert_eq!(
        e.accept_proposal(&agent, &proposal).unwrap_err().reason(),
        "file_read_only"
    );
    assert_eq!(e.document().view.text, "Texte");
    assert_eq!(e.history().undo, 0);
    e.session.read_only = false;
    call(&mut e, "text.insert", json!({"text":"Utilisateur "})).unwrap();
    let before = e.stored_document();
    let context = e.primary_context();
    let participant = e.editing_context(&agent).unwrap();
    let guard = e.target_guard();
    assert_eq!(
        e.accept_proposal(&agent, &proposal).unwrap_err().reason(),
        "stale_revision"
    );
    assert_eq!(e.stored_document(), before);
    assert_eq!(e.primary_context(), context);
    assert_eq!(e.editing_context(&agent).unwrap(), participant);
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.history().undo, 1);
    call(&mut e, "edit.undo", json!({})).unwrap();
    // Identical text after undo is still a different revision.
    assert_eq!(
        e.accept_proposal(&agent, &proposal).unwrap_err().reason(),
        "stale_revision"
    );
    let guard = e.target_guard();
    call(&mut e, "document.close", json!({"guard":guard})).unwrap();
    assert_eq!(
        e.accept_proposal(&agent, &proposal).unwrap_err().reason(),
        "foreign_document"
    );
}

#[test]
fn accepting_style_only_updates_only_the_participant_without_dirtying_or_losing_redo() {
    let mut e = engine("Texte");
    call(&mut e, "text.insert", json!({"text":"X"})).unwrap();
    call(&mut e, "edit.undo", json!({})).unwrap();
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let proposal = e
        .prepare_proposal(
            &agent,
            params(&e, vec![("text.bold", json!({"value":"on"}))]),
        )
        .unwrap();
    let before = e.stored_document();
    let guard = e.target_guard();
    let user = e.primary_context();
    let outcome = e.accept_proposal(&agent, &proposal).unwrap();
    assert!(!outcome.changed && outcome.effects.is_empty());
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.stored_document(), before);
    assert_eq!(e.primary_context(), user);
    assert!(e.editing_context(&agent).unwrap().typing_style.bold);
    assert_eq!(e.history().undo, 0);
    assert_eq!(e.history().redo, 1);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(e.document().view.text, "XTexte");
}

#[test]
fn accepted_assets_signature_and_metadata_share_one_transaction_without_saving() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("courrier.typsto");
    let mut e = engine("Bonjour");
    let guard = e.target_guard();
    call(
        &mut e,
        "document.save_as",
        json!({"guard":guard,"path":path}),
    )
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let before = e.stored_document();
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let image = ImageInput::from(&crate::assets::ImageData {
        extension: "svg".into(),
        data: include_bytes!("../../../../../../fixtures/parc.svg")
            .as_slice()
            .into(),
    });
    let proposal = e
        .prepare_resolved_proposal(
            &agent,
            params(
                &e,
                vec![
                    ("insert.image", json!({"image":image,"caption":"Vue"})),
                    ("document.set_signature", json!({"image":image})),
                    (
                        "metadata.set",
                        json!({"field":"letter.subject","value":"Courrier relu"}),
                    ),
                ],
            ),
            |_, draft, call| {
                let mut p = call.params.clone();
                if call.id.as_str() != "insert.image" {
                    p["guard"] = json!(draft.target_guard());
                }
                Ok(ResolvedBatchStep {
                    command: Command::from_call(call.id.as_str(), p)?,
                    selection: None,
                })
            },
        )
        .unwrap();
    let candidate = proposal.document().clone();
    let outcome = e.accept_proposal(&agent, &proposal).unwrap();
    assert_eq!(e.stored_document(), candidate);
    assert_eq!(outcome.effects, [Effect::Render]);
    assert!(e.is_dirty());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(e.history().undo, 1);
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.stored_document(), before);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(e.stored_document(), candidate);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
