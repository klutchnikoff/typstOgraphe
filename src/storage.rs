//! Archive ZIP autonome, jamais extraite sur le disque. Seul le format actuel est pris en charge.
use crate::{
    document::Document,
    document_types::{DocumentType, Metadata},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const EXTENSION: &str = "typsto";
const FORMAT_VERSION: u32 = 7;
const LIMIT: u64 = crate::assets::MAX_DOCUMENT_BYTES;
pub(crate) const EXTRA_ENTRY_LIMIT: u64 = 16 * 1024;
const ENTRIES: [&str; 5] = [
    "manifest.json",
    "main.typ",
    "contenu.typ",
    "metadonnees.json",
    "modele.typ",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDocument {
    pub source: String,
    pub metadata: Metadata,
    pub template: String,
    pub assets: crate::assets::Assets,
}
impl StoredDocument {
    pub fn demo(kind: DocumentType) -> Self {
        Self {
            source: kind.demo_source().into(),
            metadata: Metadata::demo(kind),
            template: kind.default_model(),
            assets: Default::default(),
        }
    }
    pub fn blank(kind: DocumentType) -> Self {
        Self {
            source: String::new(),
            metadata: Metadata::blank(kind),
            template: kind.default_model(),
            assets: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    document_type: DocumentType,
}
impl Manifest {
    fn new(document_type: DocumentType) -> Self {
        Self {
            format: "typstOgraphe".into(),
            version: FORMAT_VERSION,
            document_type,
        }
    }
}

pub fn read(path: &Path) -> Result<StoredDocument, String> {
    let file =
        File::open(path).map_err(|e| crate::tr!("storage-open-failed", error = e.to_string()))?;
    if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT + 512 * 1024 {
        return Err(crate::tr!("storage-file-too-large"));
    }
    decode(file).map_err(|e| crate::tr!("storage-invalid", error = e))
}

fn decode(reader: impl Read + std::io::Seek) -> Result<StoredDocument, String> {
    decode_archive(reader, None).map(|(document, _)| document)
}

/// La récupération partage le format documentaire, avec une entrée privée
/// supplémentaire. Les fichiers utilisateur continuent de refuser cette entrée.
pub(crate) fn decode_archive(
    reader: impl Read + Seek,
    extra_name: Option<&str>,
) -> Result<(StoredDocument, Option<String>), String> {
    let mut zip = ZipArchive::new(reader)
        .map_err(|e| crate::tr!("storage-zip-unreadable", error = e.to_string()))?;
    let extra_count = usize::from(extra_name.is_some());
    if zip.len() < ENTRIES.len() + extra_count || zip.len() > 1024 + extra_count {
        return Err(crate::tr!("storage-file-count"));
    }
    let mut total = 0_u64;
    let mut names = std::collections::HashSet::new();
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(|e| e.to_string())?;
        if (!ENTRIES.contains(&file.name())
            && Some(file.name()) != extra_name
            && !crate::assets::valid_path(file.name()))
            || !names.insert(file.name().to_owned())
        {
            return Err(crate::tr!("storage-unknown-entry"));
        }
        if Some(file.name()) == extra_name {
            if file.size() > EXTRA_ENTRY_LIMIT {
                return Err(crate::tr!("storage-invalid-size"));
            }
        } else {
            total = total
                .checked_add(file.size())
                .ok_or(crate::tr!("storage-invalid-size"))?;
        }
        if total > LIMIT {
            return Err(crate::tr!("storage-expanded-too-large"));
        }
    }
    let manifest: Manifest = serde_json::from_str(&entry(&mut zip, "manifest.json")?)
        .map_err(|e| crate::tr!("storage-manifest-unreadable", error = e.to_string()))?;
    if manifest.format != "typstOgraphe" || manifest.version != FORMAT_VERSION {
        return Err(crate::tr!("storage-unsupported"));
    }
    let mut assets = crate::assets::Assets::new();
    for name in names.iter().filter(|name| crate::assets::valid_path(name)) {
        let mut data = Vec::new();
        zip.by_name(name)
            .map_err(|e| e.to_string())?
            .take(crate::assets::MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        assets.insert(name.clone(), data.into());
    }
    let letter = StoredDocument {
        source: entry(&mut zip, "contenu.typ")?,
        metadata: serde_json::from_str(&entry(&mut zip, "metadonnees.json")?)
            .map_err(|e| crate::tr!("storage-metadata-unreadable", error = e.to_string()))?,
        template: entry(&mut zip, "modele.typ")?,
        assets,
    };
    if manifest.document_type != letter.metadata.document_type() {
        return Err(crate::tr!("storage-unsupported"));
    }
    validate(&letter)?;
    if entry(&mut zip, "main.typ")? != letter.metadata.main_source() {
        return Err(crate::tr!("storage-main-mismatch"));
    }
    let extra = extra_name.map(|name| entry(&mut zip, name)).transpose()?;
    Ok((letter, extra))
}

fn entry<R: Read + std::io::Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<String, String> {
    let file = zip
        .by_name(name)
        .map_err(|e| crate::tr!("storage-entry-missing", name = name, error = e.to_string()))?;
    let mut value = String::new();
    file.take(LIMIT + 1)
        .read_to_string(&mut value)
        .map_err(|e| {
            crate::tr!(
                "storage-entry-unreadable",
                name = name,
                error = e.to_string()
            )
        })?;
    if value.len() as u64 > LIMIT {
        return Err(crate::tr!("storage-entry-too-large", name = name));
    }
    Ok(value)
}

fn validate(letter: &StoredDocument) -> Result<(), String> {
    let document = Document::from_source(&letter.source)?;
    if let Some(path) = letter.metadata.signature_path() {
        if !crate::assets::valid_path(path) {
            return Err(crate::tr!("storage-unknown-entry"));
        }
        if !letter.assets.contains_key(path) {
            return Err(crate::tr!("image-resource-missing", path = path));
        }
    }
    for image in document.view.images.values() {
        if !letter.assets.contains_key(&image.path) {
            return Err(crate::tr!(
                "image-resource-missing",
                path = image.path.clone()
            ));
        }
    }
    for (path, data) in &letter.assets {
        if !crate::assets::valid_path(path) {
            return Err(crate::tr!("storage-unknown-entry"));
        }
        crate::assets::decode(data, path.rsplit('.').next().unwrap())?;
    }
    if letter.template.trim().is_empty()
        || typst_syntax::Source::detached(&letter.template)
            .root()
            .diagnosis()
            .errors
    {
        return Err(crate::tr!("storage-invalid-template"));
    }
    Ok(())
}

/// Écrit et synchronise un fichier temporaire voisin, puis remplace la cible
/// par renommage atomique. Une erreur d'écriture conserve le courrier précédent.
pub fn save(path: &Path, letter: &StoredDocument) -> Result<(), String> {
    let mut letter = letter.clone();
    letter.assets = crate::rendering::retained_assets(
        &letter.source,
        &letter.metadata,
        &letter.template,
        &letter.assets,
    )?;
    save_archive(path, &letter, None)
}

/// Une seule voie d’écriture atomique pour les documents et leur récupération.
/// Les copies de récupération conservent les ressources sans lancer de rendu.
pub(crate) fn save_archive(
    path: &Path,
    letter: &StoredDocument,
    extra: Option<(&str, &str)>,
) -> Result<(), String> {
    validate(letter)?;
    if letter.assets.len() + ENTRIES.len() > 1024 {
        return Err(crate::tr!("storage-file-count"));
    }
    if extra.is_some_and(|(_, text)| text.len() as u64 > EXTRA_ENTRY_LIMIT) {
        return Err(crate::tr!("storage-invalid-size"));
    }
    let manifest = serde_json::to_string_pretty(&Manifest::new(letter.metadata.document_type()))
        .map_err(|e| e.to_string())?;
    let metadata = serde_json::to_string_pretty(&letter.metadata).map_err(|e| e.to_string())?;
    let main = letter.metadata.main_source();
    let files = [
        manifest.as_str(),
        main.as_str(),
        letter.source.as_str(),
        metadata.as_str(),
        letter.template.as_str(),
    ];
    if files.iter().map(|text| text.len() as u64).sum::<u64>()
        + letter
            .assets
            .values()
            .map(|data| data.len() as u64)
            .sum::<u64>()
        > LIMIT
    {
        return Err(crate::tr!("storage-letter-too-large"));
    }
    let options = atomic_write_file::AtomicWriteFile::options();
    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = options;
        options.mode(0o600);
        options
    };
    let mut temporary = options
        .open(path)
        .map_err(|e| crate::tr!("storage-temporary-failed", error = e.to_string()))?;
    {
        let mut zip = ZipWriter::new(&mut temporary);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in ENTRIES.into_iter().zip(files) {
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            zip.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        for (name, data) in &letter.assets {
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            zip.write_all(data).map_err(|e| e.to_string())?;
        }
        if let Some((name, text)) = extra {
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            zip.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    temporary
        .commit()
        .map_err(|e| crate::tr!("storage-replace-failed", error = e.to_string()))?;
    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn rewrite(path: &Path, edit: impl FnOnce(&mut Vec<(String, Vec<u8>)>)) {
        let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut entries = Vec::new();
        for index in 0..zip.len() {
            let mut file = zip.by_index(index).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            entries.push((file.name().to_owned(), bytes));
        }
        drop(zip);
        edit(&mut entries);
        let mut writer = ZipWriter::new(File::create(path).unwrap());
        for (name, bytes) in entries {
            writer
                .start_file(
                    name,
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn archive_and_export_omit_unused_images_but_keep_document_and_model_resources() {
        let dir = tempfile::tempdir().unwrap();
        let mut letter = StoredDocument::demo(DocumentType::Courrier);
        let bytes: std::sync::Arc<[u8]> = include_bytes!("../tests/fixtures/parc.svg")
            .as_slice()
            .into();
        for name in ["active", "logo", "deleted"] {
            letter
                .assets
                .insert(format!("images/{name}.svg"), bytes.clone());
        }
        letter.source = "#figure(image(\"images/active.svg\"), caption: [Un parc.])".into();
        letter.template = letter.template.replace(
            "  body\n",
            "  image(\"images/\" + \"logo\" + \".svg\", width: 12pt)\n  body\n",
        );
        let path = dir.path().join("courrier.typsto");
        let mut world = crate::rendering::TypstWorld::default();
        for with_image in [true, false] {
            if !with_image {
                letter.source = "Texte seul.".into();
            }
            save(&path, &letter).unwrap();
            let mut zip = ZipArchive::new(File::open(&path).unwrap()).unwrap();
            assert_eq!(zip.len(), 6 + usize::from(with_image));
            assert!(zip.by_name("images/deleted.svg").is_err());
            assert!(zip.by_name("images/logo.svg").is_ok());
            assert_eq!(zip.by_name("images/active.svg").is_ok(), with_image);
            let restored = read(&path).unwrap();
            assert_eq!(restored.assets.len(), 1 + usize::from(with_image));
            let export = dir.path().join(if with_image {
                "avec-image"
            } else {
                "sans-image"
            });
            crate::rendering::render(
                &mut world,
                &crate::rendering::RenderRequest {
                    revision: 1,
                    source: letter.source.clone(),
                    source_map: Document::from_source(&letter.source).unwrap().source_map,
                    metadata: letter.metadata.clone(),
                    template: letter.template.clone(),
                    assets: letter.assets.clone(),
                    export_dir: Some(export.clone()),
                },
            )
            .unwrap();
            assert_eq!(
                std::fs::read_dir(export.join("images")).unwrap().count(),
                1 + usize::from(with_image)
            );
            assert!(!export.join("images/deleted.svg").exists());
            assert!(export.join("images/logo.svg").exists());
            assert_eq!(export.join("images/active.svg").exists(), with_image);
            let mut clean_world = crate::rendering::TypstWorld::default();
            clean_world.set_assets(&restored.assets);
            clean_world
                .compile_with_template(&restored.source, &restored.metadata, &restored.template)
                .unwrap();
        }
        assert_eq!(letter.assets.len(), 3);
    }

    #[test]
    fn image_archive_is_self_contained_and_exported_sources_recompile() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("Images.typsto");
        let mut letter = StoredDocument::demo(DocumentType::Courrier);
        letter.source =
            "#figure(image(\"images/parc.svg\", alt: \"Un parc arboré\"), caption: [Le *parc*.])"
                .into();
        letter.assets.insert(
            "images/parc.svg".into(),
            include_bytes!("../tests/fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        save(&archive, &letter).unwrap();
        let restored = read(&archive).unwrap();
        assert_eq!(restored, letter);
        let mut world = crate::rendering::TypstWorld::default();
        let document = Document::from_source(&restored.source).unwrap();
        let export = dir.path().join("Export");
        let request = crate::rendering::RenderRequest {
            revision: 1,
            source: restored.source,
            source_map: document.source_map,
            metadata: restored.metadata,
            template: restored.template,
            assets: restored.assets,
            export_dir: Some(export.clone()),
        };
        let output = crate::rendering::render(&mut world, &request).unwrap();
        assert!(!output.pages.is_empty());
        assert_eq!(
            std::fs::read(export.join("images/parc.svg")).unwrap(),
            include_bytes!("../tests/fixtures/parc.svg")
        );
        // Le moteur ne lit aucun fichier local : réinjecter uniquement l’export
        // vérifie que toutes les ressources nécessaires ont bien été exportées.
        let mut exported = crate::assets::Assets::new();
        exported.insert(
            "images/parc.svg".into(),
            std::fs::read(export.join("images/parc.svg"))
                .unwrap()
                .into(),
        );
        let mut other_world = crate::rendering::TypstWorld::default();
        other_world.set_assets(&exported);
        other_world
            .compile_with_template(
                &std::fs::read_to_string(export.join("contenu.typ")).unwrap(),
                &request.metadata,
                &std::fs::read_to_string(export.join("modele.typ")).unwrap(),
            )
            .unwrap();
        for corrupt in 0..4 {
            save(&archive, &letter).unwrap();
            rewrite(&archive, |files| match corrupt {
                0 => {
                    files.pop();
                }
                1 => files.last_mut().unwrap().0 = "images/../escape.svg".into(),
                2 => files.last_mut().unwrap().1 = b"invalide".to_vec(),
                _ => files.last_mut().unwrap().0 = "images/parc.png".into(),
            });
            assert!(read(&archive).is_err(), "cas {corrupt}");
        }
    }

    #[test]
    fn archive_round_trip_preserves_exact_source_metadata_and_custom_template() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Mon courrier éà.typsto");
        let mut letter = StoredDocument::demo(DocumentType::Courrier);
        letter.metadata.courrier_mut().objet = "Une demande « particulière » # \\".into();
        letter.metadata.options_mut().paper = crate::document::PaperFormat::A4;
        letter.metadata.options_mut().landscape = false;
        letter.metadata.options_mut().language = crate::document::DocumentLanguage::English;
        letter.metadata.options_mut().bullet_marker = Some("◆".into());
        letter
            .source
            .push_str("\n\n/ Échéance: Réponse avant vendredi.\n");
        letter.source.push_str(
            "\n#quote(block: true, attribution: [Service _municipal_])[Message à conserver.]\n",
        );
        letter.template = crate::document_types::models::Model::Personal.source();
        letter
            .template
            .push_str("\n#let editor-options = (bullet-markers: (\"◆\",),)\n");
        letter.template = letter
            .template
            .replace("VILLE DE VALMONT", "MAIRIE D’ESSAI");
        save(&path, &letter).unwrap();
        assert!(std::fs::read(&path).unwrap().starts_with(b"PK"));
        assert_eq!(read(&path).unwrap(), letter);
        let mut zip = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        assert_eq!(zip.len(), 5);
        assert_eq!(entry(&mut zip, "contenu.typ").unwrap(), letter.source);
        assert_eq!(
            entry(&mut zip, "main.typ").unwrap(),
            letter.metadata.main_source()
        );
        let restored = read(&path).unwrap();
        let mut world = crate::rendering::TypstWorld::default();
        let document = Document::from_source(&restored.source).unwrap();
        let export = dir.path().join("export");
        let rendered = crate::rendering::render(
            &mut world,
            &crate::rendering::RenderRequest {
                revision: 1,
                assets: restored.assets.clone(),
                source: restored.source.clone(),
                source_map: document.source_map,
                metadata: restored.metadata,
                template: restored.template.clone(),
                export_dir: Some(export.clone()),
            },
        )
        .unwrap();
        assert!(!rendered.pages.is_empty());
        assert_eq!(
            std::fs::read_to_string(export.join("modele.typ")).unwrap(),
            restored.template
        );
        assert_eq!(
            typst::World::source(&world, crate::rendering::file_id("modele.typ"))
                .unwrap()
                .text(),
            letter.template
        );
    }

    #[test]
    fn replacing_an_archive_is_atomic_and_invalid_save_preserves_existing_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("courrier.typsto");
        let mut letter = StoredDocument::demo(DocumentType::Courrier);
        save(&path, &letter).unwrap();
        letter.metadata.courrier_mut().objet = "Un objet modifié".into();
        save(&path, &letter).unwrap();
        assert_eq!(read(&path).unwrap(), letter);
        let previous = std::fs::read(&path).unwrap();
        letter.source = "#panic(\"invalide\")".into();
        assert!(save(&path, &letter).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        assert!(
            save(
                &dir.path().join("inexistant/courrier.typsto"),
                &StoredDocument::demo(DocumentType::Courrier)
            )
            .is_err()
        );
        let target = dir.path().join("dossier");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("conserver"), "intact").unwrap();
        assert!(save(&target, &StoredDocument::demo(DocumentType::Courrier)).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("conserver")).unwrap(),
            "intact"
        );
    }

    #[test]
    fn rejects_corrupt_future_incomplete_and_unsupported_archives_without_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("courrier.typsto");
        std::fs::write(&path, "ceci n’est pas une archive").unwrap();
        assert!(read(&path).is_err());
        for invalid in 0..10 {
            save(&path, &StoredDocument::demo(DocumentType::Courrier)).unwrap();
            rewrite(&path, |files| match invalid {
                0 => {
                    files[0].1 =
                        br#"{"format":"typstOgraphe","version":42,"document_type":"courrier"}"#
                            .to_vec()
                }
                1 => {
                    files.pop();
                }
                2 => files[0].0 = "../hors-archive".into(),
                3 => files[2].1 = b"#unknown[construction non prise en charge]".to_vec(),
                4 => files[3].1 = b"{json invalide".to_vec(),
                5 => files[4].1 = b"#let courrier(".to_vec(),
                6 => files[1].1 = b"en-tete incoherent".to_vec(),
                8 => {
                    files[0].1 =
                        br#"{"format":"typstOgraphe","version":3,"document_type":"inconnu"}"#
                            .to_vec()
                }
                9 => files[3].1 = br#"{"type":"inconnu","fields":{}}"#.to_vec(),
                _ => files[2].1 = vec![b'x'; LIMIT as usize + 1],
            });
            assert!(read(&path).is_err(), "cas {invalid}");
        }
        assert!(!dir.path().parent().unwrap().join("hors-archive").exists());
    }
}
