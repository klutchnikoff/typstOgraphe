//! Erreurs du futur cœur : variantes typées, arguments et causes, sans Fluent.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Contrat de message dérivé des variantes ; aucune dépendance au traducteur.
pub struct MessageContract {
    pub key: &'static str,
    pub parameters: &'static [&'static str],
}

macro_rules! error_family {
    ($name:ident { $( $variant:ident $( { $( $field:ident : $ty:ty ),* } )? => $key:literal; )* }) => {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(tag = "kind", content = "params", rename_all = "snake_case")]
        pub enum $name { $( $variant $( { $( $field: $ty ),* } )?, )* }
        impl $name {
            fn contracts() -> Vec<MessageContract> {
                vec![$( MessageContract { key: $key, parameters: &[ $( $( stringify!($field), )* )? ] }, )*]
            }
            pub fn message_key(&self) -> &'static str {
                match self { $( Self::$variant $( { $( $field: _ ),* } )? => $key, )* }
            }
            pub fn arguments(&self) -> BTreeMap<String, String> {
                match self { $( Self::$variant $( { $( $field ),* } )? => {
                    let fields = [ $( $( (stringify!($field).to_owned(), $field.to_string()), )* )? ];
                    BTreeMap::from(fields)
                }, )* }
            }
            #[cfg(test)]
            pub(crate) fn samples() -> Vec<CoreError> {
                vec![$( Self::$variant $( { $( $field: <$ty>::default() ),* } )?.into(), )*]
            }
        }
        impl From<$name> for CoreError { fn from(error: $name) -> Self { Self::$name(error) } }
    };
}

error_family! { ExportError {
    Folder => "export-folder";
    Nonempty => "export-nonempty";
    WriteFailed { name: String, error: String } => "export-write-failed";
} }

error_family! { PackageError {
    InventoryInvalid => "package-inventory-invalid";
    ResourceMissing { path: String } => "package-resource-missing";
} }

error_family! { RecoveryError {
    WorkerStopped => "recovery-worker-stopped";
} }

error_family! { SourceError {
    ComputedCall => "source-computed-call";
    HeadingDepth => "source-heading-depth";
    ImageSignature => "source-image-signature";
    LetterNoteSignature => "source-letter-note-signature";
    NestedBlocks => "source-nested-blocks";
    QuoteSignature => "source-quote-signature";
    SyntaxError => "source-syntax-error";
    Unreadable => "source-unreadable";
    UnsupportedFunction { name: String } => "source-unsupported-function";
    UnsupportedNode { kind: String } => "source-unsupported-node";
} }

error_family! { SpellingError {
    NoDictionary { language: String } => "spelling-no-dictionary-detail";
} }

error_family! { StorageError {
    EntryMissing { name: String, error: String } => "storage-entry-missing";
    EntryTooLarge { name: String } => "storage-entry-too-large";
    EntryUnreadable { name: String, error: String } => "storage-entry-unreadable";
    ExpandedTooLarge => "storage-expanded-too-large";
    FileCount => "storage-file-count";
    FileTooLarge => "storage-file-too-large";
    ImageResourceMissing { path: String } => "image-resource-missing";
    Invalid { error: Box<CoreError> } => "storage-invalid";
    InvalidSize => "storage-invalid-size";
    InvalidTemplate => "storage-invalid-template";
    LetterTooLarge => "storage-letter-too-large";
    MainMismatch => "storage-main-mismatch";
    ManifestUnreadable { error: String } => "storage-manifest-unreadable";
    MetadataUnreadable { error: String } => "storage-metadata-unreadable";
    OpenFailed { error: String } => "storage-open-failed";
    ReplaceFailed { error: String } => "storage-replace-failed";
    TemporaryFailed { error: String } => "storage-temporary-failed";
    UnknownEntry => "storage-unknown-entry";
    Unsupported => "storage-unsupported";
    ZipUnreadable { error: String } => "storage-zip-unreadable";
} }

