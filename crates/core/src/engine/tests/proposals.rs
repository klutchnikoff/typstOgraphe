use super::*;
mod acceptance;
fn params(e: &Engine, steps: Vec<(&str, Value)>) -> BatchParams {
    BatchParams {
        guard: e.target_guard(),
        save: None,
        steps: steps
            .into_iter()
            .map(|(id, params)| Call {
                id: id.to_owned().try_into().unwrap(),
                params,
            })
            .collect(),
    }
}
#[test]
fn entire_proposal_is_isolated_including_context_style_history_and_review_positions() {
    let mut e = engine("Avant fin");
    call(&mut e, "text.select", json!({"target":{"kind":"all"}})).unwrap();
    let user = e.primary_context().unwrap();
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let before = e.stored_document();
    let original = e.editing_context(&agent).unwrap();
    let retained = e
        .selection_target(Selection {
            secondary: Caret {
                index: 6,
                ..Default::default()
            },
            primary: Caret {
                index: 9,
                ..Default::default()
            },
        })
        .unwrap();
    let proposal = e
        .prepare_proposal(
            &agent,
            params(
                &e,
                vec![
                    ("text.bold", json!({"value":"on"})),
                    ("text.insert", json!({"text":"Été "})),
                    ("text.bold", json!({"value":"off"})),
                    ("text.insert", json!({"text":"à Paris. "})),
                ],
            ),
        )
        .unwrap();
    let candidate = Document::from_source(&proposal.document().source).unwrap();
    assert_eq!(candidate.view.text, "Été à Paris. Avant fin");
    assert!(candidate.view.styles[..4].iter().all(|s| s.bold));
    assert!(candidate.view.styles[4..].iter().all(|s| !s.bold));
    assert!(!proposal.typing_style().bold);
    assert!(proposal.map_for_review(retained).is_ok());
    assert_eq!(e.stored_document(), before);
    assert_eq!(e.primary_context(), Some(user));
    assert_eq!(e.editing_context(&agent).unwrap(), original);
    assert_eq!(e.history().undo, 0);
    e.check_proposal(&agent, &proposal).unwrap();
    // Cursor movement does not invalidate content; any published edit does.
    e.update_adapter_cursor(Selection::default()).unwrap();
    e.check_proposal(&agent, &proposal).unwrap();
    call(&mut e, "text.insert", json!({"text":"Un "})).unwrap();
    assert_eq!(
        e.check_proposal(&agent, &proposal).unwrap_err().reason(),
        "stale_revision"
    );
    drop(proposal);
    assert!(e.stored_document().source.starts_with("Un Avant"));
}
#[test]
fn style_only_and_failed_proposals_never_change_participant_or_primary_state() {
    let mut e = engine("Texte");
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let proposal = e
        .prepare_proposal(
            &agent,
            params(&e, vec![("text.italic", json!({"value":"on"}))]),
        )
        .unwrap();
    assert!(proposal.typing_style().italic);
    assert_eq!(proposal.document(), &e.stored_document());
    assert!(!e.editing_context(&agent).unwrap().typing_style.italic);
    assert!(!e.primary_context().unwrap().typing_style.italic);
    let other = e.create_edit_context(e.target_guard(), None).unwrap();
    assert_eq!(
        e.check_proposal(&other, &proposal).unwrap_err().reason(),
        "edit_context_closed"
    );
    let guard = e.target_guard();
    assert!(
        e.prepare_proposal(
            &agent,
            params(
                &e,
                vec![
                    ("text.insert", json!({"text":"Provisoire"})),
                    ("text.link", json!({"url":"javascript:bad"})),
                ]
            )
        )
        .is_err()
    );
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.history().undo, 0);
}
#[test]
fn save_and_forbidden_steps_are_rejected_before_resolution_or_resource_reads() {
    let mut e = engine("Texte");
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    for forbidden in [
        "edit.undo",
        "document.open",
        "recovery.restore",
        "document.batch",
    ] {
        let p = params(
            &e,
            vec![("text.insert", json!({"text":"X"})), (forbidden, json!({}))],
        );
        assert!(
            e.prepare_resolved_proposal(&agent, p, |_, _, _| panic!("preflight before resolver"))
                .is_err()
        );
    }
    let mut p = params(
        &e,
        vec![(
            "insert.image",
            json!({"image":{"kind":"path","path":"missing.svg"}}),
        )],
    );
    p.save = Some(BatchSave::Current {});
    let error = e.prepare_resolved_proposal(&agent, p, |_, _, _| panic!("no resource read"));
    assert_eq!(error.err().unwrap().reason(), "proposal_files_forbidden");
    assert_eq!(e.history().undo, 0);
}

#[test]
fn candidate_images_and_signatures_are_private_and_the_proposal_does_not_hold_the_file_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let mut e = engine("Bonjour");
    let guard = e.target_guard();
    call(
        &mut e,
        "document.save_as",
        json!({"guard":guard,"path":path}),
    )
    .unwrap();
    let on_disk = std::fs::read(&path).unwrap();
    let before = e.stored_document();
    let agent = e.create_edit_context(e.target_guard(), None).unwrap();
    let image = ImageInput::from(&crate::assets::ImageData {
        extension: "svg".into(),
        data: include_bytes!("../../../../../fixtures/parc.svg")
            .as_slice()
            .into(),
    });
    let p = params(
        &e,
        vec![
            ("insert.image", json!({"image":image,"caption":"Vue"})),
            ("document.set_signature", json!({"image":image})),
        ],
    );
    let proposal = e
        .prepare_resolved_proposal(&agent, p, |_, draft, call| {
            let mut p = call.params.clone();
            if call.id.as_str() == "document.set_signature" {
                p["guard"] = json!(draft.target_guard());
            }
            Ok(ResolvedBatchStep {
                command: Command::from_call(call.id.as_str(), p)?,
                selection: None,
            })
        })
        .unwrap();
    assert!(proposal.document().assets.len() > before.assets.len());
    assert_eq!(e.stored_document(), before);
    assert_eq!(std::fs::read(&path).unwrap(), on_disk);
    let guard = e.target_guard();
    call(&mut e, "document.close", json!({"guard":guard})).unwrap();
    assert_eq!(
        e.check_proposal(&agent, &proposal).unwrap_err().reason(),
        "foreign_document"
    );
    // The candidate remains alive, but it must not prolong ownership of the file.
    let mut other = Engine::empty();
    let guard = other.target_guard();
    call(
        &mut other,
        "document.open",
        json!({"guard":guard,"path":path}),
    )
    .unwrap();
    assert_eq!(other.stored_document(), before);
}
