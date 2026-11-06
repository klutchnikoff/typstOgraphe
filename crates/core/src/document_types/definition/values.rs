//! Document-value rules shared by every adapter, distinct from manifest validation.
use super::{
    CollectionKind, Diagnostic, DiagnosticCode, ExportRequirement, Field, LoadedDefinition,
    RichProfile,
};
use crate::{content::editorial::RichText, document::DocumentLanguage};
use serde_json::Value;
use std::collections::BTreeSet;

fn issue(code: DiagnosticCode, path: impl Into<String>, detail: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code,
        path: path.into(),
        detail: detail.into(),
    }
}
fn valid_text(text: &str, max: usize, multiline: bool) -> bool {
    text.len() <= max
        && !text
            .chars()
            .any(|c| c.is_control() && !(c == '\n' && multiline))
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && valid_text(id, 128, false)
}

impl LoadedDefinition {
    /// `values` uses logical field/collection IDs, not storage keys or Typst parameters.
    pub fn validate_values(&self, values: &Value) -> Result<(), Vec<Diagnostic>> {
        let mut issues = Vec::new();
        let Some(object) = values.as_object() else {
            return Err(vec![issue(
                DiagnosticCode::InvalidValue,
                "",
                "expected documentary values object",
            )]);
        };
        for key in object.keys() {
            if !self.0.fields.iter().any(|f| f.id() == key)
                && !self.0.collections.iter().any(|c| c.id == *key)
            {
                issues.push(issue(
                    DiagnosticCode::InvalidValue,
                    format!("/{}", super::validation::pointer(key)),
                    "unknown documentary field",
                ));
            }
        }
        validate_fields(&self.0.fields, values, "", values, &mut issues);
        for collection in &self.0.collections {
            let path = format!("/{}", super::validation::pointer(&collection.id));
            let Some(records) = values[&collection.id].as_array() else {
                issues.push(issue(
                    DiagnosticCode::InvalidValue,
                    &path,
                    "expected ordered records",
                ));
                continue;
            };
            if records.len() > collection.max_items as usize {
                issues.push(issue(
                    DiagnosticCode::InvalidValue,
                    &path,
                    "collection size exceeds its contract",
                ));
            }
            if collection.kind != CollectionKind::OrderedRecords {
                let result = match collection.kind {
                    CollectionKind::BibliographicRecords => {
                        serde_json::from_value::<Vec<typstographe_bibliography::Record>>(
                            Value::Array(records.clone()),
                        )
                        .map_err(|e| e.to_string())
                        .and_then(|records| {
                            typstographe_bibliography::Library::new(records)
                                .map(|_| ())
                                .map_err(|e| e.to_string())
                        })
                    }
                    _ => serde_json::from_value::<Vec<typstographe_bibliography::ImportSource>>(
                        Value::Array(records.clone()),
                    )
                    .map_err(|e| e.to_string())
                    .and_then(|sources| {
                        let ids: BTreeSet<_> = sources.iter().map(|s| s.id()).collect();
                        if ids.len() == sources.len() {
                            Ok(())
                        } else {
                            Err("Duplicate bibliography source identity".into())
                        }
                    }),
                };
                if let Err(detail) = result {
                    issues.push(issue(DiagnosticCode::InvalidValue, &path, detail));
                }
                continue;
            }
            let mut ids = BTreeSet::new();
            let mut unique = BTreeSet::new();
            for (i, record) in records.iter().enumerate() {
                let at = format!("{path}/{i}");
                let Some(object) = record.as_object() else {
                    issues.push(issue(
                        DiagnosticCode::InvalidValue,
                        &at,
                        "expected record object",
                    ));
                    continue;
                };
                if !record["id"].as_str().is_some_and(valid_id)
                    || !ids.insert(record["id"].clone().to_string())
                {
                    issues.push(issue(
                        DiagnosticCode::DuplicateValue,
                        format!("{at}/id"),
                        "record IDs must be valid and distinct",
                    ));
                }
                for key in object.keys().filter(|k| k.as_str() != "id") {
                    if !collection.fields.iter().any(|f| f.id() == key) {
                        issues.push(issue(
                            DiagnosticCode::InvalidValue,
                            format!("{at}/{}", super::validation::pointer(key)),
                            "unknown record field",
                        ));
                    }
                }
                validate_fields(&collection.fields, record, &at, values, &mut issues);
                if !collection.unique_by.is_empty() {
                    let tuple: Vec<_> = collection
                        .unique_by
                        .iter()
                        .map(|k| record[k].clone())
                        .collect();
                    if !unique.insert(serde_json::to_string(&tuple).expect("scalar uniqueness key"))
                    {
                        issues.push(issue(
                            DiagnosticCode::DuplicateValue,
                            &at,
                            "duplicate collection uniqueness key",
                        ));
                    }
                }
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    /// Incomplete drafts remain valid: export requirements are evaluated separately.
    pub fn export_diagnostics(
        &self,
        values: &Value,
        language: DocumentLanguage,
    ) -> Vec<Diagnostic> {
        if let Err(issues) = self.validate_values(values) {
            return issues;
        }
        let mut issues = Vec::new();
        required(&self.0.fields, values, "", &mut issues);
        for collection in &self.0.collections {
            for (i, record) in values[&collection.id]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                required(
                    &collection.fields,
                    record,
                    &format!("/{}/{i}", super::validation::pointer(&collection.id)),
                    &mut issues,
                );
            }
        }
        for requirement in &self.0.export_requirements {
            let ExportRequirement::PrimaryLanguage {
                collection,
                language_field,
                value_field,
            } = requirement;
            let field = self
                .0
                .collections
                .iter()
                .find(|c| c.id == *collection)
                .unwrap()
                .fields
                .iter()
                .find(|f| f.id() == value_field)
                .unwrap();
            let complete = values[collection]
                .as_array()
                .into_iter()
                .flatten()
                .any(|r| r[language_field] == language.code() && !empty(field, &r[value_field]));
            if !complete {
                issues.push(issue(
                    DiagnosticCode::MissingValue,
                    format!("/{}", super::validation::pointer(collection)),
                    format!("{value_field} is required in {}", language.code()),
                ));
            }
        }
        issues
    }
}

fn validate_fields(
    fields: &[Field],
    value: &Value,
    path: &str,
    all: &Value,
    issues: &mut Vec<Diagnostic>,
) {
    for field in fields {
        let at = format!("{path}/{}", super::validation::pointer(field.id()));
        let Some(v) = value.get(field.id()) else {
            issues.push(issue(
                DiagnosticCode::MissingValue,
                at,
                "missing documentary field",
            ));
            continue;
        };
        if v.is_null() && field.presentation().is_some() {
            continue;
        }
        let valid = match field {
            Field::Text { multiline, .. } => {
                v.as_str().is_some_and(|s| valid_text(s, 16000, *multiline))
            }
            Field::ResourceReference { .. } => {
                serde_json::from_value::<Option<crate::resources::ResourceReference>>(v.clone())
                    .is_ok()
            }
            Field::Date { .. } => {
                serde_json::from_value::<Option<crate::dates::DateValue>>(v.clone()).is_ok()
            }
            Field::Boolean { .. } => v.is_boolean(),
            Field::Choice { choices, .. } => {
                v.as_str().is_some_and(|s| choices.iter().any(|c| c == s))
            }
            Field::Language { .. } => serde_json::from_value::<DocumentLanguage>(v.clone()).is_ok(),
            Field::Tokens { .. } => v.as_array().is_some_and(|a| {
                a.len() <= 100
                    && a.iter()
                        .all(|v| v.as_str().is_some_and(|s| valid_text(s, 1000, false)))
            }),
            Field::RichText { profile, .. } => serde_json::from_value::<RichText>(v.clone())
                .is_ok_and(|t| *profile != RichProfile::InlineText || t.is_inline()),
            Field::RecordReferences {
                target_collection, ..
            } => {
                let mut ids = BTreeSet::new();
                let valid = v.as_array().is_some_and(|a| {
                    a.iter()
                        .all(|v| v.as_str().is_some_and(|s| valid_id(s) && ids.insert(s)))
                });
                if valid {
                    for id in v.as_array().unwrap() {
                        if !all[target_collection]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|r| r["id"] == *id))
                        {
                            issues.push(issue(
                                DiagnosticCode::UnresolvedReference,
                                &at,
                                format!("{id} is absent from {target_collection}"),
                            ));
                        }
                    }
                }
                valid
            }
        };
        if !valid {
            issues.push(issue(
                DiagnosticCode::InvalidValue,
                at,
                "value does not satisfy the declared field contract",
            ));
        }
    }
}
fn empty(field: &Field, value: &Value) -> bool {
    match field {
        Field::RichText { .. } => {
            serde_json::from_value::<RichText>(value.clone()).map_or(true, |t| t.is_empty())
        }
        _ => match value {
            Value::String(s) => s.trim().is_empty(),
            Value::Array(a) => a.is_empty(),
            Value::Null => true,
            _ => false,
        },
    }
}
fn required(fields: &[Field], value: &Value, path: &str, issues: &mut Vec<Diagnostic>) {
    for f in fields.iter().filter(|f| f.required_for_export()) {
        if empty(f, &value[f.id()]) {
            issues.push(issue(
                DiagnosticCode::MissingValue,
                format!("{path}/{}", super::validation::pointer(f.id())),
                "field is required for export",
            ));
        }
    }
}
