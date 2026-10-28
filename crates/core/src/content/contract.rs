//! Values shared by native and headless adapters. No UI or transport types.
use super::{Node, source};
use crate::{
    assets::Assets,
    document_types::{DocumentType, Metadata},
};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

/// Stable error codes and data; adapters own their presentation and translation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Error {
    #[error("unsupported_contract_version: {version}")]
    UnsupportedContractVersion { version: u32 },
    #[error("invalid_shared_state: {reason}")]
    InvalidSharedState { reason: String },
    #[error("incomplete_shared_state")]
    IncompleteSharedState,
    #[error("invalid_document: {reason}")]
    InvalidDocument { reason: String },
    #[error("invalid_settings: {reason}")]
    InvalidSettings { reason: String },
    #[error("inconsistent_state: {field}")]
    InconsistentState { field: String },
    #[error("wrong_document")]
    WrongDocument,
    #[error("stale_revision: expected {expected}, actual {actual}")]
    StaleRevision { expected: u64, actual: u64 },
    #[error("state_limit_exceeded")]
    StateLimitExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub metadata: Metadata,
    pub template: String,
    pub model_id: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            metadata: Metadata::demo(DocumentType::Courrier),
            template: DocumentType::Courrier.default_model(),
            model_id: "courrier.model.personal".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), Error> {
        let invalid = |reason: &str| Error::InvalidSettings {
            reason: reason.into(),
        };
        self.metadata
            .validate()
            .map_err(|reason| Error::InvalidDocument { reason })?;
        if self.template.len() > 2 * 1024 * 1024 {
            return Err(invalid("template_too_large"));
        }
        // Embedded models are checked against the Typst contract by the renderer.
        if self.model_id == "embedded" {
            return Ok(());
        }
        let model = self
            .metadata
            .document_type()
            .models()
            .find(|m| m.model.id() == self.model_id)
            .ok_or_else(|| invalid("unknown_model"))?;
        if model.source != self.template {
            return Err(invalid("model_source_mismatch"));
        }
        if let Some((paper, landscape)) = model.page
            && (self.metadata.options().paper != paper
                || self.metadata.options().landscape != landscape)
        {
            return Err(invalid("fixed_page"));
        }
        Ok(())
    }
}

/// Adapter snapshot retained during migration. The confirmed state uses explicit settings.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub revision: u64,
    pub document: Node,
    #[serde(default)]
    pub settings: Option<Settings>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub version: u32,
    pub document: Node,
    pub settings: Settings,
}
impl State {
    pub fn new(document: Node, settings: Settings) -> Self {
        Self {
            version: VERSION,
            document,
            settings,
        }
    }
    /// Structural validity is separate from metadata.required_for_export.
    pub fn validate(&self, assets: &Assets) -> Result<String, Error> {
        if self.version != VERSION {
            return Err(Error::UnsupportedContractVersion {
                version: self.version,
            });
        }
        self.settings.validate()?;
        crate::bibliography::validate_document(&self.document, &self.settings.metadata)
            .map_err(|reason| Error::InvalidDocument { reason })?;
        if self
            .settings
            .metadata
            .signature_path()
            .is_some_and(|path| !assets.contains_key(path))
        {
            return Err(Error::InvalidDocument {
                reason: "missing_signature".into(),
            });
        }
        source(&self.document, assets).map_err(|reason| Error::InvalidDocument { reason })
    }
}

/// Session identity and revision, never a persistent file path or a Yjs client ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub document: String,
    pub revision: u64,
}
impl Guard {
    pub fn check(&self, current: &Self) -> Result<(), Error> {
        if self.document != current.document {
            Err(Error::WrongDocument)
        } else if self.revision != current.revision {
            Err(Error::StaleRevision {
                expected: self.revision,
                actual: current.revision,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_editorial_data_is_a_valid_draft_but_unknown_contracts_are_refused() {
        let settings = Settings {
            metadata: Metadata::blank(DocumentType::Courrier),
            ..Settings::default()
        };
        let document = serde_json::from_value(
            serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}),
        )
        .unwrap();
        let mut state = State::new(document, settings);
        assert!(!state.settings.metadata.ready_for_export());
        state.validate(&Assets::default()).unwrap();
        state.version += 1;
        assert!(matches!(
            state.validate(&Assets::default()),
            Err(Error::UnsupportedContractVersion { .. })
        ));
    }

    #[test]
    fn installed_model_identity_cannot_describe_a_different_source() {
        let mut settings = Settings::default();
        settings
            .template
            .push_str("\n// A changed model belongs to the project.");
        assert_eq!(
            settings.validate().unwrap_err(),
            Error::InvalidSettings {
                reason: "model_source_mismatch".into()
            }
        );
        settings.model_id = "embedded".into();
        settings.validate().unwrap();
    }
}
