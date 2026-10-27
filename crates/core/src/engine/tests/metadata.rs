use super::*;
use crate::{
    document::{DocumentLanguage, PaperFormat},
    document_types::MetadataField,
};

fn set(
    e: &mut Engine,
    field: MetadataField,
    value: &str,
    input: MetadataInput,
) -> Result<Outcome, CommandError> {
    call(
        e,
        "metadata.set",
        json!({"guard":e.target_guard(),"field":field,"value":value,"input":input}),
    )
}
fn option(e: &mut Engine, option: DocumentOption) -> Result<Outcome, CommandError> {
    call(
        e,
        "document.set_option",
        json!({"guard":e.target_guard(),"option":option}),
    )
}

#[test]
fn field_catalogues_drive_typed_updates_for_every_type_without_touching_the_body_or_resources() {
    for kind in DocumentType::ALL {
        let mut e = engine("Un *été* à _Noël_.");
        e.session.metadata = Metadata::blank(kind);
        let before = e.stored_document();
        let selection = e.selection();
        let read = e.query_call(Query::Metadata(Empty {}).to_call()).unwrap();
        assert_eq!(
            read["fields"].as_array().unwrap().len(),
            match kind {
                DocumentType::Courrier => 13,
                DocumentType::CompteRendu => 6,
                DocumentType::Article => 3,
            }
        );
        assert_eq!(e.session.revision, 0);
        assert!(e.session.undo.is_empty());
        if kind == DocumentType::Article {
            continue;
        } // structured fields have their own guarded editing recipe
        for field in e.session.metadata.field_values() {
            let value = if field.multiline {
                "Zoé\nÉlodie"
            } else {
                "Un texte #en [Typst]"
            };
            let result = set(&mut e, field.field, value, MetadataInput::Discrete).unwrap();
            assert!(result.changed);
            assert_eq!(result.effects, [Effect::Render]);
            assert_eq!(*e.session.metadata.field_mut(field.field).unwrap(), value);
            assert_eq!(e.selection(), selection);
            assert_eq!(e.session.metadata.document_type(), kind);
        }
        assert_eq!(e.session.document.source(), before.source);
        assert_eq!(e.session.document.assets, before.assets);
        assert_eq!(e.session.dependencies, before.dependencies);
        assert_eq!(e.session.template, before.template);
        assert!(e.session.metadata.ready_for_export());
        let len = e.session.undo.len();
        for _ in 0..len {
            call(&mut e, "edit.undo", json!({})).unwrap();
        }
        assert!(e.stored_document() == before);
    }
}

#[test]
fn field_typing_groups_by_identity_and_explicit_discrete_boundaries() {
    let mut e = engine("Corps");
    for value in ["A", "AB"] {
        set(
            &mut e,
            MetadataField::LetterSubject,
            value,
            MetadataInput::Typing,
        )
        .unwrap();
    }
    assert_eq!(e.session.undo.len(), 1);
    set(
        &mut e,
        MetadataField::LetterReference,
        "R",
        MetadataInput::Typing,
    )
    .unwrap();
    assert_eq!(e.session.undo.len(), 2);
    set(
        &mut e,
        MetadataField::LetterSubject,
        "ABC",
        MetadataInput::Discrete,
    )
    .unwrap();
    assert_eq!(e.session.undo.len(), 3);
    set(
        &mut e,
        MetadataField::LetterSubject,
        "ABCD",
        MetadataInput::Typing,
    )
    .unwrap();
    assert_eq!(e.session.undo.len(), 4);
    set(
        &mut e,
        MetadataField::LetterSubject,
        "ABCDE",
        MetadataInput::Discrete,
    )
    .unwrap();
    assert_eq!(e.session.undo.len(), 5);
    let revision = e.session.revision;
    assert!(
        !set(
            &mut e,
            MetadataField::LetterSubject,
            "ABCDE",
            MetadataInput::Typing
        )
        .unwrap()
        .changed
    );
    assert_eq!(e.session.revision, revision);
    assert_eq!(e.session.undo.len(), 5);
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.session.metadata.courrier().objet, "ABCD");
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(e.session.metadata.courrier().objet, "ABCDE");
}

