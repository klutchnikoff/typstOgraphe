//! Bibliographic values and pure operations shared by project and personal libraries.
//! Callers own identity allocation, persistence, document history and publication.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod classification;
mod diagnostic;
mod identity;
mod import;
mod keys;
mod model;
mod native;
mod plan;
mod profile;
mod search;

pub use diagnostic::{Code, Diagnostic, Error, Result, Severity};
pub use identity::{
    CandidateGroup, ConflictReason, Evidence, Signal, candidate_groups, evidence, normalize_arxiv,
    normalize_doi,
};
pub use import::{
    ImportFormat, ImportSource, PreparedImport, completeness, parse_native, prepare_import,
};
pub use keys::suggest_key;
pub use model::{FieldPatch, Fingerprint, Library, Provenance, Record, RecordId};
pub use plan::{Action, Decision, ImportPlan, PlanItem};
pub use profile::{FieldDescriptor, FieldKind, Profile, ProfileDescriptor, profiles};
pub use search::{EntrySummary, SearchQuery, normalize_text, search, summarize};

/// Imported source size, before parsing. Original bytes are retained by the caller.
pub const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
/// Maximum records in one library or import.
pub const MAX_RECORDS: usize = 10_000;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

pub mod form;