error_family! { TemplateError {
    BulletMarkerNotAllowed => "model-bullet-marker-not-allowed";
    Compilation { error: String } => "core-external-error";
    ContractMissing { function: String } => "model-contract-missing";
    InvalidEditorOptions => "model-invalid-editor-options";
} }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "domain", content = "error", rename_all = "snake_case")]
pub enum CoreError {
    ExportError(ExportError),
    PackageError(PackageError),
    RecoveryError(RecoveryError),
    SourceError(SourceError),
    SpellingError(SpellingError),
    StorageError(StorageError),
    TemplateError(TemplateError),
    Image(crate::assets::ImageError),
    External { error: String },
}
impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for CoreError {}
impl From<crate::assets::ImageError> for CoreError {
    fn from(error: crate::assets::ImageError) -> Self {
        Self::Image(error)
    }
}
impl CoreError {
    pub fn message_contracts() -> Vec<MessageContract> {
        let mut contracts = Vec::new();
        contracts.extend(ExportError::contracts());
        contracts.extend(PackageError::contracts());
        contracts.extend(RecoveryError::contracts());
        contracts.extend(SourceError::contracts());
        contracts.extend(SpellingError::contracts());
        contracts.extend(StorageError::contracts());
        contracts.extend(TemplateError::contracts());
        for key in [
            "image-too-large",
            "image-format-unsupported",
            "image-invalid-svg",
        ] {
            contracts.push(MessageContract {
                key,
                parameters: &[],
            });
        }
        contracts.push(MessageContract {
            key: "core-external-error",
            parameters: &["error"],
        });
        contracts
    }
    pub fn external(error: impl std::fmt::Display) -> Self {
        Self::External {
            error: error.to_string(),
        }
    }
    pub fn message_key(&self) -> &'static str {
        match self {
            Self::ExportError(e) => e.message_key(),
            Self::PackageError(e) => e.message_key(),
            Self::RecoveryError(e) => e.message_key(),
            Self::SourceError(e) => e.message_key(),
            Self::SpellingError(e) => e.message_key(),
            Self::StorageError(e) => e.message_key(),
            Self::TemplateError(e) => e.message_key(),
            Self::Image(e) => match e {
                crate::assets::ImageError::TooLarge => "image-too-large",
                crate::assets::ImageError::Unsupported => "image-format-unsupported",
                crate::assets::ImageError::InvalidSvg => "image-invalid-svg",
                _ => "core-external-error",
            },
            Self::External { .. } => "core-external-error",
        }
    }
    pub fn arguments(&self) -> BTreeMap<String, String> {
        match self {
            Self::ExportError(e) => e.arguments(),
            Self::PackageError(e) => e.arguments(),
            Self::RecoveryError(e) => e.arguments(),
            Self::SourceError(e) => e.arguments(),
            Self::SpellingError(e) => e.arguments(),
            Self::StorageError(e) => e.arguments(),
            Self::TemplateError(e) => e.arguments(),
            Self::Image(
                crate::assets::ImageError::Decode(error) | crate::assets::ImageError::Io(error),
            )
            | Self::External { error } => BTreeMap::from([("error".into(), error.clone())]),
            Self::Image(_) => BTreeMap::new(),
        }
    }
    pub fn cause(&self) -> Option<&CoreError> {
        match self {
            Self::StorageError(StorageError::Invalid { error }) => Some(error),
            _ => None,
        }
    }
    #[cfg(test)]
    pub(crate) fn samples() -> Vec<Self> {
        let mut samples = Vec::new();
        samples.extend(ExportError::samples());
        samples.extend(PackageError::samples());
        samples.extend(RecoveryError::samples());
        samples.extend(SourceError::samples());
        samples.extend(SpellingError::samples());
        samples.extend(StorageError::samples());
        samples.extend(TemplateError::samples());
        samples.extend([
            Self::Image(crate::assets::ImageError::TooLarge),
            Self::Image(crate::assets::ImageError::Unsupported),
            Self::Image(crate::assets::ImageError::InvalidSvg),
            Self::Image(crate::assets::ImageError::Decode("decoder".into())),
            Self::Image(crate::assets::ImageError::Io("io".into())),
            Self::external("external"),
        ]);
        samples
    }
}
#[cfg(test)]
impl Default for CoreError {
    fn default() -> Self {
        Self::external("cause")
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn typed_errors_round_trip_with_arguments_and_nested_causes_without_localization() {
        for error in CoreError::samples() {
            let value = serde_json::to_value(&error).unwrap();
            let restored: CoreError = serde_json::from_value(value).unwrap();
            assert_eq!(restored, error);
            assert!(CoreError::message_contracts().iter().any(|contract| {
                contract.key == error.message_key()
                    && contract
                        .parameters
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                        == error.arguments().keys().map(String::as_str).collect()
            }));
        }
        let error = CoreError::from(StorageError::Invalid {
            error: Box::new(
                SourceError::UnsupportedFunction {
                    name: "inconnue".into(),
                }
                .into(),
            ),
        });
        assert!(
            matches!(error.cause(),Some(CoreError::SourceError(SourceError::UnsupportedFunction { name })) if name == "inconnue")
        );
    }
}
