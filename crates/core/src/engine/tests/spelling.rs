use super::*;
use crate::{document::DocumentLanguage, spelling::WordLists};

fn issues(e: &Engine) -> SpellingIssues {
    serde_json::from_value(
        e.query_call(Query::SpellingIssues(Empty {}).to_call())
            .unwrap(),
    )
    .unwrap()
}
fn ignore(e: &mut Engine, issue: &SpellingIssue) -> Outcome {
    call(e, "spelling.ignore", json!({"issue":issue})).unwrap()
}
fn reset(e: &mut Engine) -> Outcome {
    call(e, "spelling.reset", json!({"guard":e.target_guard()})).unwrap()
}

#[test]
fn spelling_queries_are_pure_guarded_and_share_the_worker_analysis() {
    let e = engine(
        "Été 🌳 *courrrier* _réservvation_ https://courrrier.example nom\\@courrrier.example 2026",
    );
    let before = e.stored_document();
    let when = e.session.changed_at;
    let selected = e.selection();
    let result = issues(&e);
    assert!(result.available);
    assert_eq!(result.language, DocumentLanguage::French);
    assert_eq!(result.issues.len(), 2);
    let mut checker = crate::spelling::Checker::new(|| {});
    checker.synchronize(
        e.session.revision,
        &e.session.document.view.text,
        DocumentLanguage::French,
        Default::default(),
        true,
        std::time::Instant::now(),
    );
    checker.wait();
    for (public, raw) in result.issues.iter().zip(&checker.issues) {
        assert_eq!(public.target.guard, e.target_guard());
        assert_eq!(e.resolve(public.target.into()).unwrap().range(), raw.range);
        assert_eq!(public.word, raw.word);
    }
    let suggestions = e
        .query_call(
            Query::SpellingSuggestions(SpellingSuggestionsParams {
                target: SuggestionTarget::Issue {
                    issue: result.issues[0].clone(),
                },
            })
            .to_call(),
        )
        .unwrap();
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .contains(&json!("courrier"))
    );
    let explicit = e
        .query(Query::SpellingSuggestions(SpellingSuggestionsParams {
            target: SuggestionTarget::Word {
                language: DocumentLanguage::French,
                word: "courrrier".into(),
            },
        }))
        .unwrap();
    assert_eq!(suggestions, explicit);
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), selected);
    assert_eq!(e.session.changed_at, when);
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty() && !e.is_dirty());
}

#[test]
fn spelling_navigation_wraps_both_directions_and_preserves_history() {
    let mut e = engine("Un *courrrier* et une _réservvation_.");
    let before = e.stored_document();
    let found = issues(&e);
    for (id, index) in [
        ("spelling.next", 0),
        ("spelling.next", 1),
        ("spelling.next", 0),
        ("spelling.previous", 1),
        ("spelling.previous", 0),
    ] {
        let outcome = call(&mut e, id, json!({})).unwrap();
        assert_eq!(
            e.selection().range(),
            e.resolve(found.issues[index].target.into())
                .unwrap()
                .range()
        );
        assert!(!outcome.changed);
        assert_eq!(outcome.effects, [Effect::RevealSelection]);
    }
    assert!(e.session.document.view.typing_style.bold);
    let target = explicit(&e, 0..0);
    call(&mut e, "spelling.previous", json!({"target":target})).unwrap();
    assert_eq!(
        e.selection().range(),
        e.resolve(found.issues[1].target.into()).unwrap().range()
    );
    assert!(e.stored_document() == before);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    assert_eq!(e.session.revision, 0);
}

