use super::*;

fn results(e: &Engine, query: &str) -> SearchResults {
    serde_json::from_value(
        e.query_call(
            Query::Find(SearchParams {
                query: query.into(),
                case_sensitive: false,
            })
            .to_call(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn all(e: &mut Engine, query: &str, replacement: &str) -> Result<Outcome, CommandError> {
    call(
        e,
        "edit.replace_all",
        json!({"guard":e.target_guard(), "query":query,"replacement":replacement}),
    )
}

#[test]
fn navigation_wraps_with_unicode_case_and_preserves_document_and_history() {
    let mut e = engine("Un *été* et un _ÉTÉ_, été.");
    let original = e.stored_document();
    let found = results(&e, "été");
    let ranges: Vec<_> = found
        .matches
        .iter()
        .map(|m| e.resolve(m.target.into()).unwrap().range())
        .collect();
    for (id, index) in [
        ("edit.find_next", 0),
        ("edit.find_next", 1),
        ("edit.find_previous", 0),
        ("edit.find_previous", 2),
        ("edit.find_next", 0),
    ] {
        let outcome = call(&mut e, id, json!({"query":"été"})).unwrap();
        assert_eq!(e.selection().range(), ranges[index]);
        assert_eq!(outcome.effects, [Effect::RevealSelection]);
        assert_eq!(outcome.value.unwrap(), json!({"index":index,"count":3}));
        assert!(!outcome.changed);
    }
    assert!(e.session.document.view.typing_style.bold);
    assert!(e.stored_document() == original);
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    let target = explicit(&e, 10..10);
    call(
        &mut e,
        "edit.find_next",
        json!({"query":"ÉTÉ","case_sensitive":true,"target":target}),
    )
    .unwrap();
    assert_eq!(e.selection().range(), ranges[1]);
}

#[test]
fn replace_current_uses_expected_text_and_advances_in_one_undoable_transaction() {
    let mut e = engine("Un *été* et un _ÉTÉ_, été.");
    let original = e.stored_document();
    let occurrence = results(&e, "été").matches.remove(0);
    let first = e.resolve(occurrence.target.into()).unwrap();
    let outcome = call(
        &mut e,
        "edit.replace_current",
        json!({"query":"été","occurrence":occurrence,"replacement":"Noël"}),
    )
    .unwrap();
    assert_eq!(outcome.value.unwrap(), json!({"count":1}));
    assert_eq!(outcome.effects, [Effect::Render, Effect::RevealSelection]);
    assert_eq!(e.session.document.view.text, "Un Noël et un ÉTÉ, été.");
    assert!(e.session.document.view.styles[3..7].iter().all(|s| s.bold));
    assert_eq!(
        e.selection().range(),
        e.resolve(results(&e, "été").matches[0].target.into())
            .unwrap()
            .range()
    );
    let after_selection = e.selection();
    assert_eq!(e.session.undo.len(), 1);
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert!(e.stored_document() == original);
    assert_eq!(e.selection(), first);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(e.selection(), after_selection);
}

#[test]
fn replace_all_covers_six_editable_zones_preserves_private_fields_and_styles() {
    let mut e = engine(
        "= *Mot*\n\n/ Mot: _Mot_\n\n#quote(block: true, attribution: [Mot])[Mot]\n\n#figure(image(\"images/mot.svg\", alt: \"Mot\"), caption: [Mot])",
    );
    let before = e.stored_document();
    let outcome = all(&mut e, "mot", "Noël").unwrap();
    assert_eq!(outcome.value.unwrap(), json!({"count":6}));
    assert_eq!(e.session.revision, 1);
    assert_eq!(e.session.undo.len(), 1);
    assert_eq!(results(&e, "Noël").matches.len(), 6);
    assert!(e.session.document.source().contains("alt: \"Mot\""));
    assert!(e.session.document.source().contains("images/mot.svg"));
    let matches = results(&e, "Noël").matches;
    let heading = e.resolve(matches[0].target.into()).unwrap().range();
    let definition = e.resolve(matches[2].target.into()).unwrap().range();
    assert!(
        e.session.document.view.styles[heading]
            .iter()
            .all(|s| s.bold)
    );
    assert!(
        e.session.document.view.styles[definition]
            .iter()
            .all(|s| s.italic)
    );
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert!(e.stored_document() == before);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(results(&e, "Noël").matches.len(), 6);
}

#[test]
fn new_matches_are_not_replaced_recursively_and_deletion_is_undoable() {
    let mut e = engine("été été");
    all(&mut e, "été", "été été").unwrap();
    assert_eq!(e.session.document.view.text, "été été été été");
    assert_eq!(e.session.undo.len(), 1);
    all(&mut e, "été", "").unwrap();
    assert!(results(&e, "été").matches.is_empty());
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.session.document.view.text, "été été été été");
}

#[test]
fn stale_and_foreign_occurrences_and_all_guards_are_refused_even_after_undo() {
    let mut e = engine("été été");
    let occurrence = results(&e, "été").matches.remove(0);
    let current = json!({"query":"été","occurrence":occurrence,"replacement":"Noël"});
    let all = json!({"guard":e.target_guard(),"query":"été","replacement":"Noël"});
    let mut other = engine("été été");
    for (id, params) in [
        ("edit.replace_current", &current),
        ("edit.replace_all", &all),
    ] {
        assert_eq!(
            call(&mut other, id, params.clone()).unwrap_err().reason(),
            "foreign_document"
        );
    }
    call(&mut e, "text.insert", json!({"text":"X"})).unwrap();
    call(&mut e, "edit.undo", json!({})).unwrap();
    let before = e.stored_document();
    let selection = e.selection();
    let revision = e.session.revision;
    let when = e.session.changed_at;
    for (id, params) in [("edit.replace_current", current), ("edit.replace_all", all)] {
        let command = Command::from_call(id, params).unwrap();
        assert!(matches!(
            e.available(&command),
            Availability::Disabled { .. }
        ));
        assert_eq!(e.run(command).unwrap_err().reason(), "stale_revision");
    }
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), selection);
    assert_eq!(e.session.revision, revision);
    assert_eq!(e.session.changed_at, when);
    assert!(e.session.undo.is_empty());
    assert_eq!(e.session.redo.len(), 1);
}

#[test]
fn mismatched_text_query_or_partial_occurrence_never_changes_published_state() {
    let mut e = engine("été été");
    let original = e.stored_document();
    let occurrence = results(&e, "été").matches.remove(0);
    let mut wrong_text = occurrence.clone();
    wrong_text.text = "Été".into();
    let partial = SearchOccurrence {
        target: e
            .selection_target(Selection {
                primary: Caret {
                    index: 2,
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap(),
        text: "ét".into(),
    };
    for (query, occurrence, case) in [
        ("été", wrong_text, false),
        ("autre", occurrence.clone(), false),
        ("ÉTÉ", occurrence, true),
        ("été", partial, false),
    ] {
        let err = call(&mut e,"edit.replace_current", json!({"query":query,"case_sensitive":case,"occurrence":occurrence,"replacement":"Noël"})).unwrap_err();
        assert_eq!(err.reason(), "search_mismatch");
    }
    assert!(e.stored_document() == original);
    assert_eq!(e.selection(), Selection::default());
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
}

#[test]
fn identical_replacement_navigates_without_dirtying_or_recording_history() {
    let mut e = engine("été été");
    let occurrence = results(&e, "été").matches.remove(0);
    let outcome = call(
        &mut e,
        "edit.replace_current",
        json!({"query":"été","occurrence":occurrence,"replacement":"été"}),
    )
    .unwrap();
    assert!(!outcome.changed);
    assert_eq!(e.selection().range(), 4..7);
    assert!(!e.is_dirty());
    assert!(e.session.undo.is_empty());
    assert!(!all(&mut e, "été", "été").unwrap().changed);
    assert!(e.session.undo.is_empty());
}

#[test]
fn invalid_replacement_absent_results_and_strict_payloads_are_refused_atomically() {
    let mut e = engine("été .\\*\n\nété");
    let before = e.stored_document();
    for replacement in ["X\nY", "X\tY", "\u{fffc}", "\u{2028}"] {
        assert_eq!(
            all(&mut e, "été", replacement).unwrap_err().reason(),
            "search_one_line"
        );
    }
    for query in ["", "absent", "été\nété"] {
        assert_eq!(
            call(&mut e, "edit.find_next", json!({"query":query}))
                .unwrap_err()
                .reason(),
            "search_no_results"
        );
    }
    assert_eq!(results(&e, ".*").matches.len(), 1);
    for (id, params) in [
        ("edit.find_next", json!({"query":"été","regex":true})),
        ("edit.replace_all", json!({"query":"été","replacement":"X"})),
        (
            "edit.replace_current",
            json!({"query":"été","replacement":"X"}),
        ),
    ] {
        assert_eq!(
            call(&mut e, id, params).unwrap_err().code,
            ErrorCode::InvalidParams
        );
    }
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), Selection::default());
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
}

#[test]
fn oversized_literal_query_is_an_explicit_error_in_read_and_write_paths() {
    let mut e = engine("été");
    let query = "a".repeat(1_000_000);
    let before = e.stored_document();
    assert_eq!(
        e.query(Query::Find(SearchParams {
            query: query.clone(),
            case_sensitive: false
        }))
        .unwrap_err()
        .reason(),
        "search_too_large"
    );
    assert_eq!(
        call(&mut e, "edit.find_next", json!({"query":query}))
            .unwrap_err()
            .reason(),
        "search_too_large"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty());
}
