//! Orthographe locale : dictionnaires embarqués, analyse et suggestions hors UI.
use crate::document::DocumentLanguage;
use crate::errors::{CoreError, SpellingError};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::OnceLock,
};
use unicode_normalization::UnicodeNormalization;

pub type WordLists = BTreeMap<String, BTreeSet<String>>;

pub fn supported(language: DocumentLanguage) -> bool {
    matches!(
        language,
        DocumentLanguage::French | DocumentLanguage::English
    )
}

/// Les mots personnels/ignorés acceptent les variantes de casse et d’apostrophe.
pub fn word_key(word: &str) -> String {
    word.nfc()
        .collect::<String>()
        .to_lowercase()
        .replace(['’', 'ʼ'], "'")
        .replace(['‐', '‑'], "-")
}

pub fn accepted(
    language: DocumentLanguage,
    personal: &WordLists,
    ignored: &WordLists,
) -> BTreeSet<String> {
    personal
        .get(language.code())
        .into_iter()
        .flatten()
        .chain(ignored.get(language.code()).into_iter().flatten())
        .map(|word| word_key(word))
        .collect()
}

fn dictionary(language: DocumentLanguage) -> Result<&'static spellbook::Dictionary, CoreError> {
    static FR: OnceLock<Result<spellbook::Dictionary, CoreError>> = OnceLock::new();
    static EN: OnceLock<Result<spellbook::Dictionary, CoreError>> = OnceLock::new();
    let (cell, aff, dic) = match language {
        DocumentLanguage::French => (
            &FR,
            include_str!("../assets/dictionaries/fr.aff"),
            include_str!("../assets/dictionaries/fr.dic"),
        ),
        DocumentLanguage::English => (
            &EN,
            include_str!("../assets/dictionaries/en_US.aff"),
            include_str!("../assets/dictionaries/en_US.dic"),
        ),
        _ => {
            return Err(CoreError::from(SpellingError::NoDictionary {
                language: language.code().into(),
            }));
        }
    };
    cell.get_or_init(|| spellbook::Dictionary::new(aff, dic).map_err(CoreError::external))
        .as_ref()
        .map_err(Clone::clone)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    /// Indices de caractères Unicode de la projection éditable, jamais d’octets.
    pub range: Range<usize>,
    pub word: String,
}

fn expressions() -> &'static (regex::Regex, regex::Regex) {
    static EXPRESSIONS: OnceLock<(regex::Regex, regex::Regex)> = OnceLock::new();
    EXPRESSIONS.get_or_init(|| (
        regex::Regex::new(r"[\p{Alphabetic}\p{M}\p{N}]+(?:['’ʼ‐‑-][\p{Alphabetic}\p{M}\p{N}]+)*").unwrap(),
        regex::Regex::new(r"(?i)(?:https?://|www\.|mailto:)\S+|[\p{L}\p{N}._%+\-]+@[\p{L}\p{N}.\-]+\.[\p{L}]{2,}").unwrap(),
    ))
}

fn check_line(dictionary: &spellbook::Dictionary, line: &str) -> Vec<Issue> {
    let (words, addresses) = expressions();
    let addresses: Vec<_> = addresses
        .find_iter(line)
        .map(|found| found.range())
        .collect();
    let mut issues = Vec::new();
    let mut byte = 0;
    let mut character = 0;
    for found in words.find_iter(line) {
        character += line[byte..found.start()].chars().count();
        let start = character;
        character += found.as_str().chars().count();
        byte = found.end();
        let word = found.as_str();
        if word.chars().any(char::is_numeric)
            || addresses
                .iter()
                .any(|range| range.start <= found.start() && found.end() <= range.end)
        {
            continue;
        }
        if !dictionary.check(&dictionary_word(word)) {
            issues.push(Issue {
                range: start..character,
                word: word.into(),
            });
        }
    }
    issues
}

fn dictionary_word(word: &str) -> String {
    word.nfc().collect::<String>().replace(['‐', '‑'], "-")
}

pub fn suggestions(language: DocumentLanguage, word: &str) -> Result<Vec<String>, CoreError> {
    let mut suggestions = Vec::new();
    dictionary(language)?.suggest(&dictionary_word(word), &mut suggestions);
    if word.contains('’') {
        for suggestion in &mut suggestions {
            *suggestion = suggestion.replace('\'', "’");
        }
    }
    suggestions.dedup();
    suggestions.truncate(8);
    Ok(suggestions)
}

pub(crate) fn is_word(word: &str) -> bool {
    expressions()
        .0
        .find(word)
        .is_some_and(|found| found.range() == (0..word.len()))
        && !word.chars().any(char::is_numeric)
}

/// Analyse synchrone réutilisable par un adaptateur qui possède déjà son worker.
/// Les positions restent des indices Unicode scalaires, comme le contrat existant.
pub fn check_text(
    language: DocumentLanguage,
    text: &str,
    accepted: &BTreeSet<String>,
) -> Result<Vec<Issue>, CoreError> {
    let dictionary = dictionary(language)?;
    let mut result = vec![];
    let mut offset = 0;
    for line in text.split('\n') {
        for mut issue in check_line(dictionary, line) {
            if !accepted.contains(&word_key(&issue.word)) {
                issue.range.start += offset;
                issue.range.end += offset;
                result.push(issue)
            }
        }
        offset += line.chars().count() + 1;
    }
    Ok(result)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn bundled_dictionaries_check_french_unicode_compounds_and_suggest_corrections() {
        let fr = dictionary(DocumentLanguage::French).unwrap();
        for word in [
            "Bonjour",
            "réservation",
            "l’école",
            "aujourd’hui",
            "rendez-vous",
            "compte-rendu",
            "Été",
            "e\u{301}cole",
        ] {
            assert!(fr.check(word), "mot français refusé : {word}");
        }
        assert!(!fr.check("courrrier"));
        assert!(
            suggestions(DocumentLanguage::French, "courrrier")
                .unwrap()
                .contains(&"courrier".into())
        );
        let en = dictionary(DocumentLanguage::English).unwrap();
        for word in ["Hello", "meeting", "don't", "don’t"] {
            assert!(en.check(word), "{word}");
        }
        assert!(
            suggestions(DocumentLanguage::English, "helllo")
                .unwrap()
                .contains(&"hello".into())
        );
    }

    #[test]
    fn analysis_preserves_unicode_offsets_excludes_addresses_and_applies_exceptions() {
        let text = "Été 🌳 courrrier\n/ Bonjour\tbonjouur\n\u{fffc} légende\nrendez‑vous M. Mme etc. aujourd’hui\nhttps://courrrier.example/courrrier nom@courrrier.example 2026 ABC123";
        let issues = check_text(DocumentLanguage::French, text, &BTreeSet::new()).unwrap();
        assert_eq!(
            issues
                .iter()
                .map(|issue| issue.word.as_str())
                .collect::<Vec<_>>(),
            ["courrrier", "bonjouur"]
        );
        for issue in &issues {
            assert_eq!(
                text.chars()
                    .skip(issue.range.start)
                    .take(issue.range.len())
                    .collect::<String>(),
                issue.word
            );
        }
        let ignored = BTreeSet::from(["courrrier".into()]);
        assert_eq!(
            check_text(DocumentLanguage::French, text, &ignored)
                .unwrap()
                .len(),
            1
        );
        assert!(
            check_text(DocumentLanguage::French, "Bonjour", &ignored)
                .unwrap()
                .is_empty()
        );
    }
}
