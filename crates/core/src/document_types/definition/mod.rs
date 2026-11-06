//! Typed declarations for supplied document types. Loading never registers a type.
mod fields;
mod loader;
mod schema;
mod structures;
mod validation;
mod values;

pub use fields::{Field, FieldControl, ReferenceRemoval, RichProfile};
pub(crate) use loader::decode_yaml;
pub use loader::{LoadError, MAX_SOURCE_BYTES, load_yaml};
pub use schema::manifest_schema;
pub use structures::{AssemblyOrder, HeaderStructure};
pub use validation::{Diagnostic, DiagnosticCode};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Declarative data, before semantic validation. Use `load_yaml` or `validate`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Definition {
    #[schemars(range(min = 1, max = 1))]
    pub contract_version: u32,
    pub id: String,
    pub label_key: String,
    pub description_key: String,
    pub composition: Composition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub structures: Vec<HeaderStructure>,
    #[serde(default, skip_serializing_if = "AssemblyOrder::is_empty")]
    pub order: AssemblyOrder,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_heading_key: Option<String>,
    #[serde(default)]
    pub export_requirements: Vec<ExportRequirement>,
    #[serde(default)]
    pub fields: Vec<Field>,
    #[serde(default)]
    pub collections: Vec<Collection>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub bindings: Vec<Binding>,
    #[serde(default)]
    pub header: Header,
}

/// Immutable validated declaration; it does not replace the current type registry.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedDefinition(Definition);
impl LoadedDefinition {
    pub fn definition(&self) -> &Definition {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Composition {
    pub function: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Collection {
    pub id: String,
    pub kind: CollectionKind,
    pub identity: Identity,
    #[schemars(range(min = 1, max = 100))]
    pub max_items: u32,
    pub label_key: String,
    pub item_label_key: String,
    pub summary_field: String,
    #[serde(default)]
    pub summary: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removal_notice_key: Option<String>,
    #[serde(default)]
    pub unique_by: Vec<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CollectionKind {
    OrderedRecords,
    BibliographicRecords,
    BibliographicSources,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Identity {
    StableId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields, rename_all_fields = "kebab-case")]
pub enum Binding {
    Field {
        field: String,
        typst_parameter: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata_key: Option<String>,
    },
    Collection {
        collection: String,
        typst_parameter: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata_key: Option<String>,
    },
}
impl Binding {
    pub fn source(&self) -> &str {
        match self {
            Self::Field { field, .. } => field,
            Self::Collection { collection, .. } => collection,
        }
    }
    pub fn metadata_key(&self) -> &str {
        match self {
            Self::Field { metadata_key, .. } | Self::Collection { metadata_key, .. } => {
                metadata_key.as_deref().unwrap_or_else(|| self.parameter())
            }
        }
    }
    pub fn parameter(&self) -> &str {
        match self {
            Self::Field {
                typst_parameter, ..
            }
            | Self::Collection {
                typst_parameter, ..
            } => typst_parameter,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Header {
    #[serde(default)]
    pub tabs: Vec<Tab>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "view",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    deny_unknown_fields
)]
pub enum Tab {
    Form {
        id: String,
        label_key: String,
        sections: Vec<Section>,
    },
    RecordCards {
        id: String,
        label_key: String,
        collection: String,
        summary_field: String,
        #[serde(default)]
        fields: Vec<String>,
        editing: Editing,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item_label_key: Option<String>,
    },
    Bibliography {
        id: String,
        label_key: String,
        collection: String,
        style_field: String,
    },
}
impl Tab {
    pub fn id(&self) -> &str {
        match self {
            Self::Form { id, .. }
            | Self::RecordCards { id, .. }
            | Self::Bibliography { id, .. } => id,
        }
    }
    pub fn label_key(&self) -> &str {
        match self {
            Self::Form { label_key, .. }
            | Self::RecordCards { label_key, .. }
            | Self::Bibliography { label_key, .. } => label_key,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Editing {
    Dialog,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Section {
    pub id: String,
    pub label_key: String,
    pub fields: Vec<String>,
    #[serde(default)]
    pub layout: SectionLayout,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SectionLayout {
    #[default]
    Compact,
    Inline,
    Editorial,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    deny_unknown_fields
)]
pub enum ExportRequirement {
    PrimaryLanguage {
        collection: String,
        language_field: String,
        value_field: String,
    },
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod value_tests;