#[test]
fn fixed_dates_are_normalized_without_an_implicit_clock_and_partial_or_free_labels_remain_editable()
{
    let mut e = engine("Bonjour");
    for (value, input, expected) in [
        ("1/2/2026", MetadataInput::Typing, "1/2/2026"),
        ("1/2/2026", MetadataInput::Discrete, "2026-02-01"),
        ("29/02/2024", MetadataInput::Typing, "2024-02-29"),
        (" 2026-10-21 ", MetadataInput::Discrete, "2026-10-21"),
        ("mars 2026", MetadataInput::Discrete, "mars 2026"),
        ("29/02/2025", MetadataInput::Discrete, "29/02/2025"),
        ("", MetadataInput::Discrete, ""),
    ] {
        set(&mut e, MetadataField::LetterDate, value, input).unwrap();
        assert_eq!(e.session.metadata.courrier().date, expected);
    }
    for value in ["today", " TODAY ", "aujourd'hui", "aujourd’hui"] {
        let before = e.stored_document();
        assert_eq!(
            set(
                &mut e,
                MetadataField::LetterDate,
                value,
                MetadataInput::Discrete
            )
            .unwrap_err()
            .reason(),
            "metadata_fixed_date"
        );
        assert!(e.stored_document() == before);
    }
    assert_eq!(e.session.document.source(), "Bonjour");
}

