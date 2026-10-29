//! First typed commands of the multizone contract. No selection, focus or transport state.
use super::{Node, contract::Error};
use crate::document_types::MetadataField;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Paths use projected child indices in the guarded revision, never Yjs identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TextZone {
    Body {
        path: Vec<usize>,
    },
    Annotation {
        path: Vec<usize>,
    },
    Metadata {
        field: MetadataField,
    },
    Record {
        collection: String,
        id: String,
        field: String,
        #[serde(default)]
        path: Vec<usize>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextTarget {
    pub zone: TextZone,
    pub from: u32,
    pub to: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldTarget {
    Option {
        name: OptionName,
    },
    Metadata {
        field: MetadataField,
    },
    Record {
        collection: String,
        id: String,
        field: String,
    },
}
/// General document options; values remain JSON, checked against their Rust types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OptionName {
    Paper,
    Landscape,
    Language,
    BulletMarker,
    SpellingIgnored,
}
impl OptionName {
    pub fn key(self) -> &'static str {
        match self {
            Self::Paper => "paper",
            Self::Landscape => "landscape",
            Self::Language => "language",
            Self::BulletMarker => "bullet_marker",
            Self::SpellingIgnored => "spelling_ignored",
        }
    }
}
/// Validated immutable image bytes, separate from the shared text/history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImageResource {
    pub path: String,
    /// Base64 on the wire; decoded immutable bytes in the engine.
    #[serde(with = "image_bytes")]
    #[schemars(with = "String")]
    pub data: Vec<u8>,
}
mod image_bytes {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(data))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let data = String::deserialize(deserializer)?;
        if data.len() > crate::assets::MAX_IMAGE_BYTES.div_ceil(3) * 4 {
            return Err(serde::de::Error::custom("image_too_large"));
        }
        STANDARD.decode(data).map_err(serde::de::Error::custom)
    }
}
/// A body tree or one rich record field. No XML identifiers or global positions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TreeRoot {
    Body,
    Record {
        collection: String,
        id: String,
        field: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TreeTarget {
    pub root: TreeRoot,
    #[serde(default)]
    pub path: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Style {
    Bold,
    Italic,
    Link { url: String },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Switch {
    #[default]
    On,
    Off,
}
/// Explicit documentary ranges. Participant typing styles live in content::editing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "id", content = "params", deny_unknown_fields)]
pub enum Command {
    #[serde(rename = "resource.add")]
    AddResource { resource: ImageResource },
    #[serde(rename = "document.set_signature")]
    SetSignature { path: Option<String> },
    #[serde(rename = "document.set_model")]
    SetModel { model_id: String, source: String },
    #[serde(rename = "text.insert")]
    Insert {
        target: TextTarget,
        text: String,
        #[serde(default)]
        marks: Vec<Style>,
    },
    #[serde(rename = "text.delete")]
    Delete { target: TextTarget },
    #[serde(rename = "text.bold")]
    Bold {
        target: TextTarget,
        #[serde(default)]
        value: Switch,
    },
    #[serde(rename = "text.italic")]
    Italic {
        target: TextTarget,
        #[serde(default)]
        value: Switch,
    },
    #[serde(rename = "text.link")]
    Link { target: TextTarget, url: String },
    #[serde(rename = "text.remove_link")]
    RemoveLink { target: TextTarget },
    #[serde(rename = "text.clear")]
    Clear { target: TextTarget },
    #[serde(rename = "spelling.ignore")]
    IgnoreSpelling {
        target: TextTarget,
        language: crate::document::DocumentLanguage,
        word: String,
    },
    #[serde(rename = "block.insert")]
    InsertBlock {
        parent: TreeTarget,
        index: u32,
        node: Node,
    },
    #[serde(rename = "block.replace")]
    ReplaceBlock { target: TreeTarget, node: Node },
    #[serde(rename = "block.splice")]
    SpliceBlocks {
        parent: TreeTarget,
        index: u32,
        delete_count: u32,
        nodes: Vec<Node>,
    },
    #[serde(rename = "inline.replace")]
    ReplaceInline { target: TextTarget, node: Node },
    #[serde(rename = "node.set_attrs")]
    SetNodeAttrs {
        target: TreeTarget,
        attrs: serde_json::Map<String, Value>,
    },
    #[serde(rename = "block.remove")]
    RemoveBlock { target: TreeTarget },
    #[serde(rename = "block.split")]
    SplitBlock { target: TextTarget },
    #[serde(rename = "block.paragraph")]
    Paragraph { target: TreeTarget },
    #[serde(rename = "block.heading_1")]
    Heading1 { target: TreeTarget },
    #[serde(rename = "block.heading_2")]
    Heading2 { target: TreeTarget },
    #[serde(rename = "block.heading_3")]
    Heading3 { target: TreeTarget },
    #[serde(rename = "insert.table")]
    InsertTable {
        parent: TreeTarget,
        index: u32,
        rows: u32,
        columns: u32,
        header: bool,
    },
    #[serde(rename = "table.row.insert")]
    InsertTableRow { target: TreeTarget, index: u32 },
    #[serde(rename = "table.row.remove")]
    RemoveTableRow { target: TreeTarget, index: u32 },
    #[serde(rename = "table.column.insert")]
    InsertTableColumn { target: TreeTarget, index: u32 },
    #[serde(rename = "table.column.remove")]
    RemoveTableColumn { target: TreeTarget, index: u32 },
    #[serde(rename = "table.header")]
    TableHeader { target: TreeTarget, enabled: bool },
    #[serde(rename = "record.add")]
    AddRecord {
        collection: String,
        id: String,
        #[serde(default)]
        values: serde_json::Map<String, Value>,
        #[serde(default)]
        after: Option<String>,
    },
    #[serde(rename = "record.remove")]
    RemoveRecord { collection: String, id: String },
    #[serde(rename = "record.move")]
    MoveRecord {
        collection: String,
        id: String,
        #[serde(default)]
        before: Option<String>,
    },
    #[serde(rename = "metadata.set")]
    SetField { target: FieldTarget, value: Value },
}
impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Self::AddResource { .. } => "resource.add",
            Self::SetSignature { .. } => "document.set_signature",
            Self::SetModel { .. } => "document.set_model",
            Self::Insert { .. } => "text.insert",
            Self::Delete { .. } => "text.delete",
            Self::Bold { .. } => "text.bold",
            Self::Italic { .. } => "text.italic",
            Self::Link { .. } => "text.link",
            Self::RemoveLink { .. } => "text.remove_link",
            Self::Clear { .. } => "text.clear",
            Self::IgnoreSpelling { .. } => "spelling.ignore",
            Self::InsertBlock { .. } => "block.insert",
            Self::ReplaceBlock { .. } => "block.replace",
            Self::SpliceBlocks { .. } => "block.splice",
            Self::ReplaceInline { .. } => "inline.replace",
            Self::SetNodeAttrs { .. } => "node.set_attrs",
            Self::RemoveBlock { .. } => "block.remove",
            Self::SplitBlock { .. } => "block.split",
            Self::Paragraph { .. } => "block.paragraph",
            Self::Heading1 { .. } => "block.heading_1",
            Self::Heading2 { .. } => "block.heading_2",
            Self::Heading3 { .. } => "block.heading_3",
            Self::InsertTable { .. } => "insert.table",
            Self::InsertTableRow { .. } => "table.row.insert",
            Self::RemoveTableRow { .. } => "table.row.remove",
            Self::InsertTableColumn { .. } => "table.column.insert",
            Self::RemoveTableColumn { .. } => "table.column.remove",
            Self::TableHeader { .. } => "table.header",
            Self::AddRecord { .. } => "record.add",
            Self::RemoveRecord { .. } => "record.remove",
            Self::MoveRecord { .. } => "record.move",
            Self::SetField { .. } => "metadata.set",
        }
    }
}
pub const IDS: &[&str] = &[
    "resource.add",
    "document.set_signature",
    "document.set_model",
    "insert.table",
    "table.row.insert",
    "table.row.remove",
    "table.column.insert",
    "table.column.remove",
    "table.header",
    "block.insert",
    "block.replace",
    "block.splice",
    "inline.replace",
    "node.set_attrs",
    "block.remove",
    "block.split",
    "block.paragraph",
    "block.heading_1",
    "block.heading_2",
    "block.heading_3",
    "record.add",
    "record.remove",
    "record.move",
    "text.insert",
    "text.delete",
    "text.bold",
    "text.italic",
    "text.link",
    "text.remove_link",
    "text.clear",
    "spelling.ignore",
    "metadata.set",
];

