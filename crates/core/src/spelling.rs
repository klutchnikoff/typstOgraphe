//! Orthographe locale : dictionnaires embarqués, analyse et suggestions hors UI.
use crate::document::DocumentLanguage;
use crate::errors::{CoreError, SpellingError};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Range,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use unicode_normalization::UnicodeNormalization;

pub const DELAY: Duration = Duration::from_millis(400);
pub type WordLists = BTreeMap<String, BTreeSet<String>>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub enabled: bool,
    pub personal: WordLists,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            enabled: true,
            personal: BTreeMap::new(),
        }
    }
}

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

/// Cache limité aux lignes du document courant ; une modification locale ne
/// relance pas les recherches Hunspell sur les passages inchangés.
#[derive(Default)]
pub(crate) struct Analysis {
    language: Option<DocumentLanguage>,
    lines: HashMap<String, Vec<Issue>>,
}
impl Analysis {
    pub(crate) fn analyze(
        &mut self,
        text: &str,
        language: DocumentLanguage,
        accepted: &BTreeSet<String>,
    ) -> Result<Vec<Issue>, CoreError> {
        self.check(
            &Request {
                ticket: 0,
                text: text.into(),
                language,
                accepted: accepted.clone(),
            },
            &AtomicU64::new(0),
        )
    }
    fn check(
        &mut self,
        request: &Request,
        generation: &AtomicU64,
    ) -> Result<Vec<Issue>, CoreError> {
        let dictionary = dictionary(request.language)?;
        if self.language != Some(request.language) {
            self.lines.clear();
            self.language = Some(request.language);
        }
        let mut current = HashMap::new();
        let mut issues = Vec::new();
        let mut offset = 0;
        for line in request.text.split_inclusive('\n') {
            if generation.load(Ordering::Relaxed) != request.ticket {
                return Ok(vec![]);
            }
            let local = current.entry(line.to_owned()).or_insert_with(|| {
                self.lines
                    .remove(line)
                    .unwrap_or_else(|| check_line(dictionary, line))
            });
            for issue in local.iter() {
                if !request.accepted.contains(&word_key(&issue.word)) {
                    issues.push(Issue {
                        range: issue.range.start + offset..issue.range.end + offset,
                        word: issue.word.clone(),
                    });
                }
            }
            offset += line.chars().count();
        }
        self.lines = current;
        Ok(issues)
    }
}

