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
#[path = "tests/availability.rs"]
mod availability;
#[path = "tests/batch.rs"]
mod batch;
#[path = "tests/blocks.rs"]
mod blocks;
#[path = "tests/discovery.rs"]
mod discovery;
#[path = "tests/files.rs"]
mod files;
#[path = "tests/images.rs"]
mod images;
#[path = "tests/metadata.rs"]
mod metadata;
#[path = "tests/models.rs"]
mod models;
#[path = "tests/native.rs"]
mod native;
#[path = "tests/render.rs"]
mod render;
#[path = "tests/search.rs"]
mod search;
#[path = "tests/spelling.rs"]
mod spelling;
#[path = "tests/statistics.rs"]
mod statistics;
#[path = "tests/validation.rs"]
mod validation;

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
    assert_eq!(catalogue().len(), 68);
    for spec in catalogue() {
        assert!(!spec.description.trim().is_empty());
        assert_eq!(spec.params_schema["type"], "object");
        assert_eq!(spec.params_schema["additionalProperties"], false);
        let params = match spec.id.as_str() {
            "document.validate" | "render.compile" => {
                json!({"guard":{"document_id":1,"revision":0}})
            }
            "render.locate" => {
                json!({"render":{"guard":{"document_id":1,"revision":0},"id":1},"position":{"direction":"to_preview"}})
            }
            "render.selection_boxes" => {
                json!({"render":{"guard":{"document_id":1,"revision":0},"id":1}})
            }
            "document.batch" => {
                json!({"guard":{"document_id":1,"revision":0},"steps":[{"id":"text.bold","params":{}}]})
            }
            "recovery.restore" => json!({"guard":{"document_id":1,"revision":0},"reference":1}),
            "recovery.delete" => json!({"reference":1}),
            "document.new" => {
                json!({"guard":{"document_id":1,"revision":0},"type":"courrier","model":"courrier.model.personal"})
            }
            "document.open" => {
                json!({"guard":{"document_id":1,"revision":0},"path":"example.typsto"})
            }
            "document.close" | "document.save" => json!({"guard":{"document_id":1,"revision":0}}),
            "document.save_as" => {
                json!({"guard":{"document_id":1,"revision":0},"path":"example.typsto"})
            }
            "document.export" => {
                json!({"guard":{"document_id":1,"revision":0},"directory":"export"})
            }
            "text.insert" => json!({"text":"Été"}),
            "insert.image" => json!({"image":{"kind":"path","path":"image.svg"}}),
            "image.replace" => {
                json!({"target":{"guard":{"document_id":1,"revision":0},"block_id":1},"image":{"kind":"path","path":"image.svg"}})
            }
            "image.remove" => {
                json!({"target":{"guard":{"document_id":1,"revision":0},"block_id":1}})
            }
            "image.set_alt" => {
                json!({"target":{"guard":{"document_id":1,"revision":0},"block_id":1},"alt":"Trees"})
            }
            "document.set_signature" => {
                json!({"guard":{"document_id":1,"revision":0},"image":null})
            }
            "text.link" => json!({"url":"https://example.org"}),
            "document.set_model" => {
                json!({"guard":{"document_id":1,"revision":0},"model":{"kind":"source","source":"#let courrier(..args) = []"}})
            }
            "courrier.model.personal" => json!({"guard":{"document_id":1,"revision":0}}),
            "metadata.set" => {
                json!({"guard":{"document_id":1,"revision":0},"field":"letter.subject","value":"Été"})
            }
            "document.set_option" => {
                json!({"guard":{"document_id":1,"revision":0},"option":{"key":"language","value":"en"}})
            }
            "spelling.ignore" | "spelling.correct" => {
                let e = engine("courrrier");
                let issue = e.query(Query::SpellingIssues(Empty {})).unwrap()["issues"][0].clone();
                if spec.id.as_str() == "spelling.correct" {
                    json!({"issue":issue,"replacement":"courrier"})
                } else {
                    json!({"issue":issue})
                }
            }
            "spelling.reset" => json!({"guard":{"document_id":1,"revision":0}}),
            "spelling.suggestions" => {
                json!({"target":{"kind":"word","language":"fr","word":"courrrier"}})
            }
            "search.find" | "edit.find_next" | "edit.find_previous" => json!({"query":"été"}),
            "edit.replace_current" => {
                let e = engine("Été");
                let occurrence = e
                    .query(Query::Find(SearchParams {
                        query: "été".into(),
                        case_sensitive: false,
                    }))
                    .unwrap()["matches"][0]
                    .clone();
                json!({"query":"été", "occurrence":occurrence, "replacement":"Noël"})
            }
            "edit.replace_all" => {
                json!({"guard":{"document_id":1,"revision":0},"query":"été","replacement":"Noël"})
            }
            _ => json!({}),
        };
        if matches!(
            spec.family,
            Family::File | Family::History | Family::Query | Family::Batch
        ) {
            assert!(
                !spec.batch_step,
                "{} must be excluded from editing steps",
                spec.id.as_str()
            );
        }
        if spec.family == Family::Query {
            let query = Query::from_call(spec.id.as_str(), params).unwrap();
            let call = query.to_call();
            assert_eq!(
                Query::from_call(call.id.as_str(), call.params).unwrap(),
                query
            );
        } else {
            let command = Command::from_call(spec.id.as_str(), params).unwrap();
            assert_eq!(command.batch_step_allowed(), spec.batch_step);
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
    let value = registry::catalogue_json();
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
        ("text.insert", json!({"text":"x", "input_rules":"execute"})),
        ("text.bold", json!({"value":true})),
        ("text.bold", json!({"target":{"kind":"current","offset":3}})),
        ("edit.undo", json!({"text":"x"})),
        ("text.select", json!({"target":{"kind":"selection"}})),
        ("text.select", json!({"part":"caption"})),
        ("block.exit", json!({"replace":true})),
        ("image.remove", json!({"image_id":1})),
        ("image.set_alt", json!({"target":{"block_id":1},"alt":"x"})),
        ("document.set_signature", json!({"image":null})),
    ] {
        assert_eq!(
            Command::from_call(id, params).unwrap_err().code,
            ErrorCode::InvalidParams
        );
    }
    assert_eq!(
        Command::from_call("document.unknown", json!({}))
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
    assert!(e.session.undo.is_empty());
    call(&mut e, "text.bold", json!({"value":"on"})).unwrap();
    assert_eq!(e.session.undo.len(), 1);
    let unchanged = call(&mut e, "text.bold", json!({"value":"on"})).unwrap();
    assert!(!unchanged.changed && unchanged.effects.is_empty());
    let outcome = call(&mut e, "text.insert", json!({"text":"にほん _été_"})).unwrap();
    assert!(outcome.changed);
    assert_eq!(outcome.effects, [Effect::Render]);
    assert_eq!(e.session.document.view.text, "にほん _été_ après");
    assert!(
        e.session
            .document
            .view
            .styles
            .iter()
            .take(9)
            .all(|style| style.bold && !style.italic)
    );
    assert!(e.selection().range().is_empty());
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.session.document.view.text, "Avant après");
    assert_eq!(e.selection().range(), 0..5);
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(e.session.document.view.text, "にほん _été_ après");
}

