use super::*;
use crate::{
    document::DocumentLanguage,
    document_types::{DocumentType, Metadata},
};
use serde_json::{Value, json};

fn sample() -> (LoadedDefinition, Value) {
    let source = include_str!("../../../../../docs/proposals/b1/article-authorship.yaml")
        .replace("id: article", "id: custom-document");
    let loaded = load_yaml(&source).unwrap();
    assert_eq!(loaded.definition().id, "custom-document");
    let mut author = loaded.record_defaults("authors").unwrap();
    author.extend([
        (String::from("id"), json!("alice")),
        ("name".into(), json!("Alice")),
        ("affiliation-ids".into(), json!(["lab"])),
    ]);
    let values = json!({"authors":[author],"affiliations":[{"id":"lab","text":"Univ Rennes, CNRS, IRMAR - UMR 6625"}]});
    (loaded, values)
}
fn has(loaded: &LoadedDefinition, values: &Value, code: DiagnosticCode, path: &str) {
    let issues = loaded.validate_values(values).unwrap_err();
    assert!(
        issues.iter().any(|d| d.code == code && d.path == path),
        "{issues:?}"
    );
}
#[test]
fn relations_are_validated_for_a_new_type_without_article_specific_rules() {
    let (loaded, values) = sample();
    loaded.validate_values(&values).unwrap();
    let mut missing = values.clone();
    missing["affiliations"] = json!([]);
    has(
        &loaded,
        &missing,
        DiagnosticCode::UnresolvedReference,
        "/authors/0/affiliation-ids",
    );
    let mut duplicate = values.clone();
    duplicate["authors"][0]["affiliation-ids"] = json!(["lab", "lab"]);
    has(
        &loaded,
        &duplicate,
        DiagnosticCode::InvalidValue,
        "/authors/0/affiliation-ids",
    );
    missing["authors"][0]["affiliation-ids"] = json!([]);
    loaded.validate_values(&missing).unwrap();
}
#[test]
fn incomplete_records_are_saved_as_drafts_but_fail_export_requirements() {
    let (loaded, mut values) = sample();
    values["authors"][0]["name"] = json!("");
    loaded.validate_values(&values).unwrap();
    let export = loaded.export_diagnostics(&values, DocumentLanguage::French);
    assert!(
        export
            .iter()
            .any(|d| d.path == "/authors/0/name" && d.code == DiagnosticCode::MissingValue)
    );
    values["authors"][0]["name"] = json!("Alice");
    assert!(
        loaded
            .export_diagnostics(&values, DocumentLanguage::French)
            .is_empty()
    );
}
#[test]
fn malformed_values_unknown_fields_and_duplicate_ids_are_rejected_without_projection() {
    let (loaded, values) = sample();
    has(&loaded, &json!(null), DiagnosticCode::InvalidValue, "");
    for (path, value, code) in [
        ("/authors/0/name", json!(17), DiagnosticCode::InvalidValue),
        (
            "/authors/0/email",
            json!("x\u{0001}"),
            DiagnosticCode::InvalidValue,
        ),
        (
            "/authors/0/corresponding",
            json!("false"),
            DiagnosticCode::InvalidValue,
        ),
        ("/authors/0/id", json!(""), DiagnosticCode::DuplicateValue),
    ] {
        let mut changed = values.clone();
        *changed.pointer_mut(path).unwrap() = value;
        has(&loaded, &changed, code, path);
    }
    let mut extra = values.clone();
    extra["unknown"] = json!(true);
    has(&loaded, &extra, DiagnosticCode::InvalidValue, "/unknown");
    let mut extra = values.clone();
    extra["authors"][0]["unknown"] = json!(true);
    has(
        &loaded,
        &extra,
        DiagnosticCode::InvalidValue,
        "/authors/0/unknown",
    );
    let mut missing = values.clone();
    missing["authors"][0]
        .as_object_mut()
        .unwrap()
        .remove("email");
    has(
        &loaded,
        &missing,
        DiagnosticCode::MissingValue,
        "/authors/0/email",
    );
    let mut duplicates = values.clone();
    duplicates["authors"]
        .as_array_mut()
        .unwrap()
        .push(values["authors"][0].clone());
    has(
        &loaded,
        &duplicates,
        DiagnosticCode::DuplicateValue,
        "/authors/1/id",
    );
}
#[test]
fn active_article_requires_the_title_in_the_primary_language_only() {
    let loaded = DocumentType::Article.definition();
    let raw = serde_json::to_value(Metadata::blank(DocumentType::Article)).unwrap();
    let mut values = super::super::runtime::values(DocumentType::Article, &raw["fields"]);
    let mut en = loaded.record_defaults("localizations").unwrap();
    en.extend([("id".into(),json!("english")),("language".into(),json!("en")),("title".into(),json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"A title"}]}]}))]);
    values["localizations"]
        .as_array_mut()
        .unwrap()
        .push(json!(en));
    loaded.validate_values(&values).unwrap();
    assert!(
        !loaded
            .export_diagnostics(&values, DocumentLanguage::French)
            .is_empty()
    );
    assert!(
        loaded
            .export_diagnostics(&values, DocumentLanguage::English)
            .is_empty()
    );
    values["localizations"][1]["language"] = json!("fr");
    has(
        loaded,
        &values,
        DiagnosticCode::DuplicateValue,
        "/localizations/1",
    );
}
#[test]
fn reference_files_use_the_registered_bibliography_validator() {
    let loaded = DocumentType::Article.definition();
    let raw = serde_json::to_value(Metadata::blank(DocumentType::Article)).unwrap();
    let mut values = super::super::runtime::values(DocumentType::Article, &raw["fields"]);
    values["references"] = json!([crate::bibliography::test_record(
        "books",
        "books.bib",
        "@book{example,title={A book},author={Doe, Jane},year={2020}}"
    )]);
    loaded.validate_values(&values).unwrap();
    let file = values["references"][0].clone();
    values["references"].as_array_mut().unwrap().push(file);
    has(loaded, &values, DiagnosticCode::InvalidValue, "/references");
    values["references"] = json!([{"id":"books","name":"books.bib","contents":"not BibLaTeX @"}]);
    has(loaded, &values, DiagnosticCode::InvalidValue, "/references");
}
#[test]
fn registered_views_controls_and_storage_bindings_cannot_redefine_business_structures() {
    let source = include_str!("../../../types/article/type.yaml");
    let base: Value =
        serde_json::to_value(yaml_serde::from_str::<yaml_serde::Value>(source).unwrap()).unwrap();
    for change in [
        "library-fields",
        "missing-style",
        "wrong-outline",
        "storage-collision",
    ] {
        let mut definition = base.clone();
        match change {
            "library-fields" => {
                definition["collections"][0]["fields"] =
                    json!([{"kind":"text","id":"script","label-key":"script"}])
            }
            "missing-style" => {
                definition["header"]["tabs"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|tab| tab["view"] == "bibliography")
                    .unwrap()["style-field"] = json!("missing");
            }
            "wrong-outline" => definition["fields"][0]["control"] = json!("outline"),
            "storage-collision" => definition["bindings"][1]["metadata-key"] = json!("authors"),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                load_yaml(&definition.to_string()),
                Err(LoadError::InvalidDefinition { .. })
            ),
            "{change}"
        );
    }
}

