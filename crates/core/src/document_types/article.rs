//! Scientific article: canonical authors, official free-text affiliations and localized rich frontmatter.
use super::authorship::{Affiliation, Author};
use crate::{
    content::editorial::RichText,
    document::{DocumentLanguage, DocumentOptions},
};
use serde::{Deserialize, Serialize};

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
        let mut values = super::DocumentType::Article
            .definition()
            .record_defaults("localizations")
            .expect("localized structure");
        values.insert("id".into(), serde_json::json!(id));
        values.insert("language".into(), serde_json::json!(language));
        serde_json::from_value(serde_json::Value::Object(values))
            .expect("declared localization defaults")
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub options: DocumentOptions,
    #[serde(default)]
    pub references: Vec<typstographe_bibliography::Record>,
    #[serde(default)]
    pub reference_sources: Vec<typstographe_bibliography::ImportSource>,
    #[serde(default = "default_citation_style")]
    pub citation_style: Option<String>,
    #[serde(default)]
    pub show_outline: Option<bool>,
    pub authors: Vec<Author>,
    pub affiliations: Vec<Affiliation>,
    pub localizations: Vec<Localization>,
}
fn default_citation_style() -> Option<String> {
    None
}
impl Default for Metadata {
    fn default() -> Self {
        serde_json::from_value(super::runtime::initial_fields(
            super::DocumentType::Article.definition(),
            &DocumentOptions::default(),
        ))
        .expect("declared article defaults")
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
        super::Metadata::Article(self.clone()).validate()
    }
    pub fn ready_for_export(&self) -> bool {
        super::Metadata::Article(self.clone()).ready_for_export()
    }
    pub fn main_source(&self) -> String {
        super::Metadata::Article(self.clone()).main_source()
    }
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
        let descriptor = DocumentType::Article.structured_metadata().unwrap();
        let panels = descriptor["panels"].as_array().unwrap();
        assert_eq!(
            panels.len() + 1,
            DocumentMetadata::blank(DocumentType::Article)
                .editor_groups()
                .len()
        );
        for group in DocumentMetadata::blank(DocumentType::Article)
            .editor_groups()
            .iter()
            .filter(|group| group.id != "bibliography")
        {
            assert!(panels.iter().any(|panel| panel["id"] == group.id));
        }
        for collection in descriptor["collections"].as_array().unwrap() {
            if ["references", "reference_sources"]
                .contains(&collection["parameter"].as_str().unwrap())
            {
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
        let titles = panels.iter().find(|panel| panel["id"] == "titles").unwrap();
        let abstracts = panels
            .iter()
            .find(|panel| panel["id"] == "abstracts")
            .unwrap();
        assert_eq!(titles["collection"], abstracts["collection"]);
        assert_eq!(titles["editing"], "dialog");
        assert_eq!(titles["summary"], serde_json::json!(["title"]));
        assert_eq!(abstracts["editing"], "dialog");
        assert_eq!(abstracts["item_label_key"], "field-abstract");
        assert_eq!(
            titles["fields"],
            serde_json::json!(["language", "title", "subtitle", "short-title"])
        );
        assert_eq!(
            abstracts["fields"],
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
            model_manifest: crate::document_types::DocumentType::Article.default_model_manifest().into(),
            template,
            model_id: Model::ArticleWorking.id().into(),
            assets: Default::default(),
            yjs: vec![],
        };
        let mut lease = crate::DocumentLease::acquire(&path).unwrap();
        crate::content::project::Project::new(archive.clone())
            .unwrap()
            .save(&mut lease)
            .unwrap();
        let restored = crate::content::project::Project::read(&path)
            .unwrap()
            .current()
            .clone();
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
            model_manifest: restored.model_manifest,
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
        let output = text(&result.pages()[0].frame).replace(['\u{a0}', '\u{202f}'], "");
        assert!(
            output.contains("#panic(«injection») [brackets]"),
            "{output}"
        );
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
