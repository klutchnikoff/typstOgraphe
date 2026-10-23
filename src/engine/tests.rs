use super::*;
use crate::{
    document::{Document, InlineStyle},
    document_types::{DocumentType, Metadata},
    session::Caret,
    storage::StoredDocument,
};
use serde_json::json;

#[path = "tests/annotations.rs"]
mod annotations;

fn engine(source: &str) -> Engine {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = source.into();
    Session::new(stored, Document::from_source(source).unwrap(), None, 0).into()
}
fn call(engine: &mut Engine, id: &str, params: Value) -> Result<Outcome, CommandError> {
    engine.run_call(Call {
        id: id.to_owned().try_into().unwrap(),
        params,
    })
}
fn explicit(engine: &Engine, range: std::ops::Range<usize>) -> TextTarget {
    engine
        .selection_target(Selection {
            secondary: Caret {
                index: range.start,
                ..Default::default()
            },
            primary: Caret {
                index: range.end,
                ..Default::default()
            },
        })
        .unwrap()
        .into()
}

#[test]
fn catalogue_has_valid_unique_ids_and_strict_schemas_and_all_calls_round_trip() {
    assert_eq!(catalogue().len(), 28);
    for spec in catalogue() {
        assert!(!spec.description.trim().is_empty());
        assert_eq!(spec.params_schema["type"], "object");
        assert_eq!(spec.params_schema["additionalProperties"], false);
        let params = match spec.id.as_str() {
            "text.insert" => json!({"text":"Été"}),
            "text.link" => json!({"url":"https://example.org"}),
            "search.find" => json!({"query":"été"}),
            _ => json!({}),
        };
        if spec.family == Family::Query {
            let query = Query::from_call(spec.id.as_str(), params).unwrap();
            let call = query.to_call();
            assert_eq!(
                Query::from_call(call.id.as_str(), call.params).unwrap(),
                query
            );
        } else {
            let command = Command::from_call(spec.id.as_str(), params).unwrap();
            let call = command.to_call();
            assert_eq!(
                Command::from_call(call.id.as_str(), call.params).unwrap(),
                command
            );
        }
    }
    for id in [
        "text",
        "text.Bold",
        "text..bold",
        "letter.insert-postscript",
        "text.é",
        "_bad.id",
    ] {
        assert!(CommandId::try_from(id.to_owned()).is_err(), "{id}");
    }
}

#[test]
fn generated_catalogues_match_the_registry() {
    let e = engine("");
    let value = e.query(Query::Commands(Empty {})).unwrap();
    assert_eq!(
        registry::catalogue_markdown(&value),
        include_str!("../../docs/commands.md").replace("\r\n", "\n")
    );
    let checked: Value = serde_json::from_str(include_str!("../../docs/commands.json")).unwrap();
    assert_eq!(value, checked);
}

#[test]
fn calls_reject_unknown_ids_fields_missing_arguments_and_invalid_parameter_types() {
    for (id, params) in [
        ("text.insert", json!({})),
        ("text.insert", json!({"text":1})),
        ("text.insert", json!({"text":"x", "input_rules":"typing"})),
        ("text.bold", json!({"value":true})),
        ("text.bold", json!({"target":{"kind":"current","offset":3}})),
        ("edit.undo", json!({"text":"x"})),
        ("text.select", json!({"target":{"kind":"selection"}})),
        ("text.select", json!({"part":"caption"})),
        ("block.exit", json!({"replace":true})),
    ] {
        assert_eq!(
            Command::from_call(id, params).unwrap_err().code,
            ErrorCode::InvalidParams
        );
    }
    assert_eq!(
        Command::from_call("document.new", json!({}))
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert_eq!(
        Query::from_call("text.bold", json!({})).unwrap_err().code,
        ErrorCode::NotFound
    );
}

#[test]
fn json_commands_keep_literal_unicode_styles_selections_and_one_step_history() {
    let mut e = engine("Avant après");
    let target = explicit(&e, 0..5);
    call(&mut e, "text.select", json!({"target":target})).unwrap();
    assert!(e.undo.is_empty());
    call(&mut e, "text.bold", json!({"value":"on"})).unwrap();
    assert_eq!(e.undo.len(), 1);
    let unchanged = call(&mut e, "text.bold", json!({"value":"on"})).unwrap();
    assert!(!unchanged.changed && unchanged.effects.is_empty());
    let outcome = call(&mut e, "text.insert", json!({"text":"にほん _été_"})).unwrap();
    assert!(outcome.changed);
    assert_eq!(outcome.effects, [Effect::Render]);
    assert_eq!(e.document.view.text, "にほん _été_ après");
    assert!(
        e.document
            .view
            .styles
            .iter()
            .take(9)
            .all(|style| style.bold && !style.italic)
    );
    assert!(e.selection().range().is_empty());
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.document.view.text, "Avant après");
    assert_eq!(e.selection().range(), 0..5);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(e.document.view.text, "にほん _été_ après");
}

