use super::*;
use crate::{
    document::DocumentLanguage,
    document_types::definition::{LoadedDefinition, load_yaml},
};
use serde_json::{Value, json};

const ARTICLE: &str = include_str!("../../../../types/article/type.yaml");
fn raw() -> Value {
    serde_json::to_value(yaml_serde::from_str::<yaml_serde::Value>(ARTICLE).unwrap()).unwrap()
}
fn load(value: &Value) -> Result<LoadedDefinition, LoadError> {
    load_yaml(&value.to_string())
}
fn invalid(value: &Value, code: DiagnosticCode) {
    let Err(LoadError::InvalidDefinition { diagnostics }) = load(value) else {
        panic!("expected semantic error")
    };
    assert!(
        diagnostics.iter().any(|d| d.code == code),
        "{diagnostics:?}"
    );
}

#[test]
fn authorship_shortcut_resolves_to_the_exact_previous_article_contract() {
    let expected: Value = serde_json::from_str(include_str!(
        "../../../../../../fixtures/document-types/article-definition.json"
    ))
    .unwrap();
    let loaded = load_yaml(ARTICLE).unwrap();
    assert_eq!(json!(loaded.definition()), expected);
    assert!(loaded.definition().structures.is_empty());
    assert!(loaded.definition().order.is_empty());
    let source = yaml_serde::to_string(loaded.definition()).unwrap();
    assert_eq!(load_yaml(&source).unwrap(), loaded);
}

#[test]
fn one_registered_structure_provides_its_own_bindings_views_and_relations_to_another_type() {
    let loaded=load(&json!({
        "contract-version":1,"id":"research-project","label-key":"project","description-key":"project-description",
        "composition":{"function":"project"},"structures":["authorship"]
    })).unwrap();
    let def = loaded.definition();
    assert_eq!(
        def.collections
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["authors", "affiliations"]
    );
    assert_eq!(
        def.header.tabs.iter().map(Tab::id).collect::<Vec<_>>(),
        ["affiliations", "authors"]
    );
    let mut author = loaded.record_defaults("authors").unwrap();
    author.extend([
        ("id".into(), json!("author")),
        ("name".into(), json!("Camille")),
        ("affiliation-ids".into(), json!(["lab"])),
    ]);
    let mut values =
        json!({"authors":[author],"affiliations":[{"id":"lab","text":"Official affiliation"}]});
    loaded.validate_values(&values).unwrap();
    assert!(
        loaded
            .export_diagnostics(&values, DocumentLanguage::French)
            .is_empty()
    );
    let mut incomplete = values.clone();
    incomplete["authors"][0]["name"] = json!("");
    assert!(
        !loaded
            .export_diagnostics(&incomplete, DocumentLanguage::French)
            .is_empty()
    );
    values["affiliations"] = json!([]);
    assert!(
        loaded
            .validate_values(&values)
            .unwrap_err()
            .iter()
            .any(|d| d.code == DiagnosticCode::UnresolvedReference)
    );
    values["authors"][0]["affiliation-ids"] = json!([]);
    loaded.validate_values(&values).unwrap();
}

#[test]
fn structures_reject_unknown_names_repeat_use_and_parameterized_overrides() {
    for reference in [
        json!(["unknown"]),
        json!(["../authorship"]),
        json!([{"id":"authorship","max-items":1}]),
    ] {
        let mut value = raw();
        value["structures"] = reference;
        assert!(matches!(load(&value), Err(LoadError::Decode { .. })));
    }
    let mut value = raw();
    value["structures"] = json!(["authorship", "authorship"]);
    invalid(&value, DiagnosticCode::DuplicateIdentifier);
}

#[test]
fn local_collections_bindings_and_tabs_cannot_replace_shared_members() {
    let expanded = json!(load_yaml(ARTICLE).unwrap().definition());
    for target in ["collections", "bindings", "tabs"] {
        let mut value = raw();
        // Disable ordering to reach the normal duplicate-name validation.
        value.as_object_mut().unwrap().remove("order");
        match target {
            "collections" => value["collections"]
                .as_array_mut()
                .unwrap()
                .push(expanded["collections"][1].clone()),
            "bindings" => value["bindings"]
                .as_array_mut()
                .unwrap()
                .push(expanded["bindings"][0].clone()),
            "tabs" => value["header"]["tabs"]
                .as_array_mut()
                .unwrap()
                .push(expanded["header"]["tabs"][2].clone()),
            _ => unreachable!(),
        }
        invalid(&value, DiagnosticCode::DuplicateIdentifier);
    }
}

#[test]
fn explicit_orders_are_complete_permutations_and_cannot_hide_content() {
    for names in [
        json!(["authors"]),
        json!(["authors", "affiliations", "references", "missing"]),
        json!(["authors", "authors", "references", "localizations"]),
    ] {
        let mut value = raw();
        value["order"]["collections"] = names;
        invalid(&value, DiagnosticCode::InvalidOrder);
    }
    let mut value = raw();
    value["order"]["tabs"] = json!(["authors"]);
    invalid(&value, DiagnosticCode::InvalidOrder);
    let mut value = raw();
    value["order"]["bindings"] = json!(["authors"]);
    invalid(&value, DiagnosticCode::InvalidOrder);
    let mut value = raw();
    value["order"]["tabs"] = json!([
        "authors",
        "affiliations",
        "titles",
        "abstracts",
        "bibliography"
    ]);
    value["order"]["bindings"] = json!([
        "affiliations",
        "authors",
        "localizations",
        "references",
        "reference_sources",
        "article.citation_style",
        "article.show_outline"
    ]);
    let loaded = load(&value).unwrap();
    assert_eq!(loaded.definition().header.tabs[0].id(), "authors");
    assert_eq!(loaded.definition().bindings[0].source(), "affiliations");
}

#[test]
fn fragments_use_the_same_strict_yaml_decoder_and_cannot_import_other_fragments() {
    let fragment = include_str!("../../../../structures/authorship.yaml");
    let crlf = fragment.replace("\r\n", "\n").replace('\n', "\r\n");
    let decoded: Fragment = crate::document_types::definition::loader::decode_yaml(&crlf).unwrap();
    assert_eq!(decoded.id, HeaderStructure::Authorship);
    assert!(
        crate::document_types::definition::loader::decode_yaml::<Fragment>(&format!(
            "{fragment}\nstructures: [authorship]\n"
        ))
        .is_err()
    );
    assert!(
        crate::document_types::definition::loader::decode_yaml::<Fragment>(&format!(
            "{fragment}\nid: authorship\n"
        ))
        .is_err()
    );
}
