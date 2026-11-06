//! Presentation of structured document and archive errors. Machine causes stay in the core.
use crate::{command_error_with, error_with};
use i18n_embed::fluent::FluentLanguageLoader;
use std::collections::HashMap;
use typstographe_core::content::{archive, contract::Error};

pub fn content_error_with(loader: &FluentLanguageLoader, error: &Error) -> String {
    let arg = |key, name, value: String| loader.get_args(key, HashMap::from([(name, value)]));
    let key = match error {
        Error::BatchFailed { index, cause } => {
            return loader.get_args(
                "engine-batch-step-error",
                HashMap::from([
                    ("step", (index + 1).to_string()),
                    ("error", content_error_with(loader, cause)),
                ]),
            );
        }
        Error::InvalidParameters { parameter } => {
            return arg("content-invalid-parameter", "parameter", parameter.clone());
        }
        Error::Bibliography { error } => {
            use typstographe_core::bibliography::NoticeErrorCode as Code;
            match error.code {
                Code::InvalidIdentity | Code::DuplicateIdentity => "content-bibliography-identity",
                Code::InvalidKey => "content-bibliography-key",
                Code::DuplicateKey => {
                    return arg(
                        "content-bibliography-key-used",
                        "key",
                        error.key.clone().unwrap_or_default(),
                    );
                }
                Code::InvalidSyntax => "content-bibliography-source",
                Code::InvalidField | Code::UnsupportedField => {
                    return arg(
                        "content-bibliography-field",
                        "field",
                        error.field.clone().unwrap_or_else(|| "…".into()),
                    );
                }
                Code::LimitExceeded => "content-bibliography-limit",
                Code::MissingRecord => "content-bibliography-missing",
                Code::StaleBase => "content-bibliography-stale",
                Code::UnresolvedConflict | Code::InvalidDecision => "content-bibliography-decision",
                _ => "content-bibliography-invalid",
            }
        }
        Error::CommandUnavailable {
            command,
            reason: typstographe_core::content::contract::UnavailableReason::UnsupportedDocumentType,
        } => {
            return arg("content-type-unavailable", "command", command.clone());
        }
        Error::CommandUnavailable { command, .. } => {
            return arg("content-command-unavailable", "command", command.clone());
        }
        Error::UnsupportedContractVersion { version } => {
            return arg("content-contract-version", "version", version.to_string());
        }
        Error::EditingContext { reason } => match reason.as_str() {
            "closed" => "engine-edit-context-closed",
            "pending_draft" => "content-context-pending",
            "selection_required" | "text_selection_required" => "content-context-selection",
            "history_conflict" => "content-history-conflict",
            _ => "content-state-mismatch",
        },
        Error::InvalidTarget { .. } => "content-invalid-target",
        Error::InvalidSharedState { .. } | Error::IncompleteSharedState => {
            "content-shared-state-invalid"
        }
        Error::InvalidDocument { reason } if reason == "missing_signature" => {
            "content-signature-missing"
        }
        Error::InvalidDocument { reason } if reason == "cross_reference_target_missing" => {
            "content-reference-target-missing"
        }
        Error::InvalidDocument { reason } if reason == "invalid_equation_alias" => {
            "content-equation-alias-invalid"
        }
        Error::InvalidDocument { reason } if reason == "duplicate_equation_alias" => {
            "content-equation-alias-duplicate"
        }
        Error::InvalidDocument { reason } if reason.starts_with("bibliography_record_missing:") => {
            "content-bibliography-cited-missing"
        }
        Error::InvalidDocument { .. } => "content-document-invalid",
        Error::InvalidSettings { reason } => match reason.as_str() {
            "template_too_large" => "model-too-large",
            "model_language_unsupported" => "model-language-unsupported",
            value if value.starts_with("model_option_not_allowed:") => "model-option-not-allowed",
            _ => "content-settings-invalid",
        },
        Error::InconsistentState { field } => match field.as_str() {
            "document" => "content-document-mismatch",
            "settings" => "content-settings-mismatch",
            "source" => "content-source-mismatch",
            "bibliography" => "content-bibliography-mismatch",
            _ => "content-state-mismatch",
        },
        Error::WrongDocument | Error::StaleRevision { .. } => "edit-stale-draft",
        Error::StateLimitExceeded => "storage-letter-too-large",
    };
    loader.get(key)
}

