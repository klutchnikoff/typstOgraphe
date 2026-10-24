use super::*;
use crate::document::DocumentLanguage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpellingIssue {
    pub target: SelectionTarget,
    pub language: DocumentLanguage,
    pub word: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellingIssues {
    pub guard: TargetGuard,
    pub language: DocumentLanguage,
    pub available: bool,
    pub issues: Vec<SpellingIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpellingIssueParams {
    pub issue: SpellingIssue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpellingCorrectionParams {
    pub issue: SpellingIssue,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardParams {
    pub guard: TargetGuard,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SuggestionTarget {
    Word {
        language: DocumentLanguage,
        word: String,
    },
    Issue {
        issue: SpellingIssue,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpellingSuggestionsParams {
    pub target: SuggestionTarget,
}
