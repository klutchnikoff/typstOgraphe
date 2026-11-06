use super::{
    Binding, CollectionKind, Definition, Field, FieldControl, LoadedDefinition, RichProfile, Tab,
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnsupportedVersion,
    InvalidIdentifier,
    DuplicateIdentifier,
    InvalidLimit,
    InvalidDefault,
    UnknownReference,
    UnsupportedCapability,
    ReservedIdentifier,
    MissingBinding,
    MissingView,
    UnsupportedPolicy,
    InvalidValue,
    MissingValue,
    DuplicateValue,
    UnresolvedReference,
    InvalidStructure,
    InvalidOrder,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    /// JSON Pointer into the declaration, independent of the authoring syntax.
    pub path: String,
    pub detail: String,
}

pub(super) fn pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}

fn typst_identifier(value: &str) -> bool {
    if !identifier(value) || value.contains('.') {
        return false;
    }
    // Lexically valid identifiers such as `if` or `none` may still be keywords.
    let tree = typst_syntax::parse_code(value);
    let mut children = tree.children();
    children
        .next()
        .is_some_and(|n| n.kind() == typst_syntax::SyntaxKind::Ident)
        && children.next().is_none()
}

#[derive(Default)]
struct Check(Vec<Diagnostic>);
impl Check {
    fn issue(&mut self, code: DiagnosticCode, path: impl Into<String>, detail: impl Into<String>) {
        self.0.push(Diagnostic {
            code,
            path: path.into(),
            detail: detail.into(),
        });
    }
    fn name(&mut self, name: &str, path: impl Into<String>) {
        if !identifier(name) {
            self.issue(DiagnosticCode::InvalidIdentifier, path, name);
        }
    }
    fn unique<'a>(&mut self, names: impl Iterator<Item = (&'a str, String)>) {
        let mut seen = BTreeSet::new();
        for (name, path) in names {
            self.name(name, &path);
            if !seen.insert(name) {
                self.issue(DiagnosticCode::DuplicateIdentifier, path, name);
            }
        }
    }
    fn limit(&mut self, count: usize, max: usize, path: impl Into<String>) {
        if count > max {
            self.issue(
                DiagnosticCode::InvalidLimit,
                path,
                format!("at most {max} items"),
            );
        }
    }
    fn fields(&mut self, fields: &[Field], path: &str, definition: &Definition, record: bool) {
        self.limit(fields.len(), 128, path);
        self.unique(
            fields
                .iter()
                .enumerate()
                .map(|(i, f)| (f.id(), format!("{path}/{i}/id"))),
        );
        for (i, field) in fields.iter().enumerate() {
            let at = format!("{path}/{i}");
            self.name(field.label_key(), format!("{at}/label-key"));
            if let Some(option) = field.presentation() {
                use crate::document_types::model_options::OptionId;
                if record
                    || field.required_for_export()
                    || !matches!(
                        (option, field),
                        (OptionId::Outline, Field::Boolean { .. })
                            | (OptionId::CitationStyle, Field::Choice { .. })
                    )
                {
                    self.issue(
                        DiagnosticCode::UnsupportedPolicy,
                        format!("{at}/presentation"),
                        "unsupported presentation option binding",
                    );
                }
                if fields[..i].iter().any(|f| f.presentation() == Some(option)) {
                    self.issue(
                        DiagnosticCode::DuplicateIdentifier,
                        format!("{at}/presentation"),
                        "duplicate presentation option",
                    );
                }
            }
            if let Some(control) = field.control() {
                let supported = !record
                    && match control {
                        FieldControl::Outline => matches!(field, Field::Boolean { .. }),
                    };
                if !supported {
                    self.issue(
                        DiagnosticCode::UnsupportedPolicy,
                        format!("{at}/control"),
                        "control is incompatible with the field",
                    );
                }
            }
            if record && matches!(field, Field::Date { .. } | Field::ResourceReference { .. }) {
                self.issue(DiagnosticCode::UnsupportedPolicy, &at,
                    "date/resource fields are supported in root forms; their record controls are not registered yet");
            }
            if record && field.id() == "id" {
                self.issue(
                    DiagnosticCode::ReservedIdentifier,
                    format!("{at}/id"),
                    "record identity is implicit",
                );
            }
            if record && !typst_identifier(field.id()) {
                self.issue(
                    DiagnosticCode::InvalidIdentifier,
                    format!("{at}/id"),
                    "record keys must be simple Typst identifiers",
                );
            }
            match field {
                Field::Choice {
                    choices, default, ..
                } => {
                    if choices.is_empty()
                        || choices.len() > 100
                        || choices.iter().any(|v| {
                            v.is_empty() || v.len() > 1000 || v.chars().any(char::is_control)
                        })
                        || choices.iter().collect::<BTreeSet<_>>().len() != choices.len()
                        || if field.presentation().is_some() {
                            default.is_some()
                        } else {
                            default
                                .as_ref()
                                .is_none_or(|value| !choices.contains(value))
                        }
                    {
                        self.issue(
                            DiagnosticCode::InvalidDefault,
                            &at,
                            "choices must be distinct and contain the default",
                        );
                    }
                }
                Field::ResourceReference {
                    default: Some(_), ..
                } => {
                    self.issue(DiagnosticCode::InvalidDefault, format!("{at}/default"),
                        "resource defaults must be null; profile initialization is explicit in the host");
                }
                Field::Date {
                    default: Some(value),
                    ..
                } if !value.valid() => {
                    self.issue(
                        DiagnosticCode::InvalidDefault,
                        format!("{at}/default"),
                        "invalid documentary date",
                    );
                }
                Field::Text {
                    default, multiline, ..
                } => {
                    if default.len() > 16000
                        || default
                            .chars()
                            .any(|c| c.is_control() && !(c == '\n' && *multiline))
                    {
                        self.issue(
                            DiagnosticCode::InvalidDefault,
                            format!("{at}/default"),
                            "text exceeds 16000 bytes or contains unsupported controls",
                        );
                    }
                }
                Field::Tokens { default, .. } => {
                    if default.len() > 100
                        || default
                            .iter()
                            .any(|s| s.len() > 1000 || s.chars().any(char::is_control))
                    {
                        self.issue(
                            DiagnosticCode::InvalidDefault,
                            format!("{at}/default"),
                            "at most 100 tokens of 1000 bytes, without controls",
                        );
                    }
                }
                Field::RichText {
                    profile: RichProfile::InlineText,
                    default,
                    ..
                } if !default.is_inline() => {
                    self.issue(
                        DiagnosticCode::InvalidDefault,
                        format!("{at}/default"),
                        "inline-text cannot contain paragraph or line breaks",
                    );
                }
                Field::RecordReferences {
                    target_collection,
                    unique,
                    default,
                    ..
                } => {
                    if !definition
                        .collections
                        .iter()
                        .any(|c| c.id == *target_collection)
                    {
                        self.issue(
                            DiagnosticCode::UnknownReference,
                            format!("{at}/target-collection"),
                            target_collection,
                        );
                    }
                    if !*unique {
                        self.issue(
                            DiagnosticCode::UnsupportedPolicy,
                            format!("{at}/unique"),
                            "record references must be unique",
                        );
                    }
                    if !default.is_empty() {
                        self.issue(
                            DiagnosticCode::InvalidDefault,
                            format!("{at}/default"),
                            "default references cannot assume document records exist",
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

impl Definition {
    pub fn validate(self) -> Result<LoadedDefinition, Vec<Diagnostic>> {
        super::structures::expand(self)?.validate_expanded()
    }

    fn validate_expanded(self) -> Result<LoadedDefinition, Vec<Diagnostic>> {
        let mut check = Check::default();
        if self.contract_version != 1 {
            check.issue(
                DiagnosticCode::UnsupportedVersion,
                "/contract-version",
                self.contract_version.to_string(),
            );
        }
        check.name(&self.id, "/id");
        check.name(&self.label_key, "/label-key");
        check.name(&self.description_key, "/description-key");
        // A composition target is one identifier, never an expression or import.
        if !typst_identifier(&self.composition.function) {
            check.issue(
                DiagnosticCode::InvalidIdentifier,
                "/composition/function",
                &self.composition.function,
            );
        }
        check.limit(self.collections.len(), 64, "/collections");
        check.unique(
            self.collections
                .iter()
                .enumerate()
                .map(|(i, c)| (c.id.as_str(), format!("/collections/{i}/id")))
                .chain(
                    self.fields
                        .iter()
                        .enumerate()
                        .map(|(i, f)| (f.id(), format!("/fields/{i}/id"))),
                ),
        );
        check.fields(&self.fields, "/fields", &self, false);
        for (i, collection) in self.collections.iter().enumerate() {
            let at = format!("/collections/{i}");
            if collection.max_items == 0
                || collection.max_items
                    > if collection.kind == CollectionKind::OrderedRecords {
                        100
                    } else {
                        10000
                    }
                || (collection.kind == CollectionKind::OrderedRecords
                    && collection.fields.is_empty())
            {
                check.issue(
                    DiagnosticCode::InvalidLimit,
                    &at,
                    "collection size outside its registered bounds or missing record fields",
                );
            }
            check.name(&collection.label_key, format!("{at}/label-key"));
            check.name(&collection.item_label_key, format!("{at}/item-label-key"));
            if collection.kind != CollectionKind::OrderedRecords {
                if !collection.fields.is_empty()
                    || !collection.unique_by.is_empty()
                    || collection.language_key.is_some()
                    || collection.summary_field
                        != if collection.kind == CollectionKind::BibliographicRecords {
                            "key"
                        } else {
                            "name"
                        }
                    || !collection.summary.is_empty()
                {
                    check.issue(
                        DiagnosticCode::UnsupportedPolicy,
                        &at,
                        "bibliographic collections own their registered fields and validation",
                    );
                }
            } else {
                for key in
                    std::iter::once(&collection.summary_field).chain(collection.summary.iter())
                {
                    if !collection.fields.iter().any(|f| f.id() == key) {
                        check.issue(DiagnosticCode::UnknownReference, &at, key);
                    }
                }
            }
            if let Some(key) = &collection.language_key
                && (!collection
                    .fields
                    .iter()
                    .any(|f| f.id() == key && matches!(f, Field::Language { .. }))
                    || collection.unique_by.as_slice() != [key.clone()])
            {
                check.issue(
                    DiagnosticCode::UnknownReference,
                    format!("{at}/language-key"),
                    "primary language requires a unique language field",
                );
            }
            if let Some(key) = &collection.removal_notice_key {
                check.name(key, format!("{at}/removal-notice-key"));
            }
            check.fields(&collection.fields, &format!("{at}/fields"), &self, true);
            check.unique(
                collection
                    .unique_by
                    .iter()
                    .enumerate()
                    .map(|(j, key)| (key.as_str(), format!("{at}/unique-by/{j}"))),
            );
            for (j, key) in collection.unique_by.iter().enumerate() {
                if !collection.fields.iter().any(|f| {
                    f.id() == key
                        && matches!(
                            f,
                            Field::Text { .. }
                                | Field::Language { .. }
                                | Field::Choice { .. }
                                | Field::Boolean { .. }
                        )
                }) {
                    check.issue(
                        DiagnosticCode::UnknownReference,
                        format!("{at}/unique-by/{j}"),
                        "uniqueness requires an existing scalar field",
                    );
                }
            }
        }
        check.limit(self.capabilities.len(), 128, "/capabilities");
        check.unique(
            self.capabilities
                .iter()
                .enumerate()
                .map(|(i, key)| (key.as_str(), format!("/capabilities/{i}"))),
        );
        for (i, capability) in self.capabilities.iter().enumerate() {
            if crate::content::catalogue::find(capability).is_none() {
                check.issue(
                    DiagnosticCode::UnsupportedCapability,
                    format!("/capabilities/{i}"),
                    capability,
                );
            }
        }
        check_bindings(&self, &mut check);
        check_views(&self, &mut check);
        check_registered(&self, &mut check);
        if check.0.is_empty() {
            Ok(LoadedDefinition(self))
        } else {
            Err(check.0)
        }
    }
}

fn check_bindings(definition: &Definition, check: &mut Check) {
    check.limit(definition.bindings.len(), 192, "/bindings");
    let mut sources = BTreeSet::new();
    let mut parameters = BTreeSet::new();
    let mut metadata_keys = BTreeSet::new();
    for (i, binding) in definition.bindings.iter().enumerate() {
        let at = format!("/bindings/{i}");
        let (id, exists) = match binding {
            Binding::Collection { collection, .. } => (
                collection,
                definition.collections.iter().any(|c| c.id == *collection),
            ),
            Binding::Field { field, .. } => {
                (field, definition.fields.iter().any(|f| f.id() == field))
            }
        };
        if !exists {
            check.issue(DiagnosticCode::UnknownReference, &at, id);
        }
        if !sources.insert(id.as_str()) || !parameters.insert(binding.parameter()) {
            check.issue(
                DiagnosticCode::DuplicateIdentifier,
                &at,
                "source and Typst parameter must each be unique",
            );
        }
        if !metadata_keys.insert(binding.metadata_key()) {
            check.issue(
                DiagnosticCode::DuplicateIdentifier,
                &at,
                "metadata storage keys must be unique",
            );
        }
        check.name(binding.metadata_key(), format!("{at}/metadata-key"));
        if !typst_identifier(binding.parameter()) {
            check.issue(
                DiagnosticCode::InvalidIdentifier,
                format!("{at}/typst-parameter"),
                binding.parameter(),
            );
        }
        if binding.parameter() == "options" || binding.metadata_key() == "options" {
            check.issue(
                DiagnosticCode::ReservedIdentifier,
                &at,
                "options belongs to the common document contract",
            );
        }
    }
    for id in definition
        .collections
        .iter()
        .map(|c| c.id.as_str())
        .chain(definition.fields.iter().map(Field::id))
    {
        if !sources.contains(id) {
            check.issue(DiagnosticCode::MissingBinding, "/bindings", id);
        }
    }
}

fn check_views(definition: &Definition, check: &mut Check) {
    check.limit(definition.header.tabs.len(), 64, "/header/tabs");
    check.unique(
        definition
            .header
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id(), format!("/header/tabs/{i}/id"))),
    );
    let mut visible = BTreeSet::new();
    for (i, tab) in definition.header.tabs.iter().enumerate() {
        let at = format!("/header/tabs/{i}");
        check.name(tab.label_key(), format!("{at}/label-key"));
        match tab {
            Tab::RecordCards {
                collection,
                summary_field,
                fields,
                item_label_key,
                ..
            } => {
                let Some(target) = definition.collections.iter().find(|c| c.id == *collection)
                else {
                    check.issue(
                        DiagnosticCode::UnknownReference,
                        format!("{at}/collection"),
                        collection,
                    );
                    continue;
                };
                if target.kind != CollectionKind::OrderedRecords {
                    check.issue(
                        DiagnosticCode::UnsupportedPolicy,
                        &at,
                        "bibliographic records require their registered bibliography view",
                    );
                }
                if let Some(key) = item_label_key {
                    check.name(key, format!("{at}/item-label-key"));
                }
                if !target.fields.iter().any(|f| f.id() == summary_field) {
                    check.issue(
                        DiagnosticCode::UnknownReference,
                        format!("{at}/summary-field"),
                        summary_field,
                    );
                }
                check.unique(
                    fields
                        .iter()
                        .enumerate()
                        .map(|(j, f)| (f.as_str(), format!("{at}/fields/{j}"))),
                );
                if fields.is_empty() {
                    for field in &target.fields {
                        visible.insert((collection.as_str(), field.id()));
                    }
                } else {
                    for (j, field) in fields.iter().enumerate() {
                        if !target.fields.iter().any(|f| f.id() == field) {
                            check.issue(
                                DiagnosticCode::UnknownReference,
                                format!("{at}/fields/{j}"),
                                field,
                            );
                        }
                        visible.insert((collection.as_str(), field.as_str()));
                    }
                }
            }
            Tab::Bibliography {
                collection,
                style_field,
                ..
            } => {
                if !definition
                    .collections
                    .iter()
                    .any(|c| c.id == *collection && c.kind == CollectionKind::BibliographicRecords)
                {
                    check.issue(
                        DiagnosticCode::UnknownReference,
                        format!("{at}/collection"),
                        collection,
                    );
                }
                if !definition
                    .fields
                    .iter()
                    .any(|f| f.id() == style_field && matches!(f, Field::Choice { .. }))
                {
                    check.issue(
                        DiagnosticCode::UnknownReference,
                        format!("{at}/style-field"),
                        style_field,
                    );
                }
                visible.insert(("", style_field.as_str()));
            }
            Tab::Form { sections, .. } => {
                check.limit(sections.len(), 64, format!("{at}/sections"));
                check.unique(
                    sections
                        .iter()
                        .enumerate()
                        .map(|(j, s)| (s.id.as_str(), format!("{at}/sections/{j}/id"))),
                );
                if sections.is_empty() {
                    check.issue(
                        DiagnosticCode::InvalidLimit,
                        &at,
                        "form requires at least one section",
                    );
                }
                for (j, section) in sections.iter().enumerate() {
                    let at = format!("{at}/sections/{j}");
                    check.name(&section.label_key, format!("{at}/label-key"));
                    check.unique(
                        section
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(k, f)| (f.as_str(), format!("{at}/fields/{k}"))),
                    );
                    if section.fields.is_empty() {
                        check.issue(
                            DiagnosticCode::InvalidLimit,
                            &at,
                            "section requires at least one field",
                        );
                    }
                    for (k, id) in section.fields.iter().enumerate() {
                        if !definition.fields.iter().any(|f| f.id() == id) {
                            check.issue(
                                DiagnosticCode::UnknownReference,
                                format!("{at}/fields/{k}"),
                                id,
                            );
                        }
                        visible.insert(("", id.as_str()));
                    }
                }
            }
        }
    }
    for (scope, fields) in std::iter::once(("", &definition.fields)).chain(
        definition
            .collections
            .iter()
            .map(|c| (c.id.as_str(), &c.fields)),
    ) {
        for field in fields {
            if !visible.contains(&(scope, field.id()))
                && field.control() != Some(FieldControl::Outline)
            {
                check.issue(
                    DiagnosticCode::MissingView,
                    "/header/tabs",
                    format!("{scope}/{}", field.id()),
                );
            }
        }
    }
}

fn check_registered(definition: &Definition, check: &mut Check) {
    if let Some(key) = &definition.settings_heading_key {
        check.name(key, "/settings-heading-key");
    }
    for (i, requirement) in definition.export_requirements.iter().enumerate() {
        let super::ExportRequirement::PrimaryLanguage {
            collection,
            language_field,
            value_field,
        } = requirement;
        if !definition.collections.iter().any(|c| {
            c.id == *collection
                && c.language_key.as_deref() == Some(language_field.as_str())
                && c.fields.iter().any(|f| {
                    f.id() == value_field
                        && matches!(f, Field::Text { .. } | Field::RichText { .. })
                })
        }) {
            check.issue(
                DiagnosticCode::UnknownReference,
                format!("/export-requirements/{i}"),
                "primary-language requires a localized text field",
            );
        }
    }
    let outlines = definition
        .fields
        .iter()
        .filter(|f| f.control() == Some(FieldControl::Outline))
        .count();
    let bibliographies = definition
        .header
        .tabs
        .iter()
        .filter(|t| matches!(t, Tab::Bibliography { .. }))
        .count();
    check.limit(outlines, 1, "/fields");
    check.limit(bibliographies, 1, "/header/tabs");
    for collection in definition
        .collections
        .iter()
        .filter(|c| c.kind == CollectionKind::BibliographicRecords)
    {
        if !definition
            .header
            .tabs
            .iter()
            .any(|t| matches!(t, Tab::Bibliography {collection:id,..} if id == &collection.id))
        {
            check.issue(DiagnosticCode::MissingView, "/header/tabs", &collection.id);
        }
    }
}
