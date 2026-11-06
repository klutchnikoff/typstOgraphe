//! Document integration of the shared native bibliography. Imports are immutable provenance.
pub mod editions;
use crate::{content::Node, document_types::Metadata};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub use typstographe_bibliography::Record;
pub use typstographe_bibliography::{Code as NoticeErrorCode, Error as NoticeError};
use typstographe_bibliography::{Library, RecordId, SearchQuery, summarize};
pub const RETAINED_PATH: &str = "bibliography/retained.json";
pub const ACTIVE_PATH: &str = "bibliography/references.yaml";
pub fn valid_path(path: &str) -> bool {
    path == ACTIVE_PATH
        || path == RETAINED_PATH
        || path
            .strip_prefix("bibliography/sources/")
            .and_then(|s| s.strip_suffix(".bib").or_else(|| s.strip_suffix(".yaml")))
            .is_some_and(|id| RecordId::new(id).is_ok())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Entry {
    pub record_id: RecordId,
    pub key: String,
    pub title: String,
    pub authors: String,
    pub year: String,
    pub label: String,
}
pub fn index(records: &[Record]) -> Result<Vec<Entry>, String> {
    Library::new(records.to_vec()).map_err(|e| e.to_string())?;
    Ok(records
        .iter()
        .map(|r| {
            let summary = summarize(r.entry());
            Entry {
                record_id: r.id().clone(),
                key: r.key().into(),
                title: summary.title,
                authors: summary.authors,
                year: summary
                    .year
                    .map(|y| y.to_string())
                    .unwrap_or_else(|| "s. d.".into()),
                label: format!(
                    "{}, {}",
                    summary.names,
                    summary
                        .year
                        .map(|y| y.to_string())
                        .unwrap_or_else(|| "s. d.".into())
                ),
            }
        })
        .collect())
}
pub fn validate_sources(meta: &crate::document_types::article::Metadata) -> Result<(), String> {
    Library::new(meta.references.clone()).map_err(|e| e.to_string())?;
    let sources: BTreeMap<_, _> = meta.reference_sources.iter().map(|s| (s.id(), s)).collect();
    if sources.len() != meta.reference_sources.len()
        || sources.len() > 1000
        || meta
            .reference_sources
            .iter()
            .map(|s| s.contents().len())
            .sum::<usize>()
            > 8 * 1024 * 1024
    {
        return Err("Invalid bibliography source collection".into());
    }
    for record in &meta.references {
        for origin in record.provenance() {
            if !sources
                .get(&origin.source_id)
                .is_some_and(|s| s.digest() == &origin.source_digest)
            {
                return Err("Bibliographic provenance source missing or inconsistent".into());
            }
        }
    }
    Ok(())
}
pub fn resources(metadata: &Metadata) -> BTreeMap<String, Arc<[u8]>> {
    let Metadata::Article(meta) = metadata else {
        return BTreeMap::new();
    };
    let mut resources = BTreeMap::new();
    if !meta.references.is_empty() {
        let yaml = Library::new(meta.references.clone())
            .and_then(|l| l.to_hayagriva())
            .expect("validated bibliography");
        resources.insert(ACTIVE_PATH.into(), Arc::from(yaml.as_bytes()));
    }
    let retained: Vec<_> = meta.references.iter().filter(|r| !r.retained_parents().is_empty()).map(|r| {
        let value=serde_json::to_value(r).expect("validated record");
        serde_json::json!({"id":r.id(),"key":r.key(),"retained_parents":value["retained_parents"]})
    }).collect();
    if !retained.is_empty() {
        resources.insert(
            RETAINED_PATH.into(),
            Arc::from(serde_json::to_vec_pretty(&retained).expect("retained native bibliography")),
        );
    }
    for source in &meta.reference_sources {
        let ext = match source.format() {
            typstographe_bibliography::ImportFormat::Biblatex => "bib",
            _ => "yaml",
        };
        resources.insert(
            format!(
                "bibliography/sources/{}.{}",
                String::from(source.id().clone()),
                ext
            ),
            Arc::from(source.contents().as_bytes()),
        );
    }
    resources
}
pub fn search(entries: &[Entry], query: &str) -> Vec<Entry> {
    let query = SearchQuery::new(query);
    let mut scored: Vec<_> = entries
        .iter()
        .filter_map(|entry| {
            query
                .score(&entry.title, &entry.authors)
                .map(|score| (score, entry))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| a.1.title.cmp(&b.1.title))
            .then_with(|| a.1.key.cmp(&b.1.key))
    });
    scored
        .into_iter()
        .take(80)
        .map(|(_, v)| v.clone())
        .collect()
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CitationItem {
    /// Stable project notice identity; keys can change without breaking citations.
    pub record_id: RecordId,
    /// Optional literal locator, for example "p. 23–25" or "chap. 2"; never Typst code.
    #[serde(default)]
    pub supplement: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    /// One or more project references in their desired order. No new notice is created.
    pub items: Vec<CitationItem>,
    /// "normal" lets the model style format the citation; "prose" integrates the author in the sentence.
    #[serde(default = "normal")]
    pub form: String,
}
fn normal() -> String {
    "normal".into()
}
impl Citation {
    pub fn validate(&self) -> Result<(), String> {
        if self.items.is_empty()
            || self.items.len() > 32
            || !["normal", "prose"].contains(&self.form.as_str())
        {
            return Err("Citation invalide.".into());
        }
        let mut unique = BTreeSet::new();
        for item in &self.items {
            if item.supplement.len() > 1000
                || item.supplement.chars().any(char::is_control)
                || !unique.insert(&item.record_id)
            {
                return Err("Référence de citation invalide ou dupliquée.".into());
            }
        }
        Ok(())
    }
}
pub fn validate_document(
    document: &Node,
    metadata: &crate::document_types::Metadata,
) -> Result<(), String> {
    let files = match metadata {
        crate::document_types::Metadata::Article(meta) => meta.references.as_slice(),
        _ => &[],
    };
    let entries = index(files)?;
    fn visit(node: &Node, entries: &[Entry]) -> Result<(), String> {
        if node.kind == "citation" {
            let citation: Citation = serde_json::from_value(node.attrs.clone().unwrap_or_default())
                .map_err(|e| e.to_string())?;
            citation.validate()?;
            for item in &citation.items {
                if !entries.iter().any(|e| e.record_id == item.record_id) {
                    return Err(format!(
                        "bibliography_record_missing:{}",
                        String::from(item.record_id.clone())
                    ));
                }
            }
        }
        for child in &node.content {
            visit(child, entries)?;
        }
        Ok(())
    }
    visit(document, &entries)
}
#[cfg(test)]
pub(crate) fn test_record(id: &str, name: &str, contents: &str) -> Record {
    let format = if name.ends_with(".bib") {
        typstographe_bibliography::ImportFormat::Biblatex
    } else {
        typstographe_bibliography::ImportFormat::Hayagriva
    };
    let library = typstographe_bibliography::parse_native(contents, format).unwrap();
    Record::new(
        RecordId::new(id).unwrap(),
        library.iter().next().unwrap().clone(),
    )
    .unwrap()
}
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{
        DocumentLease,
        content::archive::Archive,
        document_types::{Metadata, article, models::Model},
        rendering::{RenderRequest, TypstWorld},
    };
    fn files() -> Vec<Record> {
        vec![
            crate::bibliography::test_record(
                "books",
                "books.bib",
                "@book{bourdieu1980, author={Bourdieu, Pierre}, title={Le sens pratique}, year={1980}, publisher={Minuit}}",
            ),
            crate::bibliography::test_record(
                "papers",
                "papers.yaml",
                "durand2020:\n  type: Article\n  title: Une enquête collective\n  author: Durand, Camille\n  date: 2020\n  parent:\n    type: Periodical\n    title: Revue exemple\n",
            ),
        ]
    }
    fn doc() -> Node {
        serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"heading","attrs":{"level":1},"content":[{"type":"text","text":"Introduction"}]},{"type":"paragraph","content":[{"type":"text","text":"Une étude "},{"type":"citation","attrs":{"items":[{"record_id":"books","supplement":"p. 23"},{"record_id":"papers","supplement":""}],"form":"normal"}}]}]})).unwrap()
    }
    #[test]
    fn native_index_fuzzy_authors_titles_and_collisions_are_validated() {
        let files = files();
        let entries = index(&files).unwrap();
        assert_eq!(search(&entries, "sens bordieu")[0].key, "bourdieu1980");
        assert_eq!(search(&entries, "enquete DURAND")[0].key, "durand2020");
        assert!(search(&entries, "introuvable").is_empty());
        let mut collision = files.clone();
        collision.push(
            Record::from_native(
                RecordId::new("copy").unwrap(),
                files[0].key(),
                files[0].native_value(),
            )
            .unwrap(),
        );
        assert!(index(&collision).is_err());
        assert!(valid_path(ACTIVE_PATH));
        assert!(valid_path("bibliography/sources/books.bib"));
        assert!(!valid_path("bibliography/../escape.bib"));
    }
    #[test]
    fn citations_archive_export_outline_and_each_supported_style_compile_offline() {
        let root = tempfile::tempdir().unwrap();
        let template = Model::ArticleWorking.source();
        let mut meta = article::Metadata::demo();
        meta.references = files();
        meta.show_outline = Some(true);
        let node = doc();
        let mut world = TypstWorld::default();
        let dependencies = crate::packages::for_template(&template);
        world.set_dependencies(&dependencies);
        let mut pages = 0;
        for style in ["chicago-author-date", "apa", "ieee"] {
            meta.citation_style = Some(style.into());
            let metadata = Metadata::Article(meta.clone());
            validate_document(&node, &metadata).unwrap();
            let (body, map) = crate::content::source_mapped_with_metadata(
                &node,
                &Default::default(),
                &Metadata::Article(meta.clone()),
            )
            .unwrap();
            assert!(
                map.characters
                    .iter()
                    .all(|range| range.start <= range.end && range.end <= body.len())
            );
            assert!(!body.contains("typstographe-end"));
            let compiled = world
                .compile_with_template(&body, &metadata, &template)
                .unwrap();
            pages = compiled.pages().len();
            let export = root.path().join(style);
            crate::exporting::write_document(
                &compiled,
                &RenderRequest {
                    source: body,
                    metadata,
                    template: template.clone(),
                    model_manifest: crate::document_types::DocumentType::Article
                        .default_model_manifest()
                        .into(),
                    assets: Default::default(),
                    dependencies: dependencies.clone(),
                },
                &export,
                crate::protocol::ExportFormat::Both,
                false,
            )
            .unwrap();
            assert_eq!(
                std::fs::read_to_string(export.join(ACTIVE_PATH)).unwrap(),
                Library::new(meta.references.clone())
                    .unwrap()
                    .to_hayagriva()
                    .unwrap()
            );
        }
        assert!(pages > 0);
        let archive = Archive {
            document: node.clone(),
            metadata: Metadata::Article(meta),
            model_manifest: crate::document_types::DocumentType::Article
                .default_model_manifest()
                .into(),
            template,
            dependencies,
            assets: Default::default(),
            model_id: Model::ArticleWorking.id().into(),
            yjs: vec![],
        };
        let path = root.path().join("article.typsto");
        let mut lease = DocumentLease::acquire(&path).unwrap();
        crate::content::project::Project::new(archive.clone())
            .unwrap()
            .save(&mut lease)
            .unwrap();
        let restored = crate::content::project::Project::read(&path)
            .unwrap()
            .current()
            .clone();
        assert_eq!(restored.metadata, archive.metadata);
        let mut broken = restored.clone();
        let Metadata::Article(m) = &mut broken.metadata else {
            unreachable!()
        };
        m.references.remove(0);
        assert!(broken.validate().unwrap_err().to_string().contains("books"));
        assert_eq!(
            crate::content::source_with_metadata(
                &restored.document,
                &restored.assets,
                &restored.metadata
            )
            .unwrap(),
            restored.validate().unwrap()
        );
    }

    #[test]
    fn citation_badge_has_real_preview_geometry() {
        let node: Node = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"citation","attrs":{"items":[{"record_id":"books"}],"form":"normal"}}]}]})).unwrap();
        let mut meta = article::Metadata::demo();
        meta.references = files();
        let template = Model::ArticleWorking.source();
        let mut world = TypstWorld::default();
        world.set_dependencies(&crate::packages::for_template(&template));
        let (body, map) = crate::content::source_mapped_with_metadata(
            &node,
            &Default::default(),
            &Metadata::Article(meta.clone()),
        )
        .unwrap();
        let compiled = world
            .compile_with_template(&body, &Metadata::Article(meta), &template)
            .unwrap();
        let sync = crate::navigation::RenderSync::new(1, world, compiled, map);
        let point = sync.forward(1).expect("citation position");
        // At the first object, the paragraph boundary and badge share an anchor.
        assert!(matches!(sync.backward(point), Some(0 | 1)));
        assert!(!sync.selection_highlights_exact(1..2).is_empty());
    }
}
