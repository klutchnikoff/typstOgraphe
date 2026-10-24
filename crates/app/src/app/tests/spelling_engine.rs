use super::annotations_engine::{equivalent, pair};

#[test]
fn keyboard_ignore_uses_the_selection_after_a_closed_popup_and_learning_stays_global() {
    let (ctx, mut app, _) = pair("courrrier réservvation", 0..0);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.apply(Command::NextSpelling);
    app.apply(Command::SpellingSuggestions);
    assert_eq!(app.spelling.target.as_ref().unwrap().word, "courrrier");
    // Une fermeture du menu conserve son ancienne cible ; un clic ordinaire
    // déplace ensuite la sélection. Le raccourci doit viser la nouvelle faute.
    app.document.selection = editor::cursor(app.spelling.checker.issues[1].range.clone());
    app.apply(Command::IgnoreSpelling);
    assert_eq!(
        app.document.engine.metadata().options().spelling_ignored["fr"],
        ["réservvation".into()].into()
    );
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.apply(Command::NextSpelling);
    let before = app.stored_document();
    let revision = app.document.engine.revision();
    let len = app.document.engine.history().undo;
    app.apply(Command::LearnSpelling);
    assert!(app.preferences.spelling.personal["fr"].contains("courrrier"));
    assert!(app.stored_document() == before);
    assert_eq!(app.document.engine.revision(), revision);
    assert_eq!(app.document.engine.history().undo, len);
    assert!(
        app.document
            .engine
            .query(Query::SpellingIssues(Empty {}))
            .unwrap()["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        Core::from_call("spelling.learn", json!({}))
            .unwrap_err()
            .code,
        crate::engine::ErrorCode::NotFound
    );
}
use super::*;
use crate::engine::{Command as Core, Empty, Engine, Query, SpellingIssues};
use serde_json::{Value, json};

fn run(engine: &mut Engine, id: &str, params: Value) {
    engine
        .run_call(Core::from_call(id, params).unwrap().to_call())
        .unwrap();
}
fn selected(engine: &Engine) -> crate::engine::SpellingIssue {
    let issues: SpellingIssues = serde_json::from_value(
        engine
            .query_call(Query::SpellingIssues(Empty {}).to_call())
            .unwrap(),
    )
    .unwrap();
    issues
        .issues
        .into_iter()
        .find(|i| {
            engine.resolve_target(i.target).unwrap()
                == (
                    engine.selection().range().start,
                    engine.selection().range().end,
                )
        })
        .unwrap()
}

#[test]
fn spelling_navigation_ignore_reset_and_history_match_json_commands() {
    let (ctx, mut app, mut engine) = pair("Un *courrrier* et une _réservvation_.", 0..0);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    for (ui, id) in [
        (Command::NextSpelling, "spelling.next"),
        (Command::PreviousSpelling, "spelling.previous"),
        (Command::NextSpelling, "spelling.next"),
    ] {
        app.apply(ui);
        run(&mut engine, id, json!({}));
        equivalent(&app, &engine);
        assert!(app.preview.editor_scroll_pending);
    }
    let personal = app.preferences.spelling.clone();
    let issue = selected(&engine);
    app.apply(Command::IgnoreSpelling);
    run(&mut engine, "spelling.ignore", json!({"issue":issue}));
    equivalent(&app, &engine);
    assert_eq!(app.preferences.spelling, personal);
    app.apply(Command::ResetSpelling);
    let guard = engine.target_guard();
    run(&mut engine, "spelling.reset", json!({"guard":guard}));
    equivalent(&app, &engine);
    for (ui, id) in [(Command::Undo, "edit.undo"), (Command::Redo, "edit.redo")] {
        app.apply(ui);
        run(&mut engine, id, json!({}));
        equivalent(&app, &engine);
    }
}

#[test]
fn spelling_popup_uses_guarded_selection_and_shared_suggestions_then_corrects_like_json() {
    let (ctx, mut app, mut engine) = pair("Un *courrrier* et une _réservvation_.", 0..0);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.apply(Command::NextSpelling);
    run(&mut engine, "spelling.next", json!({}));
    let issue = selected(&engine);
    run(
        &mut engine,
        "text.select",
        json!({"target":crate::engine::TextTarget::from(issue.target)}),
    );
    app.apply(Command::SpellingSuggestions);
    app.spelling.checker.wait();
    equivalent(&app, &engine);
    let suggestions = engine
        .query(Query::SpellingSuggestions(
            crate::engine::SpellingSuggestionsParams {
                target: crate::engine::SuggestionTarget::Issue {
                    issue: issue.clone(),
                },
            },
        ))
        .unwrap();
    let graphical = app
        .spelling
        .checker
        .suggestions
        .as_ref()
        .unwrap()
        .1
        .as_ref()
        .unwrap();
    assert_eq!(json!(graphical), suggestions["suggestions"]);
    assert!(!engine.can_undo());
    let target = app.spelling.target.clone().unwrap();
    app.correct_spelling(&target, "courrier");
    run(
        &mut engine,
        "spelling.correct",
        json!({"issue":issue,"replacement":"courrier"}),
    );
    equivalent(&app, &engine);
    assert!(app.spelling.target.is_none());
    for (ui, id) in [(Command::Undo, "edit.undo"), (Command::Redo, "edit.redo")] {
        app.apply(ui);
        run(&mut engine, id, json!({}));
        equivalent(&app, &engine);
    }
}

#[test]
fn captured_spelling_actions_never_retarget_an_identical_word_in_another_document() {
    let (ctx, mut app, _) = pair("courrrier", 0..0);
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.apply(Command::NextSpelling);
    app.apply(Command::SpellingSuggestions);
    let old = app.spelling.target.clone().unwrap();
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "courrrier".into();
    app.install_document(stored.clone(), None).unwrap();
    app.refresh_spelling(&ctx);
    app.spelling.checker.wait();
    app.apply(Command::NextSpelling);
    let before = app.stored_document();
    let selection = app.document.selection;
    app.correct_spelling(&old, "courrier");
    app.apply_spelling_issue(Command::IgnoreSpelling, old);
    assert!(app.stored_document() == before);
    assert_eq!(app.document.selection, selection);
    assert!(!app.document.engine.can_undo());
    assert_eq!(
        app.dialogs.message.as_deref(),
        Some(crate::tr!("edit-stale-draft").as_str())
    );
}
