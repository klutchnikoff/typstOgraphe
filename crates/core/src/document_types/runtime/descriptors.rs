use super::*;
use crate::document_types::{FieldSection, definition::Tab};

pub(super) fn build(loaded: &LoadedDefinition) -> (Vec<RootField>, Vec<FieldGroup>, Option<Value>) {
    let def = loaded.definition();
    let mut fields = Vec::new();
    for binding in &def.bindings {
        let (id, label, multiline, kind, required) = match binding {
            Binding::Field { field, .. } => {
                let f = def.fields.iter().find(|f| f.id() == field).unwrap();
                (
                    f.id().to_owned(),
                    f.label_key(),
                    matches!(
                        f,
                        Field::Text {
                            multiline: true,
                            ..
                        }
                    ),
                    if matches!(f, Field::ResourceReference { .. }) {
                        FieldKind::ResourceReference
                    } else if matches!(f, Field::Date { .. }) {
                        FieldKind::Date
                    } else if matches!(f, Field::Text { .. } | Field::Choice { .. }) {
                        FieldKind::Text
                    } else {
                        FieldKind::Records
                    },
                    f.required_for_export(),
                )
            }
            Binding::Collection { collection, .. } => {
                let c = def
                    .collections
                    .iter()
                    .find(|c| c.id == *collection)
                    .unwrap();
                (
                    format!("{}.{}", def.id, c.id),
                    c.label_key.as_str(),
                    false,
                    FieldKind::Records,
                    false,
                )
            }
        };
        let group = def
            .header
            .tabs
            .iter()
            .find(|tab| match tab {
                Tab::Form { sections, .. } => sections.iter().any(|s| s.fields.contains(&id)),
                Tab::RecordCards { collection, .. } => binding.source() == collection,
                Tab::Bibliography {
                    collection,
                    style_field,
                    ..
                } => binding.source() == collection || binding.source() == style_field,
            })
            .map_or("general", |tab| tab.id());
        fields.push(RootField {
            public: serde_json::from_value(json!(id)).expect("registered metadata identity"),
            resource: def
                .fields
                .iter()
                .find(|f| f.id() == binding.source())
                .and_then(|f| {
                    if let Field::ResourceReference {
                        resource_kind,
                        profile_default,
                        ..
                    } = f
                    {
                        Some(crate::resources::ResourceDescriptor {
                            kind: *resource_kind,
                            profile_default: *profile_default,
                        })
                    } else {
                        None
                    }
                }),
            presentation: def
                .fields
                .iter()
                .find(|field| field.id() == binding.source())
                .and_then(Field::presentation),
            parameter: binding.metadata_key().into(),
            label_key: label.into(),
            group: group.into(),
            multiline,
            kind,
            required_for_export: required,
        });
    }
    let groups = def
        .header
        .tabs
        .iter()
        .map(|tab| FieldGroup {
            id: tab.id().into(),
            label_key: tab.label_key().into(),
            sections: match tab {
                Tab::Form { sections, .. } => sections
                    .iter()
                    .map(|section| FieldSection {
                        id: section.id.clone(),
                        label_key: section.label_key.clone(),
                        layout: section.layout,
                        fields: section
                            .fields
                            .iter()
                            .map(|id| {
                                serde_json::from_value(json!(id))
                                    .expect("registered metadata field")
                            })
                            .collect(),
                    })
                    .collect(),
                _ => vec![],
            },
        })
        .collect();
    let structured = if def.collections.is_empty() {
        None
    } else {
        Some(structured(loaded))
    };
    (fields, groups, structured)
}

