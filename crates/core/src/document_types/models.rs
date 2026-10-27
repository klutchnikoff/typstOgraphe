//! Catalogue des modèles fournis. Toutes les listes et commandes en dérivent.
use super::DocumentType;
use crate::document::PaperFormat;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Personal,
    ArticleWorking,
}
pub struct Definition {
    pub model: Model,
    pub kind: DocumentType,
    pub source: &'static str,
    pub page: Option<(PaperFormat, bool)>,
    pub name: &'static str,
    pub description: &'static str,
    pub keywords: &'static [&'static str],
}
pub const ALL: &[Definition] = &[
    Definition {
        model: Model::Personal,
        kind: DocumentType::Courrier,
        source: include_str!("../../templates/courrier-personnel.typ"),
        page: None,
        name: "Personal letter",
        description: "A restrained personal letter with selectable paper size and orientation; A4 portrait retains the DL window-envelope layout.",
        keywords: &[
            "personal",
            "French",
            "A4",
            "A5",
            "US Letter",
            "US Legal",
            "landscape",
            "DL",
            "window envelope",
            "minimal",
        ],
    },
    Definition {
        model: Model::ArticleWorking,
        kind: DocumentType::Article,
        source: include_str!("../../templates/article-working.typ"),
        page: None,
        name: "Working article",
        description: "A readable single-column scientific working article with multilingual frontmatter and shared official affiliations.",
        keywords: &[
            "article",
            "research",
            "working paper",
            "SHS",
            "multilingual",
        ],
    },
];
impl Model {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Personal => "courrier.model.personal",
            Self::ArticleWorking => "article.model.working",
        }
    }
    #[cfg(test)]
    pub fn all() -> impl Iterator<Item = Self> {
        ALL.iter().map(|entry| entry.model)
    }
    pub fn definition(self) -> &'static Definition {
        ALL.iter()
            .find(|entry| entry.model == self)
            .expect("modèle enregistré")
    }
    pub fn source(self) -> String {
        self.definition().source.into()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn type_availability_defaults_and_public_commands_all_follow_the_catalog() {
        let mut ids = std::collections::HashSet::new();
        for kind in DocumentType::ALL {
            let entries: Vec<_> = kind.models().collect();
            assert_eq!(kind.can_create(), !entries.is_empty());
            assert_eq!(
                kind.default_model(),
                entries.first().map_or("", |entry| entry.source)
            );
            for entry in entries {
                assert!(ids.insert(entry.model.id()));
                assert_eq!(entry.model.definition().kind, kind);
            }
        }
        assert!(DocumentType::Courrier.can_create());
        assert!(!DocumentType::CompteRendu.can_create());
        assert!(DocumentType::Article.can_create());
        assert_eq!(ALL.len(), 2);
    }
}