#[test]
fn typing_context_changes_do_not_dirty_the_document_or_advance_history() {
    let mut e = engine("");
    let outcome = call(&mut e, "text.bold", json!({})).unwrap();
    assert!(!outcome.changed);
    assert_eq!(e.revision, 0);
    assert!(e.undo.is_empty() && !e.is_dirty());
    call(&mut e, "text.insert", json!({"text":"Été"})).unwrap();
    assert_eq!(
        e.document.view.styles,
        vec![
            InlineStyle {
                bold: true,
                ..Default::default()
            };
            3
        ]
    );
    assert_eq!(e.undo.len(), 1);
}

#[test]
fn on_and_off_are_idempotent_even_on_mixed_styles() {
    let mut e = engine("Un *été*");
    call(&mut e, "text.select", json!({"target":{"kind":"all"}})).unwrap();
    call(&mut e, "text.bold", json!({"value":"off"})).unwrap();
    assert!(e.document.view.styles.iter().all(|style| !style.bold));
    let revision = e.revision;
    call(&mut e, "text.bold", json!({"value":"off"})).unwrap();
    assert_eq!(e.revision, revision);
    call(&mut e, "text.italic", json!({"value":"on"})).unwrap();
    assert!(e.document.view.styles.iter().all(|style| style.italic));
}

#[test]
fn stale_and_foreign_targets_never_reanchor_after_edits_or_undo() {
    let mut e = engine("Avant après");
    let target = explicit(&e, 0..5);
    call(&mut e, "text.insert", json!({"text":"Un "})).unwrap();
    let state = e.stored_document();
    let selection = e.selection();
    let revision = e.revision;
    let error = call(&mut e, "text.bold", json!({"target":target})).unwrap_err();
    assert_eq!(error.reason(), "stale_revision");
    assert!(e.stored_document() == state);
    assert_eq!(e.selection(), selection);
    assert_eq!(e.revision, revision);
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(
        call(&mut e, "text.delete", json!({"target":target}))
            .unwrap_err()
            .reason(),
        "stale_revision"
    );
    let mut other = engine("Avant après");
    assert_eq!(
        call(&mut other, "text.delete", json!({"target":target}))
            .unwrap_err()
            .reason(),
        "foreign_document"
    );
}

#[test]
fn protected_boundaries_invalid_urls_and_out_of_bounds_positions_leave_everything_unchanged() {
    let mut e = engine("/ Mot: Définition");
    let target = explicit(&e, 2..6);
    let before = e.stored_document();
    let when = e.changed_at;
    assert_eq!(
        call(&mut e, "text.insert", json!({"target":target,"text":"X"}))
            .unwrap_err()
            .reason(),
        "protected_boundary"
    );
    let target = explicit(&e, 0..3);
    assert_eq!(
        call(
            &mut e,
            "text.link",
            json!({"target":target,"url":"javascript:alert(0)"})
        )
        .unwrap_err()
        .reason(),
        "invalid_url"
    );
    let mut target_json = serde_json::to_value(target).unwrap();
    target_json["selection"]["head"]["position"]["offset"] = json!(999);
    assert_eq!(
        call(&mut e, "text.select", json!({"target":target_json}))
            .unwrap_err()
            .reason(),
        "invalid_selection"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.revision, 0);
    assert_eq!(e.changed_at, when);
    assert!(e.undo.is_empty() && e.redo.is_empty());
}

#[test]
fn queries_return_revision_guarded_data_without_mutating_text_selection_or_history() {
    let mut e = engine("Été été\n\n/ Mot: Définition");
    let before = e.stored_document();
    let selection = e.selection();
    for (id, params) in [
        ("document.inspect", json!({})),
        ("document.text", json!({})),
        ("document.structure", json!({})),
        ("metadata.get", json!({})),
        ("commands.list", json!({})),
        ("search.find", json!({"query":"ÉTÉ"})),
    ] {
        let value = e
            .query_call(Query::from_call(id, params).unwrap().to_call())
            .unwrap();
        if id == "search.find" {
            assert_eq!(value["matches"].as_array().unwrap().len(), 2);
            let target: SelectionTarget =
                serde_json::from_value(value["matches"][0]["target"].clone()).unwrap();
            assert_eq!(e.resolve_target(target).unwrap(), (0, 3));
        }
    }
    assert!(e.stored_document() == before);
    assert_eq!(e.selection(), selection);
    assert_eq!(e.revision, 0);
    assert!(e.undo.is_empty() && e.redo.is_empty());
    assert!(matches!(
        e.available(&Command::Undo(Empty {})),
        Availability::Disabled { .. }
    ));
    // Une requête ne doit pas casser le regroupement d'une frappe native.
    e.typing_group = true;
    e.query(Query::Inspect(Empty {})).unwrap();
    assert!(e.typing_group);
}

#[test]
fn letter_commands_enforce_document_type_and_postscript_reuse_does_not_add_history() {
    let mut e = engine("Bonjour");
    call(&mut e, "letter.insert_postscript", json!({})).unwrap();
    assert_eq!(e.undo.len(), 1);
    let target = e.selection();
    call(&mut e, "letter.insert_postscript", json!({})).unwrap();
    assert_eq!(e.undo.len(), 1);
    assert_eq!(e.selection(), target);
    e.metadata = Metadata::blank(DocumentType::CompteRendu);
    assert_eq!(
        call(&mut e, "letter.insert_attachment", json!({}))
            .unwrap_err()
            .reason(),
        "document_type"
    );
}
