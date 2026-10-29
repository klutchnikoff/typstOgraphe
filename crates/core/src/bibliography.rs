//! Original bibliography files are authoritative. Parsed entries and fuzzy results are caches.
use crate::content::Node;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex, OnceLock},
};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};
const MAX_FILES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceFile {
    pub id: String,
    pub name: String,
    pub contents: String,
}
impl ReferenceFile {
    pub fn path(&self) -> String {
        format!(
            "bibliography/{}.{}",
            self.id,
            if self.name.to_ascii_lowercase().ends_with(".bib") {
                "bib"
            } else {
                "yaml"
            }
        )
    }
}
pub fn valid_path(path: &str) -> bool {
    path.strip_prefix("bibliography/")
        .and_then(|s| s.strip_suffix(".bib").or_else(|| s.strip_suffix(".yaml")))
        .is_some_and(valid_id)
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub file_id: String,
    pub file_name: String,
    pub key: String,
    pub title: String,
    pub authors: String,
    pub year: String,
    pub label: String,
}
type Cache = VecDeque<([u8; 32], Arc<Vec<Entry>>)>;
fn parse(file: &ReferenceFile) -> Result<Arc<Vec<Entry>>, String> {
    if !valid_id(&file.id)
        || file.name.len() > 256
        || file.name.contains(['/', '\\'])
        || file.name.chars().any(char::is_control)
        || ![".bib", ".yaml", ".yml"]
            .iter()
            .any(|ext| file.name.to_ascii_lowercase().ends_with(ext))
        || file.contents.len() > 2 * 1024 * 1024
    {
        return Err(format!("Fichier bibliographique invalide : {}", file.name));
    }
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let digest: [u8; 32] = Sha256::digest(serde_json::to_vec(file).unwrap()).into();
    let cache = CACHE.get_or_init(Default::default);
    if let Some((_, result)) = cache.lock().unwrap().iter().find(|(key, _)| *key == digest) {
        return Ok(result.clone());
    }
    let library = if file.name.to_ascii_lowercase().ends_with(".bib") {
        hayagriva::io::from_biblatex_str(&file.contents)
            .map_err(|e| format!("{} : {e:?}", file.name))?
    } else {
        hayagriva::io::from_yaml_str(&file.contents).map_err(|e| format!("{} : {e}", file.name))?
    };
    if library.len() > 10000 {
        return Err("Bibliothèque trop volumineuse.".into());
    }
    let result: Arc<Vec<Entry>> = Arc::new(
        library
            .iter()
            .map(|entry| {
                let title = entry
                    .title()
                    .map(|v| v.value.to_string())
                    .unwrap_or_default();
                let people = entry.authors().or_else(|| entry.editors()).unwrap_or(&[]);
                let authors = people
                    .iter()
                    .map(|a| a.name_first(false, false))
                    .collect::<Vec<_>>()
                    .join(", ");
                let names = match people {
                    [] => {
                        if title.is_empty() {
                            entry.key().into()
                        } else {
                            title.clone()
                        }
                    }
                    [a] => a.name.clone(),
                    [a, b] => format!("{} & {}", a.name, b.name),
                    [a, ..] => format!("{} et al.", a.name),
                };
                let year = entry
                    .date_any()
                    .map(|d| d.year.to_string())
                    .unwrap_or_else(|| "s. d.".into());
                Entry {
                    file_id: file.id.clone(),
                    file_name: file.name.clone(),
                    key: entry.key().into(),
                    title,
                    authors,
                    label: format!("{names}, {year}"),
                    year,
                }
            })
            .collect(),
    );
    let mut guard = cache.lock().unwrap();
    // Cover a whole project: a smaller cache would reparse every file on each
    // validation when iterating a library at the supported file limit.
    if guard.len() >= MAX_FILES {
        guard.pop_front();
    }
    guard.push_back((digest, result.clone()));
    Ok(result)
}
pub fn index(files: &[ReferenceFile]) -> Result<Vec<Entry>, String> {
    if files.len() > MAX_FILES
        || files.iter().map(|f| f.contents.len()).sum::<usize>() > 8 * 1024 * 1024
    {
        return Err("Bibliothèque trop volumineuse.".into());
    }
    let mut keys = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut entries = vec![];
    for file in files {
        if !ids.insert(&file.id) {
            return Err("Identité de fichier bibliographique dupliquée.".into());
        }
        for entry in parse(file)?.iter() {
            if let Some(other) = keys.insert(entry.key.clone(), file.name.clone()) {
                return Err(format!(
                    "Clé bibliographique « {} » dupliquée dans « {} » et « {} ».",
                    entry.key, other, file.name
                ));
            }
            entries.push(entry.clone());
        }
    }
    Ok(entries)
}
fn normalized(value: &str) -> String {
    value
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}
pub fn search(entries: &[Entry], query: &str) -> Vec<Entry> {
    if query.len() > 512 {
        return vec![];
    }
    let query = normalized(query);
    let tokens: Vec<_> = query.split_whitespace().collect();
    let mut scored: Vec<_> = entries
        .iter()
        .filter_map(|entry| {
            let haystack = normalized(&format!("{} {}", entry.title, entry.authors));
            let mut score = 0.0;
            for token in &tokens {
                let best = if haystack.contains(token) {
                    1.0
                } else {
                    haystack
                        .split(|c: char| !c.is_alphanumeric())
                        .map(|s| strsim::jaro_winkler(token, s))
                        .fold(0.0, f64::max)
                };
                if best < 0.84 {
                    return None;
                }
                score += best;
            }
            Some((score, entry))
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
    /// Stable project file identity returned by bibliography.search, not a disk path.
    pub file_id: String,
    /// Exact citation key returned by bibliography.search; keys are case-sensitive.
    pub key: String,
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
            if !valid_id(&item.file_id)
                || item.key.is_empty()
                || item.key.len() > 1024
                || item.key.chars().any(char::is_control)
                || item.supplement.len() > 1000
                || item.supplement.chars().any(char::is_control)
                || !unique.insert((&item.file_id, &item.key))
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
                if !entries
                    .iter()
                    .any(|e| e.file_id == item.file_id && e.key == item.key)
                {
                    return Err(format!("Référence citée absente : {}", item.key));
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
pub fn resources(metadata: &crate::document_types::Metadata) -> BTreeMap<String, Arc<[u8]>> {
    match metadata {
        crate::document_types::Metadata::Article(meta) => meta
            .references
            .iter()
            .map(|file| (file.path(), Arc::from(file.contents.as_bytes())))
            .collect(),
        _ => BTreeMap::new(),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{
        DocumentLease,
        content::{archive::Archive, source, source_mapped},
        document_types::{Metadata, article, models::Model},
        rendering::{RenderRequest, TypstWorld},
    };
    fn files() -> Vec<ReferenceFile> {
        vec![
        ReferenceFile { id:"books".into(),name:"books.bib".into(),contents:"@book{bourdieu1980, author={Bourdieu, Pierre}, title={Le sens pratique}, year={1980}, publisher={Minuit}}".into() },
        ReferenceFile { id:"papers".into(),name:"papers.yaml".into(),contents:"durand2020:\n  type: Article\n  title: Une enquête collective\n  author: Durand, Camille\n  date: 2020\n  parent:\n    type: Periodical\n    title: Revue exemple\n".into() },
    ]
    }
    fn doc() -> Node {
        serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"heading","attrs":{"level":1},"content":[{"type":"text","text":"Introduction"}]},{"type":"paragraph","content":[{"type":"text","text":"Une étude "},{"type":"citation","attrs":{"items":[{"file_id":"books","key":"bourdieu1980","supplement":"p. 23"},{"file_id":"papers","key":"durand2020","supplement":""}],"form":"normal"}}]}]})).unwrap()
    }
    #[test]
    fn original_files_index_fuzzy_authors_titles_and_collisions_are_validated() {
        let files = files();
        let entries = index(&files).unwrap();
        assert_eq!(search(&entries, "sens bordieu")[0].key, "bourdieu1980");
        assert_eq!(search(&entries, "enquete DURAND")[0].key, "durand2020");
        assert!(search(&entries, "introuvable").is_empty());
        let mut collision = files.clone();
        collision.push(ReferenceFile {
            id: "copy".into(),
            ..files[0].clone()
        });
        assert!(index(&collision).unwrap_err().contains("bourdieu1980"));
        collision[2].id = "../../escape".into();
        assert!(index(&collision).is_err());
        assert!(valid_path("bibliography/books.bib"));
        assert!(!valid_path("bibliography/../escape.bib"));
        for file in &files {
            let mut duplicate = file.clone();
            duplicate.contents.push('\n');
            duplicate.contents.push_str(&file.contents);
            assert!(
                index(&[duplicate]).is_err(),
                "duplicate within {}",
                file.name
            );
            let mut uppercase = file.clone();
            uppercase.name = uppercase.name.to_ascii_uppercase();
            assert_eq!(index(&[uppercase]).unwrap().len(), 1);
        }
        let mut invalid = files[0].clone();
        invalid.contents = "@book{broken".into();
        assert!(index(&[invalid]).is_err());
    }
    #[test]
    fn citations_archive_export_outline_and_each_supported_style_compile_offline() {
        let root = tempfile::tempdir().unwrap();
        let template = Model::ArticleWorking.source();
        let mut meta = article::Metadata::demo();
        meta.references = files();
        meta.show_outline = true;
        let node = doc();
        let mut world = TypstWorld::default();
        let dependencies = crate::packages::for_template(&template);
        world.set_dependencies(&dependencies);
        let mut pages = 0;
        for style in ["chicago-author-date", "apa", "ieee"] {
            meta.citation_style = style.into();
            let metadata = Metadata::Article(meta.clone());
            validate_document(&node, &metadata).unwrap();
            let (body, map) = source_mapped(&node, &Default::default()).unwrap();
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
                    assets: Default::default(),
                    dependencies: dependencies.clone(),
                },
                &export,
                crate::protocol::ExportFormat::Both,
                false,
            )
            .unwrap();
            assert_eq!(
                std::fs::read_to_string(export.join("bibliography/books.bib")).unwrap(),
                meta.references[0].contents
            );
        }
        assert!(pages > 0);
        let archive = Archive {
            document: node.clone(),
            metadata: Metadata::Article(meta),
            template,
            dependencies,
            assets: Default::default(),
            model_id: Model::ArticleWorking.id().into(),
            yjs: vec![],
        };
        let path = root.path().join("article.typsto");
        let mut lease = DocumentLease::acquire(&path).unwrap();
        archive.save(&mut lease).unwrap();
        let restored = Archive::read(&path).unwrap();
        assert_eq!(restored.metadata, archive.metadata);
        let mut broken = restored.clone();
        let Metadata::Article(m) = &mut broken.metadata else {
            unreachable!()
        };
        m.references.remove(0);
        assert!(
            broken
                .validate()
                .unwrap_err()
                .to_string()
                .contains("bourdieu1980")
        );
        assert_eq!(
            source(&restored.document, &restored.assets).unwrap(),
            restored.validate().unwrap()
        );
    }

    #[test]
    fn citation_badge_has_real_preview_geometry() {
        let node: Node = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"citation","attrs":{"items":[{"file_id":"books","key":"bourdieu1980"}],"form":"normal"}}]}]})).unwrap();
        let mut meta = article::Metadata::demo();
        meta.references = files();
        let template = Model::ArticleWorking.source();
        let mut world = TypstWorld::default();
        world.set_dependencies(&crate::packages::for_template(&template));
        let (body, map) = source_mapped(&node, &Default::default()).unwrap();
        let compiled = world
            .compile_with_template(&body, &Metadata::Article(meta), &template)
            .unwrap();
        let sync = crate::navigation::RenderSync::new(1, world, compiled, map);
        let point = sync.forward(1).expect("citation position");
        // At the first object, the paragraph boundary and badge share an anchor.
        assert!(matches!(sync.backward(point), Some(0 | 1)));
        assert!(!sync.selection_highlights_exact(1..2).is_empty());
    }

    #[test]
    fn archive_rejects_changed_missing_or_unclaimed_bibliography_files() {
        use std::io::{Read, Write};
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("article.typsto");
        let template = Model::ArticleWorking.source();
        let mut meta = article::Metadata::demo();
        meta.references = files();
        let archive = Archive {
            document: doc(),
            metadata: Metadata::Article(meta),
            dependencies: crate::packages::for_template(&template),
            template,
            assets: Default::default(),
            model_id: Model::ArticleWorking.id().into(),
            yjs: vec![],
        };
        let mut lease = DocumentLease::acquire(&path).unwrap();
        archive.save(&mut lease).unwrap();
        drop(lease);
        let mut original = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let entries: Vec<_> = (0..original.len())
            .map(|index| {
                let mut entry = original.by_index(index).unwrap();
                let mut data = vec![];
                entry.read_to_end(&mut data).unwrap();
                (entry.name().to_owned(), data)
            })
            .collect();
        drop(original);
        for alteration in ["changed", "missing", "unclaimed"] {
            let output = root.path().join(format!("{alteration}.typsto"));
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&output).unwrap());
            for (name, data) in &entries {
                if alteration == "missing" && name == "bibliography/books.bib" {
                    continue;
                }
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(
                    if alteration == "changed" && name == "bibliography/books.bib" {
                        b"tampered"
                    } else {
                        data
                    },
                )
                .unwrap();
            }
            if alteration == "unclaimed" {
                zip.start_file(
                    "bibliography/unclaimed.bib",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
                zip.write_all(b"@book{unclaimed, title={Unknown}}").unwrap();
            }
            zip.finish().unwrap();
            assert!(
                Archive::read(&output)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("inconsistent_state: bibliography"),
                "{alteration}"
            );
        }
    }
}