#[test]
fn correction_is_literal_preserves_styles_and_all_six_zone_roles_with_undo() {
    for source in [
        "= *courrrier*",
        "/ *courrrier*: Bonjour",
        "/ Bonjour: *courrrier*",
        "#quote(block: true, attribution: [Bonjour])[*courrrier*]",
        "#quote(block: true, attribution: [*courrrier*])[Bonjour]",
        "#figure(image(\"images/parc.svg\", alt: \"courrrier\"), caption: [*courrrier*])",
    ] {
        let mut e = engine(source);
        let before = e.stored_document();
        let issue = issues(&e).issues.remove(0);
        let range = e.resolve(issue.target.into()).unwrap().range();
        call(
            &mut e,
            "text.select",
            json!({"target":TextTarget::from(issue.target)}),
        )
        .unwrap();
        let outcome = call(
            &mut e,
            "spelling.correct",
            json!({"issue":issue,"replacement":"courrier"}),
        )
        .unwrap();
        assert!(outcome.changed);
        assert_eq!(e.session.undo.len(), 1);
        assert_eq!(e.selection().range(), (range.start + 8)..(range.start + 8));
        assert!(
            e.session.document.view.styles[range.start..range.start + 8]
                .iter()
                .all(|s| s.bold)
        );
        assert!(issues(&e).issues.is_empty());
        let after = e.stored_document();
        call(&mut e, "edit.undo", json!({})).unwrap();
        assert!(e.stored_document() == before);
        assert_eq!(e.selection().range(), range);
        call(&mut e, "edit.redo", json!({})).unwrap();
        assert!(e.stored_document() == after);
    }
    let mut e = engine("courrrier");
    let issue = issues(&e).issues.remove(0);
    call(
        &mut e,
        "spelling.correct",
        json!({"issue":issue,"replacement":"_été_"}),
    )
    .unwrap();
    assert_eq!(e.session.document.view.text, "_été_");
    assert!(e.session.document.view.styles.iter().all(|s| !s.italic));
}

#[test]
fn ignore_is_normalized_document_scoped_and_reset_clears_all_languages_in_one_undo() {
    let mut e = engine("Courrrier courrrier re\u{301}servvation réservvation");
    let main = e.session.metadata.main_source();
    let before = e.stored_document();
    let issue = issues(&e).issues.remove(0);
    ignore(&mut e, &issue);
    assert_eq!(issues(&e).issues.len(), 2);
    assert_eq!(e.session.metadata.main_source(), main);
    let issue = issues(&e).issues.remove(0);
    ignore(&mut e, &issue);
    assert!(issues(&e).issues.is_empty());
    assert!(e.session.metadata.options().spelling_ignored["fr"].contains("réservvation"));
    // Préparer une autre langue comme le fera la famille métadonnées.
    let before_en = e.session.snapshot(e.selection());
    e.session
        .metadata
        .options_mut()
        .spelling_ignored
        .insert("en".into(), ["helllo".into()].into());
    let selection = e.selection();
    e.session.remember(before_en, selection, false).unwrap();
    let with_exceptions = e.stored_document();
    let len = e.session.undo.len();
    reset(&mut e);
    assert_eq!(e.session.undo.len(), len + 1);
    assert!(e.session.metadata.options().spelling_ignored.is_empty());
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert!(e.stored_document() == with_exceptions);
    call(&mut e, "edit.redo", json!({})).unwrap();
    let len = e.session.undo.len();
    assert!(!reset(&mut e).changed);
    assert_eq!(e.session.undo.len(), len);
    assert!(before.metadata.options().spelling_ignored.is_empty());
    let another = engine("courrrier réservvation");
    assert_eq!(issues(&another).issues.len(), 2);
}

#[test]
fn personal_words_are_explicit_read_configuration_and_invalidate_cached_issues() {
    let mut e = engine("Courrrier réservvation");
    let before = e.stored_document();
    let first = issues(&e).issues.remove(0);
    let personal = WordLists::from([("fr".into(), ["COURRRIER".into()].into())]);
    e.configure_spelling(&personal);
    assert_eq!(issues(&e).issues.len(), 1);
    assert_eq!(
        call(&mut e, "spelling.ignore", json!({"issue":first}))
            .unwrap_err()
            .reason(),
        "spelling_mismatch"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty() && !e.is_dirty());
    e.configure_spelling(&WordLists::new());
    assert_eq!(issues(&e).issues.len(), 2);
    assert!(personal["fr"].contains("COURRRIER"));
}

