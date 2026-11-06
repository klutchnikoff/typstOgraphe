use crate::{
    Code, Diagnostic, Error, Fingerprint, Library, MAX_RECORDS, MAX_SOURCE_BYTES, Profile,
    Provenance, Record, RecordId, Result, Severity,
};
use biblatex::ChunksExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    Hayagriva,
    Biblatex,
}

/// An immutable original, separate from the active native records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(try_from = "SourceData")]
pub struct ImportSource {
    id: RecordId,
    name: String,
    format: ImportFormat,
    contents: String,
    digest: Fingerprint,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SourceData {
    id: RecordId,
    name: String,
    format: ImportFormat,
    contents: String,
    digest: Fingerprint,
}
impl TryFrom<SourceData> for ImportSource {
    type Error = Error;
    fn try_from(data: SourceData) -> Result<Self> {
        let source = Self::new(data.id, &data.name, data.format, data.contents)?;
        if source.digest != data.digest {
            return Err(Error::new(
                Code::InvalidField,
                "Import source digest mismatch",
            ));
        }
        Ok(source)
    }
}

impl ImportSource {
    pub fn new(id: RecordId, name: &str, format: ImportFormat, contents: String) -> Result<Self> {
        if name.is_empty()
            || name.len() > 256
            || name.contains(['/', '\\'])
            || name.chars().any(char::is_control)
        {
            return Err(Error::new(
                Code::InvalidField,
                "An import requires a display name without path separators",
            ));
        }
        size(&contents)?;
        // Hash original UTF-8 bytes, not a normalized or reserialized source.
        let digest = crate::model::digest(contents.as_bytes());
        Ok(Self {
            id,
            name: name.into(),
            format,
            contents,
            digest,
        })
    }
    pub fn id(&self) -> &RecordId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn format(&self) -> ImportFormat {
        self.format
    }
    pub fn contents(&self) -> &str {
        &self.contents
    }
    pub fn digest(&self) -> &Fingerprint {
        &self.digest
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PreparedImport {
    pub source: ImportSource,
    pub records: Vec<Record>,
    pub diagnostics: Vec<Diagnostic>,
}

impl PreparedImport {
    /// Guard the complete preparation, including conversion diagnostics and source bytes.
    pub fn fingerprint(&self) -> Fingerprint {
        crate::model::fingerprint(self)
    }
}

fn size(text: &str) -> Result<()> {
    if text.len() > MAX_SOURCE_BYTES {
        Err(Error::new(
            Code::LimitExceeded,
            "Bibliographic source exceeds 2 MiB",
        ))
    } else {
        Ok(())
    }
}

/// Parse only, for existing source-based callers. Prepared imports add provenance and diagnostics.
pub fn parse_native(text: &str, format: ImportFormat) -> Result<hayagriva::Library> {
    size(text)?;
    let library: hayagriva::Library = match format {
        ImportFormat::Hayagriva => {
            // Validate raw YAML first, including duplicate keys, before any map conversion.
            hayagriva::io::from_yaml_str(text)
                .map_err(|e| Error::new(Code::InvalidSyntax, e.to_string()))?;
            let mut value: Value = yaml_serde::from_str(text)
                .map_err(|e| Error::new(Code::InvalidSyntax, e.to_string()))?;
            if let Some(entries) = value.as_object_mut() {
                entries
                    .values_mut()
                    .for_each(crate::native::normalize_entry);
            }
            serde_json::from_value(value)
                .map_err(|e| Error::new(Code::InvalidSyntax, e.to_string()))?
        }
        ImportFormat::Biblatex => {
            let bibliography = biblatex::Bibliography::parse(text)
                .map_err(|e| Error::new(Code::InvalidSyntax, e.to_string()))?;
            let mut entries = vec![];
            for raw in bibliography.iter() {
                entries.push(hayagriva::Entry::try_from(raw).map_err(|e| {
                    Error::new(Code::InvalidField, e.to_string()).at(&raw.key, None)
                })?);
            }
            entries.into_iter().collect()
        }
    };
    if library.len() > MAX_RECORDS {
        return Err(Error::new(
            Code::LimitExceeded,
            "Too many bibliographic entries",
        ));
    }
    for entry in &library {
        crate::model::valid_key(entry.key())?;
    }
    Ok(library)
}

pub(crate) fn native_field(field: &str) -> bool {
    matches!(
        field,
        "type"
            | "parent"
            | "title"
            | "author"
            | "date"
            | "editor"
            | "affiliated"
            | "publisher"
            | "location"
            | "organization"
            | "issue"
            | "chapter"
            | "volume"
            | "volume-total"
            | "edition"
            | "page-range"
            | "page-total"
            | "time-range"
            | "runtime"
            | "url"
            | "serial-number"
            | "serial"
            | "language"
            | "archive"
            | "archive-location"
            | "call-number"
            | "note"
            | "abstract"
            | "genre"
    )
}

fn diagnostic(code: Code, key: &str, field: Option<&str>, detail: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code,
        key: key.into(),
        field: field.map(Into::into),
        detail: detail.into(),
        severity: if code == Code::ConversionReview {
            Severity::Information
        } else {
            Severity::Warning
        },
    }
}

/// No mutation: returns native values, immutable source and reviewable diagnostics.
/// Identity allocation belongs to the caller; duplicate allocated identities are refused.
pub fn prepare_import(
    source: ImportSource,
    mut identify: impl FnMut(&str) -> RecordId,
) -> Result<PreparedImport> {
    let native = parse_native(source.contents(), source.format())?;
    let mut diagnostics = vec![];
    let mut mappings = std::collections::BTreeMap::new();
    if source.format() == ImportFormat::Hayagriva {
        let values: Value = yaml_serde::from_str(source.contents())
            .map_err(|e| Error::new(Code::InvalidSyntax, e.to_string()))?;
        if let Some(map) = values.as_object() {
            for (key, value) in map {
                for path in crate::native::unsupported_fields(value) {
                    diagnostics.push(diagnostic(
                        Code::UnsupportedField,
                        key,
                        Some(&path),
                        "Field is retained only in the original source",
                    ));
                }
            }
        }
    } else {
        let bibliography = biblatex::Bibliography::parse(source.contents())
            .map_err(|e| Error::new(Code::InvalidSyntax, e.to_string()))?;
        for raw in bibliography.iter() {
            let kind = match raw.entry_type {
                biblatex::EntryType::Software => Some(Profile::Software),
                biblatex::EntryType::Unpublished => Some(Profile::Preprint),
                _ => None,
            };
            if let Some(kind) = kind {
                mappings.insert(raw.key.clone(), kind);
            }
            diagnostics.push(diagnostic(
                Code::ConversionReview,
                &raw.key,
                None,
                format!(
                    "Review Hayagriva conversion of fields: {}",
                    raw.fields.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
            for (field, chunks) in &raw.fields {
                // Conservative: unqualified fields must be reviewed, never reported lossless.
                let reviewed = matches!(
                    field.as_str(),
                    "title"
                        | "author"
                        | "editor"
                        | "date"
                        | "year"
                        | "month"
                        | "journaltitle"
                        | "journal"
                        | "booktitle"
                        | "publisher"
                        | "location"
                        | "address"
                        | "volume"
                        | "number"
                        | "pages"
                        | "doi"
                        | "isbn"
                        | "issn"
                        | "url"
                        | "urldate"
                        | "edition"
                        | "institution"
                        | "organization"
                        | "note"
                        | "language"
                        | "crossref"
                        | "xdata"
                );
                if !reviewed || (field == "date" && chunks.format_verbatim().contains('/')) {
                    diagnostics.push(diagnostic(Code::ConversionLoss, &raw.key, Some(field),
                        "Representation may be reduced or absent in native Hayagriva; original retained"));
                }
            }
            if matches!(
                raw.entry_type,
                biblatex::EntryType::Software | biblatex::EntryType::Dataset
            ) {
                diagnostics.push(diagnostic(Code::TypeMapping, &raw.key, Some("type"),
                    "Native conversion does not uniquely preserve the source genre; explicit mapping required"));
            }
        }
    }
    let mut records = vec![];
    for entry in native {
        let mapping = mappings.get(entry.key()).copied();
        let mut record = Record::new(identify(entry.key()), entry)?;
        if let Some(profile) = mapping {
            record = record.reclassified(&record.fingerprint(), profile)?;
        }
        record.add_provenance(Provenance {
            source_id: source.id.clone(),
            source_digest: source.digest.clone(),
            original_key: record.key().into(),
        });
        diagnostics.extend(completeness(&record));
        records.push(record);
    }
    // Validate all allocated identities/keys without silently dropping an entry.
    Library::new(records.clone())?;
    Ok(PreparedImport {
        source,
        records,
        diagnostics,
    })
}

pub fn completeness(record: &Record) -> Vec<Diagnostic> {
    crate::form::form_fields(record)
        .into_iter()
        .filter_map(|field| {
            let value = &field.value;
            let missing = value.is_null()
                || value.as_str().is_some_and(|s| s.trim().is_empty())
                || value.as_array().is_some_and(Vec::is_empty);
            (field.recommended && missing).then(|| {
                diagnostic(
                    Code::Incomplete,
                    record.key(),
                    Some(&field.path),
                    "Recommended bibliographic field is missing",
                )
            })
        })
        .collect()
}
