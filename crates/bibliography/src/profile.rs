use hayagriva::{Entry, types::EntryType};
use serde::{Deserialize, Serialize};

/// Editing views derived exclusively from the active native entry and its parents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Article,
    JournalArticle,
    Book,
    BookChapter,
    ConferencePaper,
    Report,
    Thesis,
    WebResource,
    Preprint,
    Software,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    FormattedText,
    People,
    Publisher,
    Date,
    NumberOrText,
    Url,
    Identifiers,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct FieldDescriptor {
    /// Canonical native JSON pointer; parents are always arrays in this API.
    pub path: String,
    pub kind: FieldKind,
    pub group: &'static str,
    pub recommended: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct ProfileDescriptor {
    pub profile: Profile,
    pub fields: Vec<FieldDescriptor>,
}

impl Profile {
    /// Classification follows native CSL-relevant structures; it is never persisted separately.
    pub fn infer(entry: &Entry) -> Self {
        use EntryType::*;
        let parent = |kind| entry.parents().iter().any(|p| p.entry_type() == &kind);
        match entry.entry_type() {
            Article if parent(Periodical) => Self::JournalArticle,
            Article if parent(Proceedings) => Self::ConferencePaper,
            Article if parent(Repository) => Self::Preprint,
            Article if parent(Blog) => Self::WebResource,
            Article if parent(Report) => Self::Report,
            Article if parent(Thesis) => Self::Thesis,
            Article => Self::Article,
            Chapter | Anthos => Self::BookChapter,
            Book => Self::Book,
            Report => Self::Report,
            Thesis => Self::Thesis,
            Web => Self::WebResource,
            Manuscript => Self::Preprint,
            Repository => Self::Software,
            _ => Self::Other,
        }
    }
    pub(crate) fn parent_types(self) -> &'static [EntryType] {
        use EntryType::*;
        match self {
            Self::JournalArticle => &[Periodical],
            Self::ConferencePaper => &[Proceedings],
            Self::BookChapter => &[Book, Anthology],
            Self::Preprint => &[Repository],
            Self::WebResource => &[Blog],
            _ => &[],
        }
    }
    /// Starting structure for a new record. Imported entries are never coerced into it.
    pub fn empty_entry(self, key: &str) -> Entry {
        use EntryType::*;
        let (kind, parent) = match self {
            Self::Article => (Article, None),
            Self::JournalArticle => (Article, Some(Periodical)),
            Self::Book => (Book, None),
            Self::BookChapter => (Chapter, Some(Book)),
            Self::ConferencePaper => (Article, Some(Proceedings)),
            Self::Report => (Report, None),
            Self::Thesis => (Thesis, None),
            Self::WebResource => (Web, None),
            Self::Preprint => (Manuscript, None),
            Self::Software => (Repository, None),
            Self::Other => (Misc, None),
        };
        let mut entry = hayagriva::Entry::new(key, kind);
        if let Some(parent) = parent {
            entry.set_parents(vec![hayagriva::Entry::new(key, parent)]);
        }
        entry
    }

    pub fn descriptor(self) -> ProfileDescriptor {
        use FieldKind::*;
        let mut fields = vec![];
        let mut add = |path: &str, kind, group, recommended| {
            fields.push(FieldDescriptor {
                path: path.into(),
                kind,
                group,
                recommended,
            })
        };
        add("/title", FormattedText, "work", true);
        add("/author", People, "work", true);
        add("/date", Date, "work", true);
        match self {
            Self::JournalArticle | Self::BookChapter | Self::ConferencePaper => {
                add("/parent/0/title", FormattedText, "publication", true);
                add("/page-range", NumberOrText, "publication", false);
                if self == Self::JournalArticle {
                    add("/volume", NumberOrText, "publication", false);
                    add("/issue", NumberOrText, "publication", false);
                } else {
                    add("/parent/0/editor", People, "publication", false);
                }
            }
            Self::Book => {
                add("/publisher", Publisher, "publication", true);
                add("/edition", NumberOrText, "publication", false);
                add("/editor", People, "publication", false);
            }
            Self::Report | Self::Thesis => {
                add("/organization", FormattedText, "publication", true);
                add("/genre", FormattedText, "publication", false);
            }
            Self::Software => add("/edition", NumberOrText, "publication", true),
            _ => {}
        }
        add("/serial-number", Identifiers, "identifiers", false);
        add(
            "/url",
            Url,
            "identifiers",
            matches!(self, Self::WebResource | Self::Software),
        );
        ProfileDescriptor {
            profile: self,
            fields,
        }
    }
}

pub fn profiles() -> Vec<ProfileDescriptor> {
    use Profile::*;
    [
        Article,
        JournalArticle,
        Book,
        BookChapter,
        ConferencePaper,
        Report,
        Thesis,
        WebResource,
        Preprint,
        Software,
        Other,
    ]
    .into_iter()
    .map(Profile::descriptor)
    .collect()
}
