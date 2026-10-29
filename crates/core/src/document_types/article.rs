//! Scientific article: canonical authors, official free-text affiliations and localized rich frontmatter.
use super::{FieldGroup, FieldKind, FieldValue, MetadataField};
use crate::{
    content::editorial::RichText,
    document::{DocumentLanguage, DocumentOptions, typst_string},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Author {
    pub id: String,
    pub name: String,
    #[serde(rename = "affiliation-ids")]
    pub affiliation_ids: Vec<String>,
    pub email: String,
    pub orcid: String,
    pub corresponding: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Affiliation {
    pub id: String,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Localization {
    pub id: String,
    pub language: DocumentLanguage,
    pub title: RichText,
    #[serde(rename = "short-title")]
    pub short_title: RichText,
    pub subtitle: RichText,
    #[serde(rename = "abstract")]
    pub summary: RichText,
    pub keywords: Vec<String>,
}
impl Localization {
    pub fn blank(language: DocumentLanguage, id: String) -> Self {
        Self {
            id,
            language,
            title: Default::default(),
            short_title: Default::default(),
            subtitle: Default::default(),
            summary: Default::default(),
            keywords: vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub options: DocumentOptions,
    #[serde(default)]
    pub references: Vec<crate::bibliography::ReferenceFile>,
    #[serde(default = "default_citation_style")]
    pub citation_style: String,
    #[serde(default)]
    pub show_outline: bool,
    pub authors: Vec<Author>,
    pub affiliations: Vec<Affiliation>,
    pub localizations: Vec<Localization>,
}
fn default_citation_style() -> String {
    "chicago-author-date".into()
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            options: Default::default(),
            references: vec![],
            citation_style: default_citation_style(),
            show_outline: false,
            authors: vec![],
            affiliations: vec![],
            localizations: vec![Localization::blank(
                DocumentLanguage::French,
                "localization-fr".into(),
            )],
        }
    }
}
impl Metadata {
    pub fn demo() -> Self {
        let mut result = Self::default();
        result.localizations[0].title =
            RichText::plain("Les transformations du travail universitaire");
        result.localizations[0].summary = RichText::plain(
            "Cette étude examine les transformations de l’organisation du travail universitaire.",
        );
        result.localizations[0].keywords =
            vec!["université".into(), "organisation du travail".into()];
        result.authors.push(Author {
            id: "author-1".into(),
            name: "Camille Martin".into(),
            ..Default::default()
        });
        result
    }
    pub fn validate(&self) -> Result<(), String> {
        crate::bibliography::index(&self.references)?;
        if !["chicago-author-date", "apa", "ieee"].contains(&self.citation_style.as_str()) {
            return Err("Style bibliographique invalide.".into());
        }
        fn ids<'a>(values: impl Iterator<Item = &'a str>) -> Result<BTreeSet<&'a str>, String> {
            let mut ids = BTreeSet::new();
            for id in values {
                if id.is_empty()
                    || id.len() > 128
                    || id.chars().any(char::is_control)
                    || !ids.insert(id)
                {
                    return Err("Identifiants d’en-tête invalides ou dupliqués.".into());
                }
            }
            Ok(ids)
        }
        if self.authors.len() > 100
            || self.affiliations.len() > 100
            || self.localizations.len() > DocumentLanguage::ALL.len()
        {
            return Err("En-tête trop volumineux.".into());
        }
        ids(self.authors.iter().map(|a| a.id.as_str()))?;
        let affiliations = ids(self.affiliations.iter().map(|a| a.id.as_str()))?;
        ids(self.localizations.iter().map(|a| a.id.as_str()))?;
        for a in &self.authors {
            let mut used = BTreeSet::new();
            if a.affiliation_ids
                .iter()
                .any(|id| !affiliations.contains(id.as_str()) || !used.insert(id))
            {
                return Err("Une association à une affiliation est absente ou dupliquée.".into());
            }
        }
        let mut languages = BTreeSet::new();
        for entry in &self.localizations {
            if !languages.insert(entry.language.code()) {
                return Err("Un seul ensemble est autorisé par langue.".into());
            }
            if ![&entry.title, &entry.short_title, &entry.subtitle]
                .iter()
                .all(|t| t.is_inline())
            {
                return Err("Un titre accepte seulement du contenu en ligne.".into());
            }
            if entry.keywords.len() > 100
                || entry
                    .keywords
                    .iter()
                    .any(|s| s.len() > 1000 || s.chars().any(char::is_control))
            {
                return Err("Mots-clés invalides ou trop nombreux.".into());
            }
        }
        for value in self
            .authors
            .iter()
            .flat_map(|a| [&a.name, &a.email, &a.orcid])
            .chain(self.affiliations.iter().map(|a| &a.text))
        {
            if value.len() > 16000 || value.chars().any(|c| c.is_control() && c != '\n') {
                return Err("Texte d’en-tête invalide ou trop volumineux.".into());
            }
        }
        Ok(())
    }
    pub fn ready_for_export(&self) -> bool {
        self.validate().is_ok()
            && self
                .localizations
                .iter()
                .any(|l| l.language == self.options.language && !l.title.is_empty())
            && self.authors.iter().all(|a| !a.name.trim().is_empty())
            && self.affiliations.iter().all(|a| !a.text.trim().is_empty())
    }
    pub fn field_values(&self) -> Vec<FieldValue> {
        [
            (
                MetadataField::ArticleReferences,
                "references",
                "field-bibliography",
                serde_json::to_value(&self.references).unwrap(),
            ),
            (
                MetadataField::ArticleCitationStyle,
                "citation_style",
                "field-citation-style",
                serde_json::json!(self.citation_style),
            ),
            (
                MetadataField::ArticleOutline,
                "show_outline",
                "field-show-outline",
                serde_json::json!(self.show_outline),
            ),
            (
                MetadataField::ArticleAuthors,
                "authors",
                "field-article-authors",
                serde_json::to_value(&self.authors).unwrap(),
            ),
            (
                MetadataField::ArticleAffiliations,
                "affiliations",
                "field-article-affiliations",
                serde_json::to_value(&self.affiliations).unwrap(),
            ),
            (
                MetadataField::ArticleLocalizations,
                "localizations",
                "field-article-localizations",
                serde_json::to_value(&self.localizations).unwrap(),
            ),
        ]
        .into_iter()
        .map(|(field, parameter, label_key, value)| FieldValue {
            field,
            parameter,
            label_key,
            value,
            group: "header",
            multiline: false,
            kind: FieldKind::Records,
            required_for_export: false,
            editor_id: parameter,
        })
        .collect()
    }
    pub fn main_source(&self) -> String {
        fn strings(items: &[String]) -> String {
            format!(
                "({})",
                items
                    .iter()
                    .map(|s| format!("{},", typst_string(s)))
                    .collect::<String>()
            )
        }
        let authors = self.authors.iter().map(|a| format!("(id: {}, name: {}, affiliation-ids: {}, email: {}, orcid: {}, corresponding: {}),", typst_string(&a.id), typst_string(&a.name), strings(&a.affiliation_ids), typst_string(&a.email), typst_string(&a.orcid), a.corresponding)).collect::<String>();
        let affiliations = self
            .affiliations
            .iter()
            .map(|a| {
                format!(
                    "(id: {}, text: {}),",
                    typst_string(&a.id),
                    typst_string(&a.text)
                )
            })
            .collect::<String>();
        let localizations = self.localizations.iter().map(|l| format!("(id: {}, language: {}, title: [{}], short-title: [{}], subtitle: [{}], abstract: [{}], keywords: {}),", typst_string(&l.id), typst_string(l.language.code()), l.title.source(true), l.short_title.source(true), l.subtitle.source(true), l.summary.source(false), strings(&l.keywords))).collect::<String>();
        let o = &self.options;
        let references = self
            .references
            .iter()
            .map(|file| format!("path({}),", typst_string(&file.path())))
            .collect::<String>();
        format!(
            "#import \"modele.typ\": article\n#show: article.with(\n  authors: ({authors}),\n  affiliations: ({affiliations}),\n  localizations: ({localizations}),\n  references: ({references}),\n  citation-style: {},\n  show-outline: {},\n  options: (paper: {}, landscape: {}, language: {}, bullet-marker: {}),\n)\n#include \"contenu.typ\"\n",
            typst_string(&self.citation_style),
            self.show_outline,
            typst_string(o.paper.typst_name()),
            o.landscape,
            typst_string(o.language.code()),
            o.bullet_marker
                .as_deref()
                .map_or_else(|| "auto".into(), typst_string)
        )
    }
}
pub const SIGNATURE: &str = "article(localizations, authors, affiliations, references, citation-style, show-outline, options, body)";
pub const GROUPS: &[FieldGroup] = &[
    FieldGroup {
        id: "titles",
        label_key: "group-article-titles",
        sections: &[],
    },
    FieldGroup {
        id: "affiliations",
        label_key: "field-article-affiliations",
        sections: &[],
    },
    FieldGroup {
        id: "authors",
        label_key: "field-article-authors",
        sections: &[],
    },
    FieldGroup {
        id: "abstracts",
        label_key: "group-article-abstracts",
        sections: &[],
    },
    FieldGroup {
        id: "bibliography",
        label_key: "field-bibliography",
        sections: &[],
    },
];

/// Shared collection descriptors consumed by GUI and documented to CLI/MCP.
pub(super) fn schema() -> serde_json::Value {
    use serde_json::json;
    let rich = |key, label, inline| json!({"key":key,"label_key":label,"kind":if inline {"rich_inline"} else {"rich"},"default":RichText::default()});
    json!({"outline":{"parameter":"show_outline","field":"article.show_outline"},"bibliography":{"tab":"bibliography","files":"references","field":"article.references","style":"citation_style","style_field":"article.citation_style"},"panels":[
        {"id":"titles","collection":"localizations","fields":["language","title","subtitle","short-title"]},
        {"id":"affiliations","collection":"affiliations","editing":"dialog"},
        {"id":"authors","collection":"authors","editing":"dialog"},
        {"id":"abstracts","collection":"localizations","fields":["language","abstract","keywords"]}
    ],"collections":[
        {"field":"article.references","parameter":"references","label_key":"field-bibliography","item_label_key":"field-reference-file","title_key":"name","schema":schemars::schema_for!(Vec<crate::bibliography::ReferenceFile>),"fields":[
            {"key":"name","label_key":"field-reference-file","kind":"text","default":""},
            {"key":"contents","label_key":"field-reference-source","kind":"source","default":""}
        ]},
        {"field":"article.authors","parameter":"authors","label_key":"field-article-authors","item_label_key":"article-author","title_key":"name","summary":["email","corresponding"],
         "schema":schemars::schema_for!(Vec<Author>), "fields":[
            {"key":"name","label_key":"field-full-name","kind":"text","default":""},
            {"key":"email","label_key":"field-email","kind":"text","default":""},
            {"key":"orcid","label_key":"field-orcid","kind":"text","default":""},
            {"key":"affiliation-ids","label_key":"field-article-affiliations","kind":"references","references":"affiliations","default":[]},
            {"key":"corresponding","label_key":"field-corresponding","kind":"boolean","default":false}
         ]},
        {"field":"article.affiliations","parameter":"affiliations","label_key":"field-article-affiliations","item_label_key":"article-affiliation","title_key":"text",
         "schema":schemars::schema_for!(Vec<Affiliation>), "fields":[
            {"key":"text","label_key":"field-official-affiliation","kind":"multiline","default":""}
         ]},
        {"field":"article.localizations","parameter":"localizations","label_key":"field-article-localizations","item_label_key":"article-localization","title_key":"language","language_key":"language",
         "schema":schemars::schema_for!(Vec<Localization>), "fields":[
            {"key":"language","label_key":"field-language","kind":"language","default":"fr"},
            rich("title","field-article-title",true), rich("short-title","field-short-title",true),
            rich("subtitle","field-subtitle",true), rich("abstract","field-abstract",false),
            {"key":"keywords","label_key":"field-keywords","kind":"tokens","default":[]}
         ]}
    ]})
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{
        document_types::{DocumentType, Metadata as DocumentMetadata, models::Model},
        rendering::TypstWorld,
    };
    use typst::layout::{Frame, FrameItem};
    fn text(frame: &Frame) -> String {
        frame
            .items()
            .map(|(_, item)| match item {
                FrameItem::Group(g) => text(&g.frame),
                FrameItem::Text(t) => t.text.to_string(),
                _ => String::new(),
            })
            .collect()
    }
    fn fixture() -> Metadata {
        let mut meta = Metadata::demo();
        meta.localizations[0].short_title = RichText::plain("TITRECOURT");
        meta.localizations[0].summary = RichText::new(serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Résumé scientifique","marks":[{"type":"italic"}]}]}]})).unwrap()).unwrap();
        let mut english = Localization::blank(DocumentLanguage::English, "localization-en".into());
        english.title = RichText::plain("Academic work");
        english.summary = RichText::plain("A study of academic work.");
        english.keywords = vec!["academic work".into()];
        meta.localizations.push(english);
        meta.affiliations.push(Affiliation {id:"irmar".into(),text:"Univ Rennes, CNRS, IRMAR (Institut de Recherche Mathématique de Rennes) - UMR 6625, F-35000 Rennes, France".into()});
        meta.authors[0].affiliation_ids.push("irmar".into());
        meta.authors[0].email = "camille@example.org".into();
        meta.authors.push(Author {
            id: "author-2".into(),
            name: "Camille Martin".into(),
            affiliation_ids: vec!["irmar".into()],
            corresponding: true,
            email: "other@example.org".into(),
            ..Default::default()
        });
        meta.authors.push(Author {
            id: "independent".into(),
            name: "Alex Dupont".into(),
            ..Default::default()
        });
        meta
    }
    #[test]
    fn editorial_tabs_share_localized_records_and_cover_every_field() {
        let descriptor = schema();
        let panels = descriptor["panels"].as_array().unwrap();
        assert_eq!(panels.len() + 1, GROUPS.len());
        for group in GROUPS.iter().filter(|group| group.id != "bibliography") {
            assert!(panels.iter().any(|panel| panel["id"] == group.id));
        }
        for collection in descriptor["collections"].as_array().unwrap() {
            if collection["parameter"] == "references" {
                continue;
            }
            for field in collection["fields"].as_array().unwrap() {
                assert!(panels.iter().any(|panel| {
                    panel["collection"] == collection["parameter"]
                        && (panel["fields"].is_null()
                            || panel["fields"].as_array().unwrap().contains(&field["key"]))
                }));
            }
        }
        assert_eq!(panels[0]["collection"], panels[3]["collection"]);
        assert_eq!(
            panels[0]["fields"],
            serde_json::json!(["language", "title", "subtitle", "short-title"])
        );
        assert_eq!(
            panels[3]["fields"],
            serde_json::json!(["language", "abstract", "keywords"])
        );
    }
    #[test]
    fn real_article_preserves_official_affiliations_multilingual_content_and_running_title() {
        let meta = fixture();
        assert!(meta.ready_for_export());
        let template = Model::ArticleWorking.source();
        let dependencies = crate::packages::for_template(&template);
        assert!(dependencies.contains_key("capabilities/authorship.typ"));
        assert!(dependencies.contains_key("capabilities/abstracts.typ"));
        let mut world = TypstWorld::default();
        world.set_dependencies(&dependencies);
        let body =
            "Un paragraphe scientifique pour éprouver les pages et les en-têtes courants.\n\n"
                .repeat(65);
        let result = world
            .compile_with_template(&body, &DocumentMetadata::Article(meta.clone()), &template)
            .unwrap();
        assert!(result.pages().len() > 1);
        let first = text(&result.pages()[0].frame);
        assert!(first.contains(&meta.affiliations[0].text), "{first}");
        assert_eq!(first.matches("UMR 6625").count(), 1);
        for expected in [
            "Résumé",
            "Abstract",
            "université",
            "academic work",
            "Alex Dupont",
            "other@example.org",
        ] {
            assert!(first.contains(expected), "{first}");
        }
        assert!(!first.contains("camille@example.org")); // a mail address alone does not make a correspondent
        assert!(text(&result.pages()[1].frame).contains("TITRECOURT"));
        assert!(!first.contains("TITRECOURT"));
        let source = meta.main_source();
        assert!(source.contains("#emph["));
        let restored: Metadata =
            serde_json::from_str(&serde_json::to_string(&meta).unwrap()).unwrap();
        assert_eq!(restored, meta);
    }
    #[test]
    fn article_archive_and_export_recompile_using_only_their_own_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Article.typsto");
        let template = Model::ArticleWorking.source();
        let archive = crate::content::archive::Archive {
            document: serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Le corps de l’article et sa note."},{"type":"footnote","attrs":{"text":"Note scientifique."}}]}]})).unwrap(),
            metadata: DocumentMetadata::Article(fixture()),
            dependencies: crate::packages::for_template(&template),
            template,
            model_id: Model::ArticleWorking.id().into(),
            assets: Default::default(),
            yjs: vec![],
        };
        let mut lease = crate::DocumentLease::acquire(&path).unwrap();
        archive.save(&mut lease).unwrap();
        let restored = crate::content::archive::Archive::read(&path).unwrap();
        assert_eq!(restored.metadata, archive.metadata);
        assert_eq!(restored.document, archive.document);
        assert_eq!(restored.dependencies, archive.dependencies);
        let body = restored.validate().unwrap();
        let mut world = TypstWorld::default();
        world.set_dependencies(&restored.dependencies);
        let document = world
            .compile_with_template(&body, &restored.metadata, &restored.template)
            .unwrap();
        let export = dir.path().join("Export");
        let request = crate::rendering::RenderRequest {
            source: body,
            metadata: restored.metadata,
            template: restored.template,
            assets: restored.assets,
            dependencies: restored.dependencies,
        };
        crate::exporting::write_document(
            &document,
            &request,
            &export,
            crate::protocol::ExportFormat::Both,
            false,
        )
        .unwrap();
        assert!(std::fs::metadata(export.join("article.pdf")).unwrap().len() > 0);
        let dependencies = std::sync::Arc::new(
            request
                .dependencies
                .keys()
                .map(|name| {
                    (
                        name.clone(),
                        std::fs::read(export.join(name)).unwrap().into(),
                    )
                })
                .collect(),
        );
        let metadata: DocumentMetadata =
            serde_json::from_slice(&std::fs::read(export.join("metadonnees.json")).unwrap())
                .unwrap();
        let mut isolated = TypstWorld::default();
        isolated.set_dependencies(&dependencies);
        let body = std::fs::read_to_string(export.join("contenu.typ")).unwrap();
        let template = std::fs::read_to_string(export.join("modele.typ")).unwrap();
        assert_eq!(
            isolated
                .compile_with_template(&body, &metadata, &template)
                .unwrap()
                .pages()
                .len(),
            document.pages().len()
        );
        let mut missing = (*dependencies).clone();
        missing.remove("capabilities/authorship.typ");
        isolated.set_dependencies(&std::sync::Arc::new(missing));
        assert!(
            isolated
                .compile_with_template(&body, &metadata, &template)
                .is_err()
        );
    }
    #[test]
    fn rich_frontmatter_rejects_objects_unsupported_marks_and_typst_injection_remains_literal() {
        for node in [
            serde_json::json!({"type":"figure","attrs":{"src":"x"}}),
            serde_json::json!({"type":"heading"}),
            serde_json::json!({"type":"paragraph","content":[{"type":"text","text":"x","marks":[{"type":"code"}]}]}),
        ] {
            let value = serde_json::json!({"type":"doc","content":[node]});
            assert!(serde_json::from_value::<RichText>(value).is_err());
        }
        let mut meta = fixture();
        meta.localizations[0].title = RichText::plain("#panic(\"injection\") [brackets]");
        let mut world = TypstWorld::default();
        let result = world
            .compile_with_template(
                "Body",
                &DocumentMetadata::Article(meta),
                &Model::ArticleWorking.source(),
            )
            .unwrap();
        assert!(text(&result.pages()[0].frame).contains("#panic(\"injection\") [brackets]"));
    }
    #[test]
    fn identities_associations_and_languages_are_validated_without_rewriting_the_data() {
        let mut meta = fixture();
        let original = meta.clone();
        meta.authors[0].affiliation_ids.push("missing".into());
        assert!(meta.validate().is_err());
        meta = original.clone();
        meta.authors[1].id = meta.authors[0].id.clone();
        assert!(meta.validate().is_err());
        meta = original.clone();
        meta.localizations[1].language = DocumentLanguage::French;
        assert!(meta.validate().is_err());
        meta = original;
        meta.options.language = DocumentLanguage::German;
        assert_eq!(meta.localizations.len(), 2);
        assert!(!meta.ready_for_export());
        assert!(!meta.localizations[0].title.is_empty());
        assert!(
            DocumentMetadata::blank(DocumentType::Article)
                .validate()
                .is_ok()
        );
    }
}
