//! Actions documentaires et lectures orthographiques ; aucune préférence modifiée.
use super::*;
use crate::{
    document::DocumentLanguage,
    spelling::{self, Issue, WordLists},
};
use serde_json::json;
use std::{cell::RefCell, collections::BTreeSet};

#[derive(Default)]
pub(super) struct SpellingState {
    personal: WordLists,
    cache: RefCell<Cache>,
}
#[derive(Default)]
struct Cache {
    key: Option<(TargetGuard, DocumentLanguage, BTreeSet<String>)>,
    analysis: spelling::Analysis,
    issues: Vec<Issue>,
}

impl SpellingState {
    pub(super) fn fork(&self) -> Self {
        Self {
            personal: self.personal.clone(),
            cache: Default::default(),
        }
    }
}
impl Engine {
    /// Configuration de lecture : hors document, historique et commandes globales.
    pub fn configure_spelling(&mut self, personal: &WordLists) {
        if self.spelling.personal != *personal {
            self.spelling.personal = personal.clone();
        }
    }
    fn spelling_ranges(&self) -> Result<Vec<Issue>, CommandError> {
        let options = self.session.metadata.options();
        if !spelling::supported(options.language) {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "spelling_no_dictionary",
            ));
        }
        let accepted = spelling::accepted(
            options.language,
            &self.spelling.personal,
            &options.spelling_ignored,
        );
        let key = (self.session.target_guard(), options.language, accepted);
        let mut cache = self.spelling.cache.borrow_mut();
        if cache.key.as_ref() != Some(&key) {
            let issues = cache
                .analysis
                .analyze(&self.session.document.view.text, options.language, &key.2)
                .map_err(|e| CommandError::detail(ErrorCode::Document, "spelling_dictionary", e))?;
            cache.issues = issues;
            cache.key = Some(key);
        }
        Ok(cache.issues.clone())
    }
    pub(super) fn spelling_issues(&self) -> Result<SpellingIssues, CommandError> {
        let language = self.session.metadata.options().language;
        let available = spelling::supported(language);
        let mut issues = Vec::new();
        if available {
            for issue in self.spelling_ranges()? {
                let target = self
                    .session
                    .selection_target(Self::spelling_selection(&issue))?;
                issues.push(SpellingIssue {
                    target,
                    language,
                    word: issue.word,
                });
            }
        }
        Ok(SpellingIssues {
            guard: self.session.target_guard(),
            language,
            available,
            issues,
        })
    }
    pub(super) fn validate_spelling_issue(
        &self,
        issue: &SpellingIssue,
    ) -> Result<Selection, CommandError> {
        let selection = self.resolve(issue.target.into())?;
        if issue.language != self.session.metadata.options().language {
            return Err(CommandError::new(ErrorCode::Document, "spelling_language"));
        }
        if !self
            .spelling_ranges()?
            .iter()
            .any(|current| current.range == selection.range() && current.word == issue.word)
        {
            return Err(CommandError::new(ErrorCode::Document, "spelling_mismatch"));
        }
        Ok(selection)
    }
    pub(super) fn spelling_suggestions(
        &self,
        params: SpellingSuggestionsParams,
    ) -> Result<Value, CommandError> {
        let (language, word) = match params.target {
            SuggestionTarget::Word { language, word } => (language, word),
            SuggestionTarget::Issue { issue } => {
                self.validate_spelling_issue(&issue)?;
                (issue.language, issue.word)
            }
        };
        if !spelling::supported(language) {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "spelling_no_dictionary",
            ));
        }
        if !spelling::is_word(&word) {
            return Err(CommandError::new(ErrorCode::InvalidParams, "spelling_word"));
        }
        let suggestions = spelling::suggestions(language, &word)
            .map_err(|e| CommandError::detail(ErrorCode::Document, "spelling_dictionary", e))?;
        Ok(json!({"language":language,"word":word,"suggestions":suggestions}))
    }
    pub(super) fn check_spelling(&self, command: &Command) -> Result<(), CommandError> {
        match command {
            Command::NextSpelling(_) | Command::PreviousSpelling(_) => {
                if self.spelling_ranges()?.is_empty() {
                    return Err(CommandError::new(
                        ErrorCode::Unavailable,
                        "spelling_no_issues",
                    ));
                }
            }
            Command::CorrectSpelling(p) => {
                self.validate_spelling_issue(&p.issue)?;
                if p.replacement.chars().any(|c| {
                    c.is_control()
                        || matches!(c, crate::document::IMAGE_OBJECT | '\u{2028}' | '\u{2029}')
                }) {
                    return Err(CommandError::new(ErrorCode::InvalidParams, "invalid_text"));
                }
            }
            Command::IgnoreSpelling(p) => {
                self.validate_spelling_issue(&p.issue)?;
            }
            Command::ResetSpelling(p) => {
                self.check_guard(p.guard)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn navigate_spelling(
        &mut self,
        params: TargetParams,
        backward: bool,
    ) -> Result<(), CommandError> {
        let range = self.resolve(params.target)?.range();
        let issues = self.spelling_ranges()?;
        let issue = if backward {
            issues
                .iter()
                .rfind(|i| i.range.start < range.start)
                .or_else(|| issues.last())
        } else {
            issues
                .iter()
                .find(|i| i.range.start >= range.end)
                .or_else(|| issues.first())
        }
        .expect("availability checked");
        self.select_resolved(Self::spelling_selection(issue));
        Ok(())
    }
    pub(super) fn correct_spelling(
        &mut self,
        params: SpellingCorrectionParams,
    ) -> Result<(), CommandError> {
        self.selection = self
            .session
            .replace_text(params.issue.target, &params.replacement)?;
        Ok(())
    }
    pub(super) fn ignore_spelling(
        &mut self,
        params: SpellingIssueParams,
    ) -> Result<(), CommandError> {
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        candidate
            .metadata
            .options_mut()
            .spelling_ignored
            .entry(params.issue.language.code().into())
            .or_default()
            .insert(spelling::word_key(&params.issue.word));
        self.selection = self.session.commit_snapshot(
            self.session.target_guard(),
            before,
            candidate,
            self.selection,
            EditGroup::Discrete,
        )?;
        Ok(())
    }
    pub(super) fn reset_spelling(&mut self) -> Result<(), CommandError> {
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        candidate.metadata.options_mut().spelling_ignored.clear();
        self.selection = self.session.commit_snapshot(
            self.session.target_guard(),
            before,
            candidate,
            self.selection,
            EditGroup::Discrete,
        )?;
        Ok(())
    }
    fn spelling_selection(issue: &Issue) -> Selection {
        use crate::session::Caret;
        Selection {
            secondary: Caret {
                index: issue.range.start,
                ..Default::default()
            },
            primary: Caret {
                index: issue.range.end,
                ..Default::default()
            },
        }
    }
}
