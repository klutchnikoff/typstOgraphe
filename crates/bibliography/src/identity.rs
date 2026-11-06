//! Adapted from Citekeep's candidate/refutation/coherence separation.
//! See fixtures/bibliography for source commit, scenarios and MIT attribution.
use crate::{Code, Error, Profile, Record, RecordId, Result, normalize_text};
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Signal {
    Doi,
    Arxiv,
    AuthorYearStem,
    AuthorTitle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictReason {
    ContradictoryIdentifiers,
    ContradictoryTitles,
    ContradictoryAuthors,
    MissingEvidence,
    PublicationState,
    VersionDifference,
    AmbiguousTarget,
    KeyCollision,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Evidence {
    pub signals: Vec<Signal>,
    pub contradictions: Vec<ConflictReason>,
    /// A refutation removes a candidate link, never a notice.
    pub refuted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CandidateGroup {
    pub members: Vec<RecordId>,
    /// Empty means coherent candidates, not permission to mutate a library.
    pub conflicts: Vec<ConflictReason>,
}

pub fn normalize_doi(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    let value = [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
        "doi:",
    ]
    .iter()
    .find_map(|p| value.strip_prefix(p))
    .unwrap_or(&value)
    .trim();
    (value.starts_with("10.") && value.contains('/') && !value.chars().any(char::is_whitespace))
        .then(|| value.into())
}

/// Repository identity, excluding a version suffix. Original values remain untouched.
pub fn normalize_arxiv(value: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)^(?:(?:https?://(?:www\.)?arxiv\.org/(?:abs|pdf)/)|(?:https?://doi\.org/10\.48550/arxiv\.)|(?:10\.48550/arxiv\.)|(?:arxiv:))?(\d{4}\.\d{4,5}|[a-z-]+(?:\.[a-z]{2})?/\d{7})(?:v\d+)?(?:\.pdf)?$").unwrap());
    re.captures(value.trim()).map(|c| c[1].to_ascii_lowercase())
}

fn words(value: &str) -> Vec<String> {
    normalize_text(value)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(Into::into)
        .collect()
}

#[derive(Clone)]
struct Facts {
    doi: Option<String>,
    arxiv: Option<String>,
    arxiv_version: Option<String>,
    title: Vec<String>,
    authors: BTreeSet<String>,
    prints: BTreeMap<Signal, String>,
    profile: Profile,
    edition: Option<String>,
}

impl Facts {
    fn new(record: &Record) -> Self {
        let entry = record.entry();
        let serial = entry.serial_number();
        let doi = serial
            .and_then(|s| s.0.get("doi"))
            .and_then(|v| normalize_doi(v))
            .or_else(|| entry.url().and_then(|u| normalize_doi(u.value.as_str())));
        let arxiv_value = serial
            .and_then(|s| s.0.get("arxiv"))
            .cloned()
            .or_else(|| {
                entry
                    .url()
                    .map(|u| u.value.to_string())
                    .filter(|v| v.to_ascii_lowercase().contains("arxiv"))
            })
            .or_else(|| {
                entry.note().map(|n| n.value.to_string()).and_then(|note| {
                    note.split_whitespace()
                        .find(|v| v.to_ascii_lowercase().starts_with("arxiv:"))
                        .map(Into::into)
                })
            });
        let arxiv = arxiv_value.as_deref().and_then(normalize_arxiv);
        let title = words(
            &entry
                .title()
                .map(|t| t.value.to_string())
                .unwrap_or_default(),
        );
        let people = entry.authors().or_else(|| entry.editors()).unwrap_or(&[]);
        let authors = people
            .iter()
            .map(|p| words(&p.name).join(" "))
            .filter(|v| !v.is_empty())
            .collect();
        let mut prints = BTreeMap::new();
        if let Some(doi) = &doi {
            prints.insert(Signal::Doi, doi.clone());
        }
        if let Some(arxiv) = &arxiv {
            prints.insert(Signal::Arxiv, arxiv.clone());
        }
        if let Some(author) = people
            .first()
            .map(|p| words(&p.name).join(" "))
            .filter(|a| !a.is_empty())
            && !title.is_empty()
        {
            prints.insert(
                Signal::AuthorTitle,
                format!("{author}\0{}", title.join(" ")),
            );
            if let Some(date) = entry.date_any() {
                prints.insert(
                    Signal::AuthorYearStem,
                    format!(
                        "{author}\0{}\0{}",
                        date.year,
                        title.iter().take(2).cloned().collect::<Vec<_>>().join(" ")
                    ),
                );
            }
        }
        let arxiv_version = arxiv_value.as_deref().and_then(|v| {
            let lower = v.to_ascii_lowercase();
            let value = lower.trim_end_matches(".pdf");
            value
                .rsplit_once('v')
                .filter(|(_, suffix)| {
                    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                })
                .map(|(_, suffix)| suffix.to_owned())
        });
        Self {
            doi,
            arxiv,
            arxiv_version,
            title,
            authors,
            prints,
            profile: record.profile(),
            edition: record
                .native_value()
                .get("edition")
                .map(ToString::to_string),
        }
    }
    fn describes(&self, other: &Self) -> bool {
        self.title_agrees(other)
            && !self.authors.is_empty()
            && !other.authors.is_empty()
            && (self.authors.is_subset(&other.authors) || other.authors.is_subset(&self.authors))
    }
    fn title_agrees(&self, other: &Self) -> bool {
        !self.title.is_empty()
            && !other.title.is_empty()
            && (self.title.starts_with(&other.title) || other.title.starts_with(&self.title))
    }
    fn evidence(&self, other: &Self) -> Evidence {
        let mut contradictions = vec![];
        let shared_arxiv = self.arxiv.is_some() && self.arxiv == other.arxiv;
        let shared_doi = self.doi.is_some() && self.doi == other.doi;
        let different_doi = self.doi.is_some() && other.doi.is_some() && self.doi != other.doi;
        let different_arxiv =
            self.arxiv.is_some() && other.arxiv.is_some() && self.arxiv != other.arxiv;
        if different_doi || different_arxiv {
            contradictions.push(ConflictReason::ContradictoryIdentifiers);
        }
        if !self.title.is_empty() && !other.title.is_empty() && !self.title_agrees(other) {
            contradictions.push(ConflictReason::ContradictoryTitles);
        }
        if !self.authors.is_empty()
            && !other.authors.is_empty()
            && !self.authors.is_subset(&other.authors)
            && !other.authors.is_subset(&self.authors)
        {
            contradictions.push(ConflictReason::ContradictoryAuthors);
        }
        if (self.title.is_empty() || other.title.is_empty()) && !shared_arxiv && !shared_doi {
            contradictions.push(ConflictReason::MissingEvidence);
        }
        if (self.profile == Profile::Preprint || other.profile == Profile::Preprint)
            && self.profile != other.profile
        {
            contradictions.push(ConflictReason::PublicationState);
        }
        let different_software = self.profile == Profile::Software
            && other.profile == Profile::Software
            && self.edition.is_some()
            && other.edition.is_some()
            && self.edition != other.edition;
        if different_software || (shared_arxiv && self.arxiv_version != other.arxiv_version) {
            contradictions.push(ConflictReason::VersionDifference);
        }
        let refuted = different_arxiv
            || (different_doi && !shared_arxiv && !self.describes(other))
            || (different_software && !shared_doi);
        let signals = self
            .prints
            .iter()
            .filter_map(|(signal, value)| {
                (other.prints.get(signal) == Some(value)).then_some(*signal)
            })
            .collect();
        Evidence {
            signals,
            contradictions,
            refuted,
        }
    }
}

pub fn evidence(one: &Record, other: &Record) -> Evidence {
    Facts::new(one).evidence(&Facts::new(other))
}

/// Candidate groups are evaluated as a whole, including contradicting endpoints of a chain.
pub fn candidate_groups(records: &[Record]) -> Result<Vec<CandidateGroup>> {
    if records.len() > crate::MAX_RECORDS * 2 {
        return Err(Error::new(
            Code::LimitExceeded,
            "Too many records to compare",
        ));
    }
    let mut ids = BTreeSet::new();
    let facts = records.iter().map(Facts::new).collect::<Vec<_>>();
    let mut buckets: BTreeMap<(Signal, String), Vec<usize>> = BTreeMap::new();
    for (i, record) in records.iter().enumerate() {
        if !ids.insert(record.id()) {
            return Err(Error::new(
                Code::DuplicateIdentity,
                "Candidate identities must be unique",
            ));
        }
        for (signal, value) in &facts[i].prints {
            buckets.entry((*signal, value.clone())).or_default().push(i);
        }
    }
    let mut parents = (0..records.len()).collect::<Vec<_>>();
    fn root(parents: &mut [usize], mut i: usize) -> usize {
        while parents[i] != i {
            parents[i] = parents[parents[i]];
            i = parents[i];
        }
        i
    }
    for members in buckets.values() {
        for (j, &one) in members.iter().enumerate() {
            for &other in &members[j + 1..] {
                if !facts[one].evidence(&facts[other]).refuted {
                    let a = root(&mut parents, one);
                    let b = root(&mut parents, other);
                    parents[a] = b;
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..records.len() {
        groups.entry(root(&mut parents, i)).or_default().push(i);
    }
    let mut result = vec![];
    for indices in groups.values().filter(|v| v.len() > 1) {
        let mut conflicts = BTreeSet::new();
        for (j, &one) in indices.iter().enumerate() {
            for &other in &indices[j + 1..] {
                conflicts.extend(facts[one].evidence(&facts[other]).contradictions);
            }
        }
        let mut members = indices
            .iter()
            .map(|&i| records[i].id().clone())
            .collect::<Vec<_>>();
        members.sort();
        result.push(CandidateGroup {
            members,
            conflicts: conflicts.into_iter().collect(),
        });
    }
    result.sort_by(|a, b| a.members.cmp(&b.members));
    Ok(result)
}