pub(crate) fn suggestions(
    language: DocumentLanguage,
    word: &str,
) -> Result<Vec<String>, CoreError> {
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

#[derive(Clone, PartialEq, Eq)]
struct Key {
    revision: u64,
    language: DocumentLanguage,
    accepted: BTreeSet<String>,
    enabled: bool,
}
struct Request {
    ticket: u64,
    text: String,
    language: DocumentLanguage,
    accepted: BTreeSet<String>,
}
enum Task {
    Check(Request),
    Suggest(u64, DocumentLanguage, String),
    #[cfg(any(test, feature = "test-support"))]
    Barrier(mpsc::Sender<()>),
}
enum Response {
    Checked(u64, Result<Vec<Issue>, CoreError>),
    Suggested(u64, String, Result<Vec<String>, CoreError>),
}

pub struct Checker {
    sender: mpsc::Sender<Task>,
    receiver: mpsc::Receiver<Response>,
    generation: Arc<AtomicU64>,
    key: Option<Key>,
    pending: Option<(Instant, Request)>,
    pub issues: Vec<Issue>,
    pub ready: bool,
    pub error: Option<CoreError>,
    pub suggestions: Option<(String, Option<Vec<String>>)>,
}
impl Checker {
    pub fn is_current(&self, revision: u64) -> bool {
        self.ready
            && self
                .key
                .as_ref()
                .is_some_and(|key| key.revision == revision && key.enabled)
    }
    pub fn new(notify: impl Fn() + Send + 'static) -> Self {
        let (sender, tasks) = mpsc::channel();
        let (results, receiver) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let current = generation.clone();
        std::thread::Builder::new()
            .name("orthographe".into())
            .spawn(move || {
                let mut analysis = Analysis::default();
                while let Ok(task) = tasks.recv() {
                    let response = match task {
                        #[cfg(any(test, feature = "test-support"))]
                        Task::Barrier(sender) => {
                            let _ = sender.send(());
                            continue;
                        }
                        Task::Check(request) => {
                            if current.load(Ordering::Relaxed) != request.ticket {
                                continue;
                            }
                            Response::Checked(request.ticket, analysis.check(&request, &current))
                        }
                        Task::Suggest(ticket, language, word) => {
                            if current.load(Ordering::Relaxed) != ticket {
                                continue;
                            }
                            let values = suggestions(language, &word);
                            Response::Suggested(ticket, word, values)
                        }
                    };
                    if results.send(response).is_err() {
                        break;
                    }
                    notify();
                }
            })
            .expect("création du thread d’orthographe");
        Self {
            sender,
            receiver,
            generation,
            key: None,
            pending: None,
            issues: vec![],
            ready: false,
            error: None,
            suggestions: None,
        }
    }
    pub fn synchronize(
        &mut self,
        revision: u64,
        text: &str,
        language: DocumentLanguage,
        accepted: BTreeSet<String>,
        enabled: bool,
        now: Instant,
    ) {
        let key = Key {
            revision,
            language,
            accepted,
            enabled,
        };
        if self.key.as_ref() != Some(&key) {
            let ticket = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
            self.issues.clear();
            self.ready = false;
            self.error = None;
            self.suggestions = None;
            self.pending = (enabled && supported(language)).then(|| {
                (
                    now + DELAY,
                    Request {
                        ticket,
                        text: text.into(),
                        language,
                        accepted: key.accepted.clone(),
                    },
                )
            });
            self.key = Some(key);
        }
        if self.pending.as_ref().is_some_and(|(due, _)| now >= *due) {
            let (_, request) = self.pending.take().unwrap();
            if let Err(error) = self.sender.send(Task::Check(request)) {
                self.error = Some(CoreError::external(error));
            }
        }
        self.poll();
    }
    fn poll(&mut self) {
        while let Ok(response) = self.receiver.try_recv() {
            let ticket = self.generation.load(Ordering::Relaxed);
            match response {
                Response::Checked(id, result) if id == ticket => match result {
                    Ok(issues) => {
                        self.issues = issues;
                        self.ready = true;
                    }
                    Err(error) => self.error = Some(error),
                },
                Response::Suggested(id, word, result)
                    if id == ticket
                        && self
                            .suggestions
                            .as_ref()
                            .is_some_and(|(wanted, _)| *wanted == word) =>
                {
                    match result {
                        Ok(values) => self.suggestions = Some((word, Some(values))),
                        Err(error) => {
                            self.error = Some(error);
                            self.suggestions = Some((word, Some(vec![])));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    pub fn request_suggestions(&mut self, language: DocumentLanguage, word: &str) {
        if self
            .suggestions
            .as_ref()
            .is_some_and(|(previous, _)| previous == word)
        {
            return;
        }
        self.suggestions = Some((word.into(), None));
        if let Err(error) = self.sender.send(Task::Suggest(
            self.generation.load(Ordering::Relaxed),
            language,
            word.into(),
        )) {
            self.error = Some(CoreError::external(error));
        }
    }
    pub fn next_wake(&self, now: Instant) -> Option<Duration> {
        self.pending
            .as_ref()
            .map(|(due, _)| due.saturating_duration_since(now))
    }
    #[cfg(any(test, feature = "test-support"))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn wait(&mut self) {
        if let Some((_, request)) = self.pending.take() {
            self.sender.send(Task::Check(request)).unwrap();
        }
        let (sender, receiver) = mpsc::channel();
        self.sender.send(Task::Barrier(sender)).unwrap();
        receiver.recv_timeout(Duration::from_secs(30)).unwrap();
        self.poll();
    }
}
impl Drop for Checker {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn background_check_debounces_rejects_old_results_and_follows_language_and_exceptions() {
        let mut checker = Checker::new(|| {});
        let now = Instant::now();
        checker.synchronize(
            1,
            "courrrier",
            DocumentLanguage::French,
            BTreeSet::new(),
            true,
            now,
        );
        assert_eq!(checker.next_wake(now), Some(DELAY));
        assert!(!checker.ready);
        checker.synchronize(
            1,
            "courrrier",
            DocumentLanguage::French,
            BTreeSet::new(),
            true,
            now + DELAY,
        );
        let (sender, receiver) = mpsc::channel();
        checker.sender.send(Task::Barrier(sender)).unwrap();
        receiver.recv_timeout(Duration::from_secs(30)).unwrap();
        // La réponse de la révision 1 attend encore dans le canal.
        checker.synchronize(
            2,
            "Bonjour",
            DocumentLanguage::French,
            BTreeSet::new(),
            true,
            now + DELAY,
        );
        assert!(checker.issues.is_empty() && !checker.ready);
        checker.wait();
        assert!(checker.ready && checker.issues.is_empty());
        checker.synchronize(
            3,
            "helllo hello",
            DocumentLanguage::English,
            BTreeSet::new(),
            true,
            now,
        );
        checker.wait();
        assert_eq!(checker.issues[0].word, "helllo");
        checker.request_suggestions(DocumentLanguage::English, "helllo");
        checker.wait();
        assert!(
            checker
                .suggestions
                .as_ref()
                .unwrap()
                .1
                .as_ref()
                .unwrap()
                .contains(&"hello".into())
        );
        checker.synchronize(
            3,
            "helllo hello",
            DocumentLanguage::English,
            BTreeSet::from(["helllo".into()]),
            true,
            now,
        );
        checker.wait();
        assert!(checker.issues.is_empty());
        checker.synchronize(
            4,
            "courrrier",
            DocumentLanguage::German,
            BTreeSet::new(),
            true,
            now,
        );
        assert!(!checker.ready && checker.next_wake(now).is_none() && checker.issues.is_empty());
        checker.synchronize(
            5,
            "courrrier",
            DocumentLanguage::French,
            BTreeSet::new(),
            false,
            now,
        );
        assert!(checker.next_wake(now).is_none());
        assert_eq!(word_key("L’ÉCOLE"), word_key("l'e\u{301}cole"));
    }

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
    fn analysis_preserves_unicode_offsets_excludes_addresses_and_reuses_only_current_lines() {
        let text = "Été 🌳 courrrier\n/ Bonjour\tbonjouur\n\u{fffc} légende\nrendez‑vous M. Mme etc. aujourd’hui\nhttps://courrrier.example/courrrier nom@courrrier.example 2026 ABC123";
        let mut request = Request {
            ticket: 1,
            text: text.into(),
            language: DocumentLanguage::French,
            accepted: BTreeSet::new(),
        };
        let generation = AtomicU64::new(1);
        let mut analysis = Analysis::default();
        let issues = analysis.check(&request, &generation).unwrap();
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
        request.accepted.insert("courrrier".into());
        assert_eq!(analysis.check(&request, &generation).unwrap().len(), 1);
        request.text = "Bonjour".into();
        assert!(analysis.check(&request, &generation).unwrap().is_empty());
        assert_eq!(analysis.lines.len(), 1);
    }
}