#[test]
fn typing_context_changes_do_not_dirty_the_document_or_advance_history() {
    let mut e = engine("");
    let outcome = call(&mut e, "text.bold", json!({})).unwrap();
    assert!(!outcome.changed);
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty() && !e.is_dirty());
    call(&mut e, "text.insert", json!({"text":"Été"})).unwrap();
    assert_eq!(
        e.session.document.view.styles,
        vec![
            InlineStyle {
                bold: true,
                ..Default::default()
            };
            3
        ]
    );
    assert_eq!(e.session.undo.len(), 1);
}

#[test]
fn on_and_off_are_idempotent_even_on_mixed_styles() {
    let mut e = engine("Un *été*");
    call(&mut e, "text.select", json!({"target":{"kind":"all"}})).unwrap();
    call(&mut e, "text.bold", json!({"value":"off"})).unwrap();
    assert!(
        e.session
            .document
            .view
            .styles
            .iter()
            .all(|style| !style.bold)
    );
    let revision = e.session.revision;
    call(&mut e, "text.bold", json!({"value":"off"})).unwrap();
    assert_eq!(e.session.revision, revision);
    call(&mut e, "text.italic", json!({"value":"on"})).unwrap();
    assert!(
        e.session
            .document
            .view
            .styles
            .iter()
            .all(|style| style.italic)
    );
}

#[test]
fn stale_and_foreign_targets_never_reanchor_after_edits_or_undo() {
    let mut e = engine("Avant après");
    let target = explicit(&e, 0..5);
    call(&mut e, "text.insert", json!({"text":"Un "})).unwrap();
    let state = e.stored_document();
    let selection = e.selection();
    let revision = e.session.revision;
    let error = call(&mut e, "text.bold", json!({"target":target})).unwrap_err();
    assert_eq!(error.reason(), "stale_revision");
    assert!(e.stored_document() == state);
    assert_eq!(e.selection(), selection);
    assert_eq!(e.session.revision, revision);
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
    let when = e.session.changed_at;
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
    assert_eq!(e.session.revision, 0);
    assert_eq!(e.session.changed_at, when);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
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
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    assert!(matches!(
        e.available(&Command::Undo(Empty {})),
        Availability::Disabled { .. }
    ));
    // Une requête ne doit pas casser le regroupement d'une frappe native.
    e.session.typing_group = true;
    e.query(Query::Inspect(Empty {})).unwrap();
    assert!(e.session.typing_group);
}

#[test]
fn letter_commands_enforce_document_type_and_postscript_reuse_does_not_add_history() {
    let mut e = engine("Bonjour");
    call(&mut e, "letter.insert_postscript", json!({})).unwrap();
    assert_eq!(e.session.undo.len(), 1);
    let target = e.selection();
    call(&mut e, "letter.insert_postscript", json!({})).unwrap();
    assert_eq!(e.session.undo.len(), 1);
    assert_eq!(e.selection(), target);
    e.session.metadata = Metadata::blank(DocumentType::CompteRendu);
    assert_eq!(
        call(&mut e, "letter.insert_attachment", json!({}))
            .unwrap_err()
            .reason(),
        "document_type"
    );
}