#[test]
fn invalid_fields_text_values_and_stale_or_foreign_guards_are_rejected_atomically() {
    let mut e = engine("Bonjour");
    let before = e.stored_document();
    assert_eq!(
        set(
            &mut e,
            MetadataField::MeetingTitle,
            "Réunion",
            MetadataInput::Discrete
        )
        .unwrap_err()
        .reason(),
        "metadata_field"
    );
    for value in ["Deux\nlignes", "a\tb", "a\0b", "\u{fffc}"] {
        assert_eq!(
            set(
                &mut e,
                MetadataField::LetterSubject,
                value,
                MetadataInput::Discrete
            )
            .unwrap_err()
            .reason(),
            "invalid_text"
        );
    }
    for (id, params) in [
        (
            "metadata.set",
            json!({"guard":e.target_guard(),"field":"signature","value":"images/a.svg"}),
        ),
        (
            "metadata.set",
            json!({"guard":e.target_guard(),"field":"letter.subject","value":42}),
        ),
        (
            "metadata.set",
            json!({"field":"letter.subject","value":"X"}),
        ),
        (
            "metadata.set",
            json!({"guard":e.target_guard(),"field":"letter.subject","value":"X","type":"compte-rendu"}),
        ),
        (
            "document.set_option",
            json!({"guard":e.target_guard(),"option":{"key":"landscape","value":"true"}}),
        ),
        (
            "document.set_option",
            json!({"guard":e.target_guard(),"option":{"key":"spelling_ignored","value":{}}}),
        ),
        (
            "document.set_option",
            json!({"guard":e.target_guard(),"option":{"key":"language","value":"zz"}}),
        ),
    ] {
        assert_eq!(
            call(&mut e, id, params).unwrap_err().code,
            ErrorCode::InvalidParams
        );
    }
    assert!(e.stored_document() == before);
    assert!(e.session.undo.is_empty());
    let guard = e.target_guard();
    let params = json!({"guard":guard,"field":"letter.subject","value":"X"});
    let mut other = engine("Bonjour");
    assert_eq!(
        call(&mut other, "metadata.set", params.clone())
            .unwrap_err()
            .reason(),
        "foreign_document"
    );
    set(
        &mut e,
        MetadataField::LetterSubject,
        "X",
        MetadataInput::Discrete,
    )
    .unwrap();
    call(&mut e, "edit.undo", json!({})).unwrap();
    let revision = e.session.revision;
    let when = e.session.changed_at;
    assert_eq!(
        call(&mut e, "metadata.set", params).unwrap_err().reason(),
        "stale_revision"
    );
    assert_eq!(
        call(
            &mut e,
            "document.set_option",
            json!({"guard":guard,"option":{"key":"language","value":"en"}})
        )
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    assert!(e.stored_document() == before);
    assert_eq!(e.session.revision, revision);
    assert_eq!(e.session.changed_at, when);
    assert_eq!(e.selection(), Selection::default());
    assert!(e.session.undo.is_empty());
    assert_eq!(e.session.redo.len(), 1);
}

#[test]
fn personal_letter_accepts_paper_and_orientation_without_ui_while_language_remains_independent() {
    let mut e = engine("Bonjour");
    let before = e.stored_document();
    for paper in PaperFormat::ALL {
        for landscape in [false, true] {
            option(&mut e, DocumentOption::Paper(paper)).unwrap();
            option(&mut e, DocumentOption::Landscape(landscape)).unwrap();
            assert_eq!(e.session.metadata.options().paper, paper);
            assert_eq!(e.session.metadata.options().landscape, landscape);
        }
    }
    option(&mut e, DocumentOption::Language(DocumentLanguage::English)).unwrap();
    assert_eq!(
        e.session.metadata.options().language,
        DocumentLanguage::English
    );
    assert_eq!(e.session.document.source(), before.source);
}

#[test]
fn bullet_choices_are_validated_against_the_actual_typst_model_not_ui_state() {
    let mut e = engine("- Bonjour");
    assert_eq!(
        option(&mut e, DocumentOption::BulletMarker(Some("▪".into())))
            .unwrap_err()
            .reason(),
        "bullet_marker_policy"
    );
    let before = e.session.snapshot(e.selection());
    e.session
        .template
        .push_str("\n#let editor-options = (bullet-markers: (\"▪\",),)");
    let selection = e.selection();
    e.session.remember(before, selection, false).unwrap();
    let read = e
        .query_call(Query::ModelPresentation(Empty {}).to_call())
        .unwrap();
    assert_eq!(read["bullet_markers"], json!(["▪"]));
    assert_eq!(read["custom_bullet_marker"], false);
    option(&mut e, DocumentOption::BulletMarker(Some("▪".into()))).unwrap();
    let before = e.stored_document();
    for value in ["", " ", "ab", "\n"] {
        assert_eq!(
            option(&mut e, DocumentOption::BulletMarker(Some(value.into())))
                .unwrap_err()
                .reason(),
            "bullet_marker_invalid"
        );
    }
    assert_eq!(
        option(&mut e, DocumentOption::BulletMarker(Some("◆".into())))
            .unwrap_err()
            .reason(),
        "bullet_marker_policy"
    );
    assert!(e.stored_document() == before);
    option(&mut e, DocumentOption::BulletMarker(None)).unwrap();
    let before = e.session.snapshot(e.selection());
    e.session
        .template
        .push_str("\n#let editor-options = (custom-bullet-marker: true,)");
    let selection = e.selection();
    e.session.remember(before, selection, false).unwrap();
    option(&mut e, DocumentOption::BulletMarker(Some("◆".into()))).unwrap();
    assert_eq!(
        e.session.metadata.options().bullet_marker.as_deref(),
        Some("◆")
    );
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert!(e.session.metadata.options().bullet_marker.is_none());
    call(&mut e, "edit.redo", json!({})).unwrap();
    assert_eq!(
        e.session.metadata.options().bullet_marker.as_deref(),
        Some("◆")
    );
}

#[test]
fn presentation_query_uses_document_dependencies_and_bad_policies_do_not_publish_options() {
    let mut e = engine("Bonjour");
    let before = e.stored_document();
    let read = e.query(Query::ModelPresentation(Empty {})).unwrap();
    assert!(read["fixed_page"].is_null());
    assert!(e.stored_document() == before);
    assert_eq!(e.session.revision, 0);
    assert!(e.session.undo.is_empty());
    e.session.template = "#let editor-options = (bullet-markers: (42,),)".into();
    let before = e.stored_document();
    // Nouvelle session : les mutations de modèles seront commandées au lot 2.4.7.
    let mut e: Engine = Session::new(before.clone(), e.session.document.clone(), None, 0).into();
    assert_eq!(
        e.query(Query::ModelPresentation(Empty {}))
            .unwrap_err()
            .reason(),
        "model_presentation"
    );
    assert_eq!(
        option(&mut e, DocumentOption::BulletMarker(Some("◆".into())))
            .unwrap_err()
            .reason(),
        "model_presentation"
    );
    assert!(e.stored_document() == before);
    assert!(e.session.undo.is_empty());
    // Le retour à la charte reste possible même si les choix du modèle sont invalides.
    assert!(
        !option(&mut e, DocumentOption::BulletMarker(None))
            .unwrap()
            .changed
    );
}

#[test]
fn metadata_options_and_normalized_multiline_fields_remain_autonomous_and_persisted() {
    let mut e = engine("Bonjour.");
    set(
        &mut e,
        MetadataField::SenderAddress,
        "Ligne 1\r\nLigne 2\rLigne 3",
        MetadataInput::Discrete,
    )
    .unwrap();
    set(
        &mut e,
        MetadataField::RecipientName,
        "Élodie",
        MetadataInput::Discrete,
    )
    .unwrap();
    set(
        &mut e,
        MetadataField::LetterSubject,
        "Réservation #set page()",
        MetadataInput::Discrete,
    )
    .unwrap();
    set(
        &mut e,
        MetadataField::LetterDate,
        "21/10/2026",
        MetadataInput::Discrete,
    )
    .unwrap();
    option(&mut e, DocumentOption::Language(DocumentLanguage::English)).unwrap();
    assert_eq!(
        e.session.metadata.courrier().adresse_expediteur,
        "Ligne 1\nLigne 2\nLigne 3"
    );
    assert_eq!(e.session.metadata.courrier().date, "2026-10-21");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metadata.typsto");
    let stored = e.stored_document();
    crate::storage::save(&path, &stored).unwrap();
    let reopened = crate::storage::read(&path).unwrap();
    assert!(reopened == stored);
    assert!(reopened.metadata.main_source().contains("language: \"en\""));
    assert_eq!(reopened.source, "Bonjour.");
}

#[test]
fn article_collections_are_guarded_typed_undoable_values_and_do_not_touch_the_body() {
    let mut e = engine("Le corps reste intact.");
    e.session.metadata = Metadata::blank(DocumentType::Article);
    e.session.template = DocumentType::Article.default_model();
    e.session.dependencies = crate::packages::for_template(&e.session.template);
    let original = e.stored_document();
    let affiliations = json!([{"id":"irmar","text":"Univ Rennes, CNRS, IRMAR - UMR 6625, F-35000 Rennes, France"}]);
    let guard = e.target_guard();
    call(
        &mut e,
        "metadata.set",
        json!({"guard":guard,"field":"article.affiliations","value":affiliations}),
    )
    .unwrap();
    assert!(
        call(
            &mut e,
            "metadata.set",
            json!({"guard":guard,"field":"article.authors","value":[]})
        )
        .is_err()
    );
    let author = json!({"id":"a1","name":"Camille","affiliation-ids":["irmar"],"email":"","orcid":"","corresponding":false});
    let guard = e.target_guard();
    call(
        &mut e,
        "metadata.set",
        json!({"guard":guard,"field":"article.authors","value":[author]}),
    )
    .unwrap();
    let guard = e.target_guard();
    assert!(
        call(
            &mut e,
            "metadata.set",
            json!({"guard":guard,"field":"article.affiliations","value":[]})
        )
        .is_err()
    );
    assert_eq!(e.document().source(), original.source);
    call(&mut e, "edit.undo", json!({})).unwrap();
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.session.metadata, original.metadata);
    assert!(
        Engine::type_documentation(DocumentType::Article)["structured_metadata"]["collections"]
            .is_array()
    );
}