pub fn archive_error_with(loader: &FluentLanguageLoader, error: &archive::Error) -> String {
    let detail = match error {
        archive::Error::Content(error) => content_error_with(loader, error),
        archive::Error::Core(error) => error_with(loader, error),
        archive::Error::Access(error) => command_error_with(loader, error),
    };
    loader.get_args("storage-project-error", HashMap::from([("error", detail)]))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod bibliography_tests {
    use super::*;
    use typstographe_core::bibliography::{NoticeError, NoticeErrorCode as Code};
    #[test]
    fn bibliographic_causes_have_translated_labels_and_keep_machine_details_separate() {
        for (language, word) in [("fr", "clé"), ("en", "key"), ("it", "key")] {
            let loader = crate::loader_for(&[language.parse().unwrap()]);
            let error = Error::Bibliography {
                error: NoticeError {
                    code: Code::DuplicateKey,
                    key: Some("existing2026".into()),
                    field: Some("key".into()),
                    detail: "Untranslated technical detail".into(),
                },
            };
            let payload = serde_json::to_value(&error).unwrap();
            let message = content_error_with(&loader, &error);
            assert!(
                message.contains(word) && message.contains("existing2026"),
                "{message}"
            );
            assert!(!message.contains("technical detail"));
            assert_eq!(serde_json::to_value(&error).unwrap(), payload);
            let field = Error::Bibliography {
                error: NoticeError {
                    code: Code::UnsupportedField,
                    key: None,
                    field: Some("/title/unknown".into()),
                    detail: "Technical data".into(),
                },
            };
            let message = content_error_with(&loader, &field);
            assert!(message.contains("/title/unknown"));
            assert!(!message.contains("Technical data"));
            for code in [
                Code::InvalidIdentity,
                Code::DuplicateIdentity,
                Code::InvalidKey,
                Code::InvalidSyntax,
                Code::LimitExceeded,
                Code::MissingRecord,
                Code::StaleBase,
                Code::UnresolvedConflict,
                Code::InvalidDecision,
                Code::TypeMapping,
            ] {
                let error = Error::Bibliography {
                    error: NoticeError {
                        code,
                        key: None,
                        field: None,
                        detail: "Technical data".into(),
                    },
                };
                let message = content_error_with(&loader, &error);
                assert!(
                    !message.is_empty()
                        && !message.contains("content-bibliography-")
                        && !message.contains("Technical data"),
                    "{message}"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_causes_and_nested_batches_are_localized_with_english_fallback() {
        let structured = Error::BatchFailed {
            index: 1,
            cause: Box::new(Error::InvalidParameters {
                parameter: "rows".into(),
            }),
        };
        let payload = serde_json::to_value(&structured).unwrap();
        for (language, phrase) in [
            ("fr", "paramètre"),
            ("en", "parameter"),
            ("de-DE", "parameter"),
        ] {
            let loader = crate::loader_for(&[language.parse().unwrap()]);
            let message = content_error_with(&loader, &structured);
            assert!(message.contains(phrase), "{message}");
            assert!(message.contains("rows") && message.contains('2'));
            let unavailable = Error::CommandUnavailable {
                command: "letter.insert_postscript".into(),
                reason:
                    typstographe_core::content::contract::UnavailableReason::UnsupportedDocumentType,
            };
            let message = content_error_with(&loader, &unavailable);
            assert!(message.contains("type") && message.contains("letter.insert_postscript"));
            let error = archive::Error::Core(
                typstographe_core::errors::StorageError::MissingEntry {
                    name: "document.json".into(),
                }
                .into(),
            );
            let message = archive_error_with(&loader, &error);
            assert!(message.contains("document.json"));
            assert!(!message.contains("StorageError") && !message.contains("MissingEntry"));
        }
        assert_eq!(serde_json::to_value(&structured).unwrap(), payload);
        for (language, phrase) in [
            ("fr", "ne correspondent pas"),
            ("en", "do not match"),
            ("it", "do not match"),
        ] {
            let loader = crate::loader_for(&[language.parse().unwrap()]);
            let message = archive_error_with(
                &loader,
                &archive::Error::Content(Error::InconsistentState {
                    field: "settings".into(),
                }),
            );
            assert!(message.contains(phrase), "{message}");
        }
    }
}

#[cfg(test)]
mod editing_tests {
    use super::*;
    #[test]
    fn context_and_history_errors_use_fluent_in_both_languages_and_fallback() {
        for language in ["fr", "en", "de"] {
            let loader = crate::loader_for(&[language.parse().unwrap()]);
            for reason in [
                "closed",
                "pending_draft",
                "selection_required",
                "text_selection_required",
                "history_conflict",
            ] {
                let message = content_error_with(
                    &loader,
                    &Error::EditingContext {
                        reason: reason.into(),
                    },
                );
                assert!(
                    !message.contains(reason) && !message.contains("EditingContext"),
                    "{message}"
                );
                assert!(!message.is_empty());
            }
        }
    }
}
