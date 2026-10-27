//! Présentation localisée des registres du cœur ; aucune liste parallèle de capacités.
use serde_json::{Value, json};
use typstographe_core::{
    content::catalogue,
    document::{DocumentLanguage, PaperFormat},
    document_types::{DocumentType, Metadata, models::Model},
};
use typstographe_i18n::text;

fn document_type(kind: DocumentType) -> Value {
    let metadata = Metadata::blank(kind);
    let fields: Vec<_> = metadata
        .field_values()
        .into_iter()
        .map(|field| {
            let mut value = serde_json::to_value(&field).expect("metadata field");
            value["parameter"] = json!(field.parameter);
            value["label"] = json!(text(field.label_key));
            value
        })
        .collect();
    let models: Vec<_> = kind
        .models()
        .map(|model| {
            let label = match model.model {
                Model::Personal => text("model-personal-letter"),
                Model::ArticleWorking => text("model-working-article"),
            };
            let page = model
                .page
                .map(|(paper, landscape)| json!({"paper":paper,"landscape":landscape}));
            json!({
                "id":model.model.id(), "label":label, "description":model.description,
                "keywords":model.keywords, "source":model.source, "fixedPage":page,
                "policy":typstographe_core::rendering::TypstWorld::default().model_presentation(model.source).expect("bundled model policy")
            })
        })
        .collect();
    let heading = if kind == DocumentType::Article {
        "article-settings-heading"
    } else if kind == DocumentType::Courrier {
        "document-settings-heading"
    } else {
        "minutes-settings-heading"
    };
    let groups: Vec<_> = metadata
        .editor_groups()
        .iter()
        .map(|group| {
            let sections: Vec<_> = group.sections.iter().map(|section| json!({
                "id": section.id, "label": text(section.label_key), "fields": section.fields,
                "layout": section.layout
            })).collect();
            json!({"id":group.id,"label":text(group.label_key),"sections":sections})
        })
        .collect();
    let mut structured = kind.structured_metadata();
    if let Some(schema) = structured.as_mut()
        && let Some(collections) = schema["collections"].as_array_mut()
    {
        for collection in collections {
            for (key, localized) in [("label_key", "label"), ("item_label_key", "itemLabel")] {
                collection[localized] = json!(text(collection[key].as_str().unwrap()));
            }
            for field in collection["fields"].as_array_mut().unwrap() {
                field["label"] = json!(text(field["label_key"].as_str().unwrap()));
            }
        }
    }
    let capabilities: Vec<_> = catalogue::for_type(kind)
        .map(|capability| capability.id)
        .collect();
    json!({
        "capabilities":capabilities,
        "id":kind,"label":text(kind.label_key()),"settingsHeading":text(heading),
        "structured":structured,"canCreate":kind.can_create(),"metadata":metadata,"fields":fields,"groups":groups,"models":models
    })
}
#[tauri::command]
pub fn content_catalogue() -> Value {
    let types: Vec<_> = DocumentType::ALL.into_iter().map(document_type).collect();
    let entries: Vec<_> = catalogue::ALL
        .iter()
        .map(|capability| {
            let mut value = serde_json::to_value(capability).expect("capability");
            value["label"] = json!(text(capability.label_key));
            value["parameters"] = catalogue::parameters(capability.id);
            if let Some(properties) = value["parameters"]["properties"].as_object_mut() {
                for parameter in properties.values_mut() {
                    if let Some(key) = parameter["x-editor"]["label_key"].as_str() {
                        parameter["title"] = json!(text(key));
                    }
                }
            }
            value["editor"] = catalogue::editor(capability.id);
            value
        })
        .collect();
    let papers =
        PaperFormat::ALL.map(|paper| json!({"value":paper,"label":text(paper.label_key())}));
    let languages = DocumentLanguage::ALL
        .map(|language| json!({"value":language,"label":text(language.label_key())}));
    json!({
        "types":types,"entries":entries,"messages":typstographe_i18n::web_messages(),
        "locale":typstographe_i18n::language(),"papers":papers,"languages":languages
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn editorial_sections_keep_localized_labels_and_canonical_field_bindings() {
        let letter = document_type(DocumentType::Courrier);
        let groups = letter["groups"].as_array().unwrap();
        assert_eq!(
            groups
                .iter()
                .map(|group| group["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["sender", "header"]
        );
        assert_eq!(groups[1]["label"], text("group-editorial-header"));
        let sections = groups[1]["sections"].as_array().unwrap();
        assert_eq!(sections[0]["label"], text("group-recipient"));
        assert_eq!(
            sections[0]["fields"],
            json!(["recipient.name", "recipient.address"])
        );
        assert_eq!(
            sections[1]["fields"],
            json!([
                "letter.subject",
                "letter.reference",
                "letter.place",
                "letter.date"
            ])
        );
        assert!(
            sections
                .iter()
                .all(|section| section["layout"] == "compact")
        );
        let sender = groups[0]["sections"].as_array().unwrap();
        assert_eq!(sender[0]["layout"], "inline");
        assert_eq!(sender[0]["fields"], json!(["sender.name"]));
        assert_eq!(sender[1]["label"], text("group-contact-details"));
        assert_eq!(sender[1]["layout"], "compact");
        assert_eq!(
            sender[1]["fields"],
            json!(["sender.address", "sender.email", "sender.phone"])
        );
        assert_eq!(sender[2]["label"], text("group-professional-details"));
        assert_eq!(
            sender[2]["fields"],
            json!(["sender.organisation", "sender.department", "sender.role"])
        );
        for section in groups
            .iter()
            .flat_map(|group| group["sections"].as_array().unwrap())
        {
            for id in section["fields"].as_array().unwrap() {
                assert!(
                    letter["fields"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|field| field["field"] == *id)
                );
            }
        }
    }

    #[test]
    fn localized_fields_preserve_shared_parameters_and_bindings() {
        let catalogue = content_catalogue();
        for entry in catalogue["entries"].as_array().unwrap() {
            let id = entry["id"].as_str().unwrap();
            let mut localized = entry["parameters"].clone();
            for parameter in localized["properties"]
                .as_object_mut()
                .unwrap()
                .values_mut()
            {
                if let Some(key) = parameter["x-editor"]["label_key"].as_str() {
                    assert_eq!(parameter["title"], text(key));
                    parameter.as_object_mut().unwrap().remove("title");
                }
            }
            assert_eq!(localized, catalogue::parameters(id));
            assert_eq!(entry["editor"], catalogue::editor(id));
        }
    }
}