/// Generated from the same types accepted by execution; no parallel parameter vocabulary.
pub fn schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Command)).expect("command schema")
}
pub fn specifications() -> Value {
    json!({"version": super::contract::VERSION, "ids": IDS, "command_schema": schema(),
        "target_offsets":"utf16", "execution":"guarded_atomic_batch",
        "adapters_migrated":false,
        "scope":"Multizone text, blocks, inline objects, header records, immutable image resources, document options, signatures and Typst models. Inserted marks are explicit. Tauri commands and participant history use the shared owner; standalone adapters are not yet migrated."})
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Availability {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Error>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ZoneInfo {
    pub text: String,
    pub length: u32,
    pub supports_styles: bool,
}

#[cfg(test)]
mod tests {
    #[test]
    fn advertised_identifiers_match_the_actual_deserializable_schema() {
        let schema = super::schema();
        let mut generated: Vec<_> = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|variant| {
                variant["properties"]["id"]["const"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        let mut advertised: Vec<_> = super::IDS.iter().map(|id| (*id).to_owned()).collect();
        generated.sort();
        advertised.sort();
        assert_eq!(generated, advertised);
        assert!(
            schema["$defs"]["TextTarget"]["properties"]["from"]["minimum"]
                .as_u64()
                .is_some()
        );
    }
}
