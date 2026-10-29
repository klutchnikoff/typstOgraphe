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
        Error::InvalidDocument { .. } => "content-document-invalid",
        Error::InvalidSettings { reason } => match reason.as_str() {
            "template_too_large" => "model-too-large",
            "fixed_page" => "model-fixed-page",
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