#[test]
fn stale_foreign_and_text_mismatched_issues_never_change_document_or_selection() {
    let mut e = engine("Un courrrier");
    let issue = issues(&e).issues.remove(0);
    let mut other = engine("Un courrrier");
    let params = json!({"issue":issue,"replacement":"courrier"});
    assert_eq!(
        call(&mut other, "spelling.correct", params.clone())
            .unwrap_err()
            .reason(),
        "foreign_document"
    );
    let mut wrong = issue.clone();
    wrong.word = "Courrrier".into();
    let mut language = issue.clone();
    language.language = DocumentLanguage::English;
    for (issue, reason) in [
        (wrong, "spelling_mismatch"),
        (language, "spelling_language"),
    ] {
        assert_eq!(
            call(&mut e, "spelling.ignore", json!({"issue":issue}))
                .unwrap_err()
                .reason(),
            reason
        );
    }
    let guard = e.target_guard();
    call(&mut e, "text.insert", json!({"text":"X"})).unwrap();
    call(&mut e, "edit.undo", json!({})).unwrap();
    let before = e.stored_document();
    let selected = e.selection();
    let revision = e.session.revision;
    for (id, params) in [
        ("spelling.correct", params),
        ("spelling.ignore", json!({"issue":issue})),
        ("spelling.reset", json!({"guard":guard})),
    ] {
        assert_eq!(
            call(&mut e, id, params).unwrap_err().reason(),
            "stale_revision"
        );
    }
    assert_eq!(
        e.query(Query::SpellingSuggestions(SpellingSuggestionsParams {
            target: SuggestionTarget::Issue { issue }
        }))
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), selected);
    assert_eq!(e.session.revision, revision);
    assert!(e.session.undo.is_empty());
    assert_eq!(e.session.redo.len(), 1);
}

#[test]
fn invalid_corrections_payloads_and_partial_words_are_refused_before_publication() {
    let mut e = engine("courrrier");
    let before = e.stored_document();
    let issue = issues(&e).issues.remove(0);
    for replacement in ["X\nY", "X\tY", "\u{fffc}", "\u{2029}"] {
        assert_eq!(
            call(
                &mut e,
                "spelling.correct",
                json!({"issue":issue,"replacement":replacement})
            )
            .unwrap_err()
            .reason(),
            "invalid_text"
        );
    }
    let target = explicit(&e, 1..8);
    let TextTarget::Selection { guard, selection } = target else {
        unreachable!()
    };
    let partial = SpellingIssue {
        target: SelectionTarget { guard, selection },
        language: DocumentLanguage::French,
        word: "ourrrie".into(),
    };
    assert_eq!(
        call(&mut e, "spelling.ignore", json!({"issue":partial}))
            .unwrap_err()
            .reason(),
        "spelling_mismatch"
    );
    for (id, params) in [
        ("spelling.correct", json!({"issue":issue})),
        ("spelling.ignore", json!({"issue":issue,"personal":true})),
        ("spelling.reset", json!({})),
        ("spelling.next", json!({"range":[0,2]})),
    ] {
        assert_eq!(
            call(&mut e, id, params).unwrap_err().code,
            ErrorCode::InvalidParams
        );
    }
    for word in ["", "deux mots", "courrrier\n", "2026", "https://test.fr"] {
        assert_eq!(
            e.query(Query::SpellingSuggestions(SpellingSuggestionsParams {
                target: SuggestionTarget::Word {
                    language: DocumentLanguage::French,
                    word: word.into()
                }
            }))
            .unwrap_err()
            .reason(),
            "spelling_word"
        );
    }
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), Selection::default());
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty());
}

#[test]
fn unsupported_languages_are_reported_and_empty_navigation_is_unavailable() {
    let mut e = engine("Bonjour");
    assert!(issues(&e).issues.is_empty());
    assert_eq!(
        call(&mut e, "spelling.next", json!({}))
            .unwrap_err()
            .reason(),
        "spelling_no_issues"
    );
    e.session.metadata.options_mut().language = DocumentLanguage::German;
    let result = issues(&e);
    assert!(!result.available && result.issues.is_empty());
    assert_eq!(
        call(&mut e, "spelling.previous", json!({}))
            .unwrap_err()
            .reason(),
        "spelling_no_dictionary"
    );
    assert_eq!(
        e.query(Query::SpellingSuggestions(SpellingSuggestionsParams {
            target: SuggestionTarget::Word {
                language: DocumentLanguage::German,
                word: "halllo".into()
            }
        }))
        .unwrap_err()
        .reason(),
        "spelling_no_dictionary"
    );
    assert!(e.session.undo.is_empty());
}