#[test]
fn structured_dates_keep_absence_drafts_and_calendar_validation_separate_from_export() {
    let source = include_str!("../../../types/courrier/type.yaml").replace(
        "id: letter.date\n  kind: date",
        "id: letter.date\n  kind: date\n  required-for-export: true",
    );
    let loaded = load_yaml(&source).unwrap();
    let mut values = Value::Object(loaded.defaults());
    assert!(values["letter.date"].is_null());
    loaded.validate_values(&values).unwrap();
    assert!(
        loaded
            .export_diagnostics(&values, DocumentLanguage::French)
            .iter()
            .any(|d| d.path == "/letter.date")
    );
    let schema = loaded.values_schema();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for date in [
        json!(null),
        json!({"kind":"calendar","iso":"2024-02-29"}),
        json!({"kind":"custom","text":"2026-02-30"}),
    ] {
        values["letter.date"] = date;
        loaded.validate_values(&values).unwrap();
        assert!(validator.is_valid(&values));
    }
    for date in [
        json!("2026-01-01"),
        json!({"kind":"calendar","iso":"2025-02-29"}),
        json!({"kind":"calendar","iso":"today"}),
        json!({"kind":"custom","text":"  "}),
        json!({"kind":"custom","text":"deux\nlignes"}),
        json!({"kind":"custom","text":"date","iso":"2026-11-03"}),
    ] {
        values["letter.date"] = date;
        has(
            &loaded,
            &values,
            DiagnosticCode::InvalidValue,
            "/letter.date",
        );
    }
    // Invalid calendar defaults fail at load time, not when compiling a document.
    assert!(
        load_yaml(&source.replace(
            "kind: date",
            "kind: date\n  default: {kind: calendar, iso: '2025-02-29'}"
        ))
        .is_err()
    );
    assert!(load_yaml(&source.replace("kind: date", "kind: text\n  control: date")).is_err());
}
