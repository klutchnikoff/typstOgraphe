use crate::{Code, Error, MAX_RECORDS, Profile, Result};
use hayagriva::Entry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RecordId(String);

impl schemars::JsonSchema for RecordId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "RecordId".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","pattern":"^[A-Za-z0-9_-]{1,128}$"})
    }
}

impl RecordId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(Error::new(
                Code::InvalidIdentity,
                "Identity must be 1–128 ASCII letters, digits, hyphens or underscores",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for RecordId {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}
impl From<RecordId> for String {
    fn from(value: RecordId) -> Self {
        value.0
    }
}

/// Content guard, not an identity. No clock or process-local hash is involved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Fingerprint(String);

impl TryFrom<String> for Fingerprint {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::new(
                Code::InvalidField,
                "Fingerprint must be 64 lowercase hexadecimal digits",
            ));
        }
        Ok(Self(value))
    }
}
impl From<Fingerprint> for String {
    fn from(value: Fingerprint) -> Self {
        value.0
    }
}
impl schemars::JsonSchema for Fingerprint {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Fingerprint".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","pattern":"^[0-9a-f]{64}$"})
    }
}

pub(crate) fn digest(bytes: &[u8]) -> Fingerprint {
    Fingerprint(
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

pub(crate) fn fingerprint(value: &impl Serialize) -> Fingerprint {
    fn sort(value: &mut Value) {
        match value {
            Value::Object(map) => {
                map.sort_keys();
                for child in map.values_mut() {
                    sort(child);
                }
            }
            Value::Array(values) => values.iter_mut().for_each(sort),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).expect("bibliographic values serialize");
    sort(&mut value);
    digest(&serde_json::to_vec(&value).unwrap())
}

/// Opaque origin link. Source bytes live in the import artifact, not in every record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub source_id: RecordId,
    pub source_digest: Fingerprint,
    pub original_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RecordData", into = "RecordData")]
pub struct Record {
    id: RecordId,
    entry: Entry,
    pub(crate) retained_parents: Vec<Entry>,
    provenance: Vec<Provenance>,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecordData {
    id: RecordId,
    key: String,
    #[schemars(with = "serde_json::Map<String, Value>")]
    entry: Value,
    /// Inactive native publication containers. Never a second active bibliography.
    #[serde(default)]
    #[schemars(with = "Vec<serde_json::Map<String, Value>>")]
    retained_parents: Vec<Value>,
    #[serde(default)]
    provenance: Vec<Provenance>,
}

impl TryFrom<RecordData> for Record {
    type Error = Error;
    fn try_from(data: RecordData) -> Result<Self> {
        let mut record = Self::new(data.id, entry_from_value(&data.key, data.entry)?)?;
        record.retained_parents = data
            .retained_parents
            .into_iter()
            .map(|value| entry_from_value(record.key(), value))
            .collect::<Result<_>>()?;
        record.provenance = data.provenance;
        Ok(record)
    }
}
impl From<Record> for RecordData {
    fn from(record: Record) -> Self {
        Self {
            id: record.id,
            key: record.entry.key().into(),
            entry: crate::native::canonical_entry(&record.entry),
            retained_parents: record
                .retained_parents
                .iter()
                .map(canonical_entry)
                .collect(),
            provenance: record.provenance,
        }
    }
}

impl schemars::JsonSchema for Record {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "BibliographicRecord".into()
    }
    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        RecordData::json_schema(generator)
    }
}

pub(crate) fn valid_key(key: &str) -> Result<()> {
    if key.trim().is_empty() || key.len() > 1024 || key.chars().any(char::is_control) {
        return Err(Error::new(
            Code::InvalidKey,
            "A citation key must be nonempty and contain no control characters",
        )
        .at(key, Some("key")));
    }
    Ok(())
}

pub(crate) fn entry_from_value(key: &str, mut value: Value) -> Result<Entry> {
    valid_key(key)?;
    if let Some(path) = crate::native::unsupported_fields(&value).first() {
        return Err(Error::new(
            Code::UnsupportedField,
            "Unknown native field would be discarded",
        )
        .at(key, Some(path)));
    }
    crate::native::normalize_entry(&mut value);
    let library: hayagriva::Library = serde_json::from_value(Value::Object(
        [(key.to_owned(), value)].into_iter().collect(),
    ))
    .map_err(|e| Error::new(Code::InvalidField, e.to_string()).at(key, None))?;
    library
        .into_iter()
        .next()
        .ok_or_else(|| Error::new(Code::InvalidField, "Missing entry"))
}

/// Canonical one-or-many representation, used for pointers without altering native values.
pub(crate) fn canonical_entry(entry: &Entry) -> Value {
    crate::native::canonical_entry(entry)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldPatch {
    pub path: String,
    /// None removes the field. Present values are validated by Hayagriva.
    pub value: Option<Value>,
}

impl Record {
    /// Validate native fields while retaining typed field diagnostics for callers.
    pub fn from_native(id: RecordId, key: &str, value: Value) -> Result<Self> {
        Self::new(id, entry_from_value(key, value)?)
    }
    pub fn new(id: RecordId, entry: Entry) -> Result<Self> {
        valid_key(entry.key())?;
        Ok(Self {
            id,
            entry,
            retained_parents: vec![],
            provenance: vec![],
        })
    }
    pub fn id(&self) -> &RecordId {
        &self.id
    }
    pub fn key(&self) -> &str {
        self.entry.key()
    }
    pub fn profile(&self) -> Profile {
        Profile::infer(&self.entry)
    }
    pub fn retained_parents(&self) -> &[Entry] {
        &self.retained_parents
    }
    pub(crate) fn with_entry(&self, value: Value) -> Result<Self> {
        let mut result = self.clone();
        result.entry = entry_from_value(self.key(), value)?;
        Ok(result)
    }
    pub fn entry(&self) -> &Entry {
        &self.entry
    }
    pub fn provenance(&self) -> &[Provenance] {
        &self.provenance
    }
    pub fn fingerprint(&self) -> Fingerprint {
        fingerprint(self)
    }
    pub fn native_value(&self) -> Value {
        canonical_entry(&self.entry)
    }
    pub(crate) fn add_provenance(&mut self, source: Provenance) {
        if !self.provenance.iter().any(|old| {
            old.source_digest == source.source_digest && old.original_key == source.original_key
        }) {
            self.provenance.push(source);
        }
    }
    pub fn renamed(&self, expected: &Fingerprint, key: &str) -> Result<Self> {
        self.guard(expected)?;
        let mut result = self.clone();
        result.entry = entry_from_value(key, self.native_value())?;
        result.retained_parents = self
            .retained_parents
            .iter()
            .map(|entry| entry_from_value(key, canonical_entry(entry)))
            .collect::<Result<_>>()?;
        Ok(result)
    }
    pub fn edited(&self, expected: &Fingerprint, patches: &[FieldPatch]) -> Result<Self> {
        self.guard(expected)?;
        let mut value = self.native_value();
        for patch in patches {
            let (container, field) = patch.path.rsplit_once('/').ok_or_else(|| {
                Error::new(Code::InvalidField, "A field requires a native JSON pointer")
            })?;
            let ancestors: Vec<_> = container
                .strip_prefix('/')
                .unwrap_or(container)
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();
            let valid_container = container.is_empty()
                || (!ancestors.is_empty()
                    && ancestors.len().is_multiple_of(2)
                    && ancestors
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .all(|pair| pair[0] == "parent" && pair[1].parse::<usize>().is_ok()));
            if !valid_container
                || !crate::import::native_field(field)
                || matches!(field, "type" | "parent" | "serial")
            {
                return Err(Error::new(Code::InvalidField, "Unsupported field patch")
                    .at(self.key(), Some(&patch.path)));
            }
            // Fill only the registered first publication container, never replace an existing tree.
            if container == "/parent/0"
                && patch.value.is_some()
                && value.pointer(container).is_none()
                && value
                    .get("parent")
                    .is_none_or(|v| v.as_array().is_some_and(Vec::is_empty))
            {
                let template = self.profile().empty_entry(self.key());
                let template = canonical_entry(&template);
                value["parent"] = if template["type"] == value["type"] {
                    template
                        .get("parent")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([{}]))
                } else {
                    serde_json::json!([{}])
                };
            }
            let target = value
                .pointer_mut(container)
                .and_then(Value::as_object_mut)
                .ok_or_else(|| {
                    Error::new(Code::InvalidField, "Field container does not exist")
                        .at(self.key(), Some(&patch.path))
                })?;
            match &patch.value {
                Some(v) => {
                    target.insert(field.into(), v.clone());
                }
                None => {
                    target.remove(field);
                }
            }
        }
        let mut result = self.clone();
        result.entry = entry_from_value(self.key(), value)?;
        Ok(result)
    }
    pub(crate) fn guard(&self, expected: &Fingerprint) -> Result<()> {
        if &self.fingerprint() == expected {
            Ok(())
        } else {
            Err(
                Error::new(Code::StaleBase, "Record changed since editing began")
                    .at(self.key(), None),
            )
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Record>", into = "Vec<Record>")]
pub struct Library {
    records: Vec<Record>,
}

impl Library {
    pub fn new(records: Vec<Record>) -> Result<Self> {
        if records.len() > MAX_RECORDS {
            return Err(Error::new(Code::LimitExceeded, "Too many records"));
        }
        let mut ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for record in &records {
            if !ids.insert(record.id()) {
                return Err(
                    Error::new(Code::DuplicateIdentity, "Duplicate record identity")
                        .at(record.key(), None),
                );
            }
            if !keys.insert(record.key()) {
                return Err(
                    Error::new(Code::DuplicateKey, "Duplicate citation key").at(record.key(), None)
                );
            }
        }
        Ok(Self { records })
    }
    pub fn records(&self) -> &[Record] {
        &self.records
    }
    pub fn get(&self, id: &RecordId) -> Option<&Record> {
        self.records.iter().find(|r| r.id() == id)
    }
    pub fn fingerprint(&self) -> Fingerprint {
        fingerprint(self)
    }
    pub fn to_hayagriva(&self) -> Result<String> {
        let native = self
            .records
            .iter()
            .map(|r| r.entry.clone())
            .collect::<hayagriva::Library>();
        hayagriva::io::to_yaml_str(&native)
            .map_err(|e| Error::new(Code::InvalidField, e.to_string()))
    }
}
impl TryFrom<Vec<Record>> for Library {
    type Error = Error;
    fn try_from(records: Vec<Record>) -> Result<Self> {
        Self::new(records)
    }
}
impl From<Library> for Vec<Record> {
    fn from(library: Library) -> Self {
        library.records
    }
}
