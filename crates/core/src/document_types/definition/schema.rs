use super::{Field, LoadedDefinition};
use serde_json::{Map, Value, json};

/// Generated from the same Serde contract as the YAML loader.
pub fn manifest_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(super::Definition)).expect("definition schema")
}

impl LoadedDefinition {
    /// Structural schema only. Relations and export readiness remain engine rules.
    pub fn values_schema(&self) -> Value {
        // Generate a subschema, not a root schema: recursive Node references
        // must resolve to definitions rather than to the documentary-values root.
        let mut generator = schemars::SchemaGenerator::default();
        let rich = generator.subschema_for::<crate::content::editorial::RichText>();
        let date = generator.subschema_for::<Option<crate::dates::DateValue>>();
        let resource = generator.subschema_for::<Option<crate::resources::ResourceReference>>();
        let mut definitions = generator.take_definitions(false);
        definitions.insert(
            "ResourceReferenceOrNull".into(),
            serde_json::to_value(resource).expect("resource schema"),
        );
        definitions.insert(
            "DateValueOrNull".into(),
            serde_json::to_value(date).expect("date schema"),
        );
        definitions.insert(
            "EditorialValue".into(),
            serde_json::to_value(rich).expect("editorial schema"),
        );

        let mut properties = field_properties(&self.0.fields);
        for collection in &self.0.collections {
            if collection.kind != super::CollectionKind::OrderedRecords {
                let mut native = match collection.kind {
                    super::CollectionKind::BibliographicRecords => serde_json::to_value(
                        schemars::schema_for!(typstographe_bibliography::Record),
                    )
                    .unwrap(),
                    _ => serde_json::to_value(schemars::schema_for!(
                        typstographe_bibliography::ImportSource
                    ))
                    .unwrap(),
                };
                if let Some(Value::Object(defs)) = native.as_object_mut().unwrap().remove("$defs") {
                    definitions.extend(defs);
                }
                native.as_object_mut().unwrap().remove("$schema");
                properties.insert(
                    collection.id.clone(),
                    json!({"type":"array","maxItems":collection.max_items,"items":native}),
                );
                continue;
            }
            let mut props = field_properties(&collection.fields);
            props.insert(
                "id".into(),
                json!({
                    "type":"string", "minLength":1, "x-max-utf8-bytes":128,
                    "pattern":r"^[^\u0000-\u001f\u007f-\u009f]+$"
                }),
            );
            let required: Vec<_> = props.keys().cloned().collect();
            properties.insert(
                collection.id.clone(),
                json!({
                    "type":"array", "maxItems":collection.max_items,
                    "x-identity":"id", "x-unique-by":collection.unique_by,
                    "items":{"type":"object","properties":props,
                             "required":required,"additionalProperties":false}
                }),
            );
        }
        let required: Vec<_> = properties.keys().cloned().collect();
        json!({
            "$schema":"https://json-schema.org/draft/2020-12/schema",
            "title":format!("{} documentary values", self.0.id),
            "description":"Structural constraints only; document relations and export requirements are validated by the engine.",
            "type":"object", "properties":properties, "required":required,
            "additionalProperties":false, "$defs":definitions
        })
    }

    /// Fresh values; export-only requirements deliberately allow an empty draft.
    pub fn defaults(&self) -> Map<String, Value> {
        self.0
            .fields
            .iter()
            .map(|f| (f.id().into(), f.default_value()))
            .chain(self.0.collections.iter().map(|c| (c.id.clone(), json!([]))))
            .collect()
    }

    /// Record fields only: the editing operation supplies the stable identity.
    pub fn record_defaults(&self, collection: &str) -> Option<Map<String, Value>> {
        self.0
            .collections
            .iter()
            .find(|c| c.id == collection)
            .map(|c| {
                if c.kind != super::CollectionKind::OrderedRecords {
                    return Map::new(); // Native records are created through bibliography commands.
                }
                c.fields
                    .iter()
                    .map(|f| (f.id().into(), f.default_value()))
                    .collect()
            })
    }
}

fn field_properties(fields: &[Field]) -> Map<String, Value> {
    fields
        .iter()
        .map(|f| {
            let mut schema = match f {
                Field::Text { multiline, .. } => json!({
                    "type":"string", "x-max-utf8-bytes":16000,
                    "pattern":if *multiline {
                        r"^[^\u0000-\u0009\u000b-\u001f\u007f-\u009f]*$"
                    } else { r"^[^\u0000-\u001f\u007f-\u009f]*$" }
                }),
                Field::ResourceReference { resource_kind, profile_default, .. } => json!({"$ref":"#/$defs/ResourceReferenceOrNull", "x-resource-kind":resource_kind, "x-profile-default":profile_default}),
                Field::Date { .. } => json!({"$ref":"#/$defs/DateValueOrNull"}),
                Field::Boolean { .. } => json!({"type":"boolean"}),
                Field::Choice { choices, .. } => json!({"type":"string","enum":choices}),
                Field::Language { .. } => json!({"type":"string","enum":
                crate::document::DocumentLanguage::ALL.map(|l| l.code())}),
                Field::Tokens { .. } => json!({"type":"array","maxItems":100,
                "items":{"type":"string","x-max-utf8-bytes":1000,
                    "pattern":r"^[^\u0000-\u001f\u007f-\u009f]*$"}}),
                Field::RichText { profile, .. } => {
                    json!({"$ref":"#/$defs/EditorialValue","x-rich-profile":profile})
                }
                Field::RecordReferences {
                    target_collection,
                    on_target_remove,
                    ..
                } => json!({
                    "type":"array","items":{"type":"string"},"uniqueItems":true,
                    "x-target-collection":target_collection,"x-on-target-remove":on_target_remove
                }),
            };
            if let Some(option) = f.presentation() {
                schema = json!({"anyOf":[schema,{"type":"null"}],"x-presentation-option":option});
            }
            schema["default"] = f.default_value();
            schema["x-label-key"] = json!(f.label_key());
            schema["x-required-for-export"] = json!(f.required_for_export());
            (f.id().into(), schema)
        })
        .collect()
}
