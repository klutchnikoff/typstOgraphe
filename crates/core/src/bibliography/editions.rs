//! Reconstruct trusted plans from explicit inputs, then publish one guarded document edit.
use crate::content::contract::{Error, State, UnavailableReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use typstographe_bibliography::{
    self as bib, Decision, FieldPatch, Fingerprint, ImportFormat, ImportPlan, ImportSource,
    Library, Profile, Record, RecordId,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceInput {
    pub id: RecordId,
    #[schemars(
        length(min = 1, max = 256),
        regex(pattern = r"^[^/\\\u0000-\u001f\u007f-\u009f]+$")
    )]
    pub name: String,
    pub format: ImportFormat,
    #[schemars(extend("x-max-utf8-bytes"=2097152))]
    pub contents: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Create {
        record: Box<Record>,
    },
    Edit {
        id: RecordId,
        expected: Fingerprint,
        patches: Vec<FieldPatch>,
        profile: Option<Profile>,
        #[serde(default)]
        key: Option<String>,
    },
    /// Replace one guarded native record prepared by a form; never changes its identity/provenance.
    Replace {
        id: RecordId,
        expected: Fingerprint,
        record: Box<Record>,
    },
    Rename {
        id: RecordId,
        expected: Fingerprint,
        key: String,
    },
    Remove {
        id: RecordId,
        expected: Fingerprint,
    },
    Import {
        source: SourceInput,
        preparation: Fingerprint,
        #[serde(default)]
        decisions: Vec<Decision>,
    },
}
pub fn metadata<'a>(
    state: &'a State,
    command: &str,
) -> Result<&'a crate::document_types::article::Metadata, Error> {
    match &state.settings.metadata {
        crate::document_types::Metadata::Article(meta) => Ok(meta),
        _ => Err(Error::CommandUnavailable {
            command: command.into(),
            reason: UnavailableReason::UnsupportedDocumentType,
        }),
    }
}
pub fn library(state: &State, command: &str) -> Result<Library, Error> {
    Library::new(metadata(state, command)?.references.clone()).map_err(Error::bibliography)
}
pub fn prepare(source: SourceInput) -> Result<bib::PreparedImport, Error> {
    let source = ImportSource::new(source.id, &source.name, source.format, source.contents)
        .map_err(Error::bibliography)?;
    let namespace = serde_json::json!([source.id(), source.digest()]).to_string();
    bib::prepare_import(source, |key| {
        let digest = Sha256::digest(serde_json::to_vec(&(namespace.as_str(), key)).unwrap());
        RecordId::new(format!(
            "ref-{}",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ))
        .unwrap()
    })
    .map_err(Error::bibliography)
}
pub fn plan(library: &Library, prepared: &bib::PreparedImport) -> Result<ImportPlan, Error> {
    let mut incoming = vec![];
    for record in &prepared.records {
        if let Some(existing) = library.get(record.id()) {
            if !record
                .provenance()
                .iter()
                .all(|p| existing.provenance().contains(p))
            {
                return Err(problem(
                    bib::Code::DuplicateIdentity,
                    "Import identity already belongs to another record",
                ));
            }
        } else {
            incoming.push(record.clone());
        }
    }
    ImportPlan::prepare(library, &incoming).map_err(Error::bibliography)
}
fn problem(code: bib::Code, detail: &str) -> Error {
    Error::bibliography(bib::Error {
        code,
        key: None,
        field: None,
        detail: detail.into(),
    })
}
pub fn apply(
    state: &State,
    expected: &Fingerprint,
    change: &Change,
) -> Result<crate::document_types::article::Metadata, Error> {
    let mut meta = metadata(state, "bibliography.apply")?.clone();
    let base = Library::new(meta.references.clone()).map_err(Error::bibliography)?;
    if &base.fingerprint() != expected {
        return Err(problem(
            bib::Code::StaleBase,
            "Bibliography changed since preparation",
        ));
    }
    let mut records = base.records().to_vec();
    match change {
        Change::Import {
            source,
            preparation,
            decisions,
        } => {
            let prepared = prepare(source.clone())?;
            if &prepared.fingerprint() != preparation {
                return Err(problem(
                    bib::Code::StaleBase,
                    "Import report changed since review",
                ));
            }
            if let Some(previous) = meta
                .reference_sources
                .iter()
                .find(|s| s.id() == prepared.source.id())
                && previous != &prepared.source
            {
                return Err(problem(
                    bib::Code::DuplicateIdentity,
                    "Import sources are immutable",
                ));
            }
            records = plan(&base, &prepared)?
                .resolve(decisions)
                .and_then(|p| p.apply(&base))
                .map_err(Error::bibliography)?
                .records()
                .to_vec();
            if records.iter().any(|r| {
                r.provenance()
                    .iter()
                    .any(|p| p.source_id == *prepared.source.id())
            }) && !meta
                .reference_sources
                .iter()
                .any(|s| s.id() == prepared.source.id())
            {
                meta.reference_sources.push(prepared.source);
            }
        }
        Change::Create { record } => {
            if !record.provenance().is_empty() {
                return Err(problem(
                    bib::Code::InvalidField,
                    "Import provenance must be assigned by an import plan",
                ));
            }
            if base.get(record.id()).is_some() {
                return Err(problem(
                    bib::Code::DuplicateIdentity,
                    "Record identity already exists",
                ));
            }
            records.push((**record).clone());
        }
        Change::Edit {
            id,
            expected,
            patches,
            profile,
            key,
        } => {
            let current = records
                .iter_mut()
                .find(|r| r.id() == id)
                .ok_or_else(|| problem(bib::Code::MissingRecord, "Record is missing"))?;
            let mut edited = current
                .edited(expected, patches)
                .map_err(Error::bibliography)?;
            if let Some(profile) = profile {
                edited = edited
                    .reclassified(&edited.fingerprint(), *profile)
                    .map_err(Error::bibliography)?;
            }
            if let Some(key) = key {
                edited = edited
                    .renamed(&edited.fingerprint(), key)
                    .map_err(Error::bibliography)?;
            }
            *current = edited;
        }
        Change::Replace {
            id,
            expected,
            record,
        } => {
            let current = records
                .iter_mut()
                .find(|r| r.id() == id)
                .ok_or_else(|| problem(bib::Code::MissingRecord, "Record is missing"))?;
            current.edited(expected, &[]).map_err(Error::bibliography)?;
            if record.id() != id || record.provenance() != current.provenance() {
                return Err(problem(
                    bib::Code::InvalidField,
                    "Replacing a notice must preserve its identity and immutable provenance",
                ));
            }
            *current = (**record).clone();
        }
        Change::Rename { id, expected, key } => {
            let current = records
                .iter_mut()
                .find(|r| r.id() == id)
                .ok_or_else(|| problem(bib::Code::MissingRecord, "Record is missing"))?;
            *current = current
                .renamed(expected, key)
                .map_err(Error::bibliography)?;
        }
        Change::Remove { id, expected } => {
            let current = base
                .get(id)
                .ok_or_else(|| problem(bib::Code::MissingRecord, "Record is missing"))?;
            if &current.fingerprint() != expected {
                return Err(problem(
                    bib::Code::StaleBase,
                    "Record changed since editing",
                ));
            }
            records.retain(|r| r.id() != id);
        }
    }
    meta.references = Library::new(records)
        .map_err(Error::bibliography)?
        .records()
        .to_vec();
    meta.reference_sources.retain(|s| {
        meta.references
            .iter()
            .any(|r| r.provenance().iter().any(|p| &p.source_id == s.id()))
    });
    super::validate_sources(&meta).map_err(|reason| Error::InvalidDocument { reason })?;
    Ok(meta)
}