fn structured(loaded: &LoadedDefinition) -> Value {
    let def = loaded.definition();
    let values_schema = loaded.values_schema();
    let collections:Vec<_>=def.collections.iter().map(|c| {
        let binding=def.bindings.iter().find(|b|matches!(b,Binding::Collection {collection,..} if collection==&c.id)).unwrap();
        let fields:Vec<Value>= match c.kind {
            CollectionKind::BibliographicRecords => ["key","entry","retained_parents","provenance"].iter().map(|key|json!({"key":key,"label_key":"field-bibliography","kind":"native","default":if *key=="retained_parents" || *key=="provenance" {json!([])} else {Value::Null}})).collect(),
            CollectionKind::BibliographicSources => ["name","format","contents","digest"].iter().map(|key|json!({"key":key,"label_key":"field-reference-source","kind":"native","default":null})).collect(),
            _ => c.fields.iter().map(record_field).collect(),
        };
        let mut schema=values_schema["properties"][&c.id].clone();
        schema["$defs"]=values_schema["$defs"].clone();
        let mut value=json!({"field":format!("{}.{}",def.id,c.id),"parameter":binding.metadata_key(),
            "label_key":c.label_key,"item_label_key":c.item_label_key,"title_key":c.summary_field,
            "schema":schema,"fields":fields});
        if !c.summary.is_empty() {value["summary"]=json!(c.summary);}
        if let Some(key)=&c.language_key {value["language_key"]=json!(key);}
        if let Some(key)=&c.removal_notice_key {value["removal_notice_key"]=json!(key);}
        value
    }).collect();
    let panels: Vec<_> = def
        .header
        .tabs
        .iter()
        .filter_map(|tab| {
            let Tab::RecordCards {
                id,
                collection,
                summary_field,
                fields,
                editing,
                item_label_key,
                ..
            } = tab
            else {
                return None;
            };
            let c = def
                .collections
                .iter()
                .find(|c| c.id == *collection)
                .unwrap();
            let mut value = json!({"id":id,"collection":collection,"editing":editing});
            if !fields.is_empty() {
                value["fields"] = json!(fields);
            }
            if summary_field != &c.summary_field {
                value["summary"] = json!([summary_field]);
            }
            if let Some(key) = item_label_key {
                value["item_label_key"] = json!(key);
            }
            Some(value)
        })
        .collect();
    let mut result = json!({"collections":collections,"panels":panels});
    for tab in &def.header.tabs {
        if let Tab::Bibliography {
            id,
            collection,
            style_field,
            ..
        } = tab
        {
            let files = def
                .bindings
                .iter()
                .find(|b| b.source() == collection)
                .unwrap();
            let style = def
                .bindings
                .iter()
                .find(|b| b.source() == style_field)
                .unwrap();
            result["bibliography"] = json!({"tab":id,"files":files.metadata_key(),"field":format!("{}.{}",def.id,collection),"style":style.metadata_key(),"style_field":style_field,"sources":"reference_sources"});
        }
    }
    for field in &def.fields {
        if field.control() == Some(FieldControl::Outline) {
            let binding = def
                .bindings
                .iter()
                .find(|b| b.source() == field.id())
                .unwrap();
            result["outline"] = json!({"parameter":binding.metadata_key(),"field":field.id()});
        }
    }
    result
}
fn record_field(field: &Field) -> Value {
    let kind = match field {
        Field::Text {
            multiline: true, ..
        } => "multiline",
        Field::Text { .. } => "text",
        Field::ResourceReference { .. } => "resource-reference",
        Field::Date { .. } => "date",
        Field::Boolean { .. } => "boolean",
        Field::Choice { .. } => "choice",
        Field::Language { .. } => "language",
        Field::Tokens { .. } => "tokens",
        Field::RichText {
            profile: RichProfile::InlineText,
            ..
        } => "rich_inline",
        Field::RichText { .. } => "rich",
        Field::RecordReferences { .. } => "references",
    };
    let mut value = json!({"key":field.id(),"label_key":field.label_key(),"kind":kind,"default":field.default_value()});
    if let Field::RecordReferences {
        target_collection, ..
    } = field
    {
        value["references"] = json!(target_collection);
    }
    if let Field::Choice { choices, .. } = field {
        value["choices"] = json!(choices);
    }
    if field.statistics() {
        value["statistics"] = json!(true);
    }
    value
}
