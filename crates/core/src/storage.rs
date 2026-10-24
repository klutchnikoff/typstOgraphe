//! Archive ZIP autonome, jamais extraite sur le disque. Seul le format actuel est pris en charge.
use crate::errors::{CoreError, StorageError};
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
const FORMAT_VERSION: u32 = 9;
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
    pub dependencies: crate::packages::Dependencies,
}
impl StoredDocument {
    pub fn demo(kind: DocumentType) -> Self {
        Self {
            source: kind.demo_source().into(),
            metadata: Metadata::demo(kind),
            dependencies: crate::packages::for_template(&kind.default_model()),
            template: kind.default_model(),
            assets: Default::default(),
        }
    }
    pub fn blank(kind: DocumentType) -> Self {
        Self {
            source: String::new(),
            metadata: Metadata::blank(kind),
            dependencies: crate::packages::for_template(&kind.default_model()),
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

pub fn read(path: &Path) -> Result<StoredDocument, CoreError> {
    let file = File::open(path).map_err(|e| {
        CoreError::from(StorageError::OpenFailed {
            error: (e.to_string()).to_string(),
        })
    })?;
    if file.metadata().map_err(CoreError::external)?.len() > LIMIT + 2 * 1024 * 1024 {
        return Err(CoreError::from(StorageError::FileTooLarge));
    }
    decode(file).map_err(|e| CoreError::from(StorageError::Invalid { error: Box::new(e) }))
}

fn decode(reader: impl Read + std::io::Seek) -> Result<StoredDocument, CoreError> {
    decode_archive(reader, None).map(|(document, _)| document)
}

/// La récupération partage le format documentaire, avec une entrée privée
/// supplémentaire. Les fichiers utilisateur continuent de refuser cette entrée.
pub(crate) fn decode_archive(
    reader: impl Read + Seek,
    extra_name: Option<&str>,
) -> Result<(StoredDocument, Option<String>), CoreError> {
    let mut zip = ZipArchive::new(reader).map_err(|e| {
        CoreError::from(StorageError::ZipUnreadable {
            error: (e.to_string()).to_string(),
        })
    })?;
    let extra_count = usize::from(extra_name.is_some());
    if zip.len() < ENTRIES.len() + extra_count || zip.len() > 4096 + extra_count {
        return Err(CoreError::from(StorageError::FileCount));
    }
    let mut total = 0_u64;
    let mut names = std::collections::HashSet::new();
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(CoreError::external)?;
        if (!ENTRIES.contains(&file.name())
            && Some(file.name()) != extra_name
            && !crate::assets::valid_path(file.name())
            && !crate::packages::valid_path(file.name()))
            || !names.insert(file.name().to_owned())
        {
            return Err(CoreError::from(StorageError::UnknownEntry));
        }
        if Some(file.name()) == extra_name {
            if file.size() > EXTRA_ENTRY_LIMIT {
                return Err(CoreError::from(StorageError::InvalidSize));
            }
        } else {
            total = total
                .checked_add(file.size())
                .ok_or(CoreError::from(StorageError::InvalidSize))?;
        }
        if total > LIMIT {
            return Err(CoreError::from(StorageError::ExpandedTooLarge));
        }
    }
    let manifest: Manifest =
        serde_json::from_str(&entry(&mut zip, "manifest.json")?).map_err(|e| {
            CoreError::from(StorageError::ManifestUnreadable {
                error: (e.to_string()).to_string(),
            })
        })?;
    if manifest.format != "typstOgraphe" || manifest.version != FORMAT_VERSION {
        return Err(CoreError::from(StorageError::Unsupported));
    }
    let mut assets = crate::assets::Assets::new();
    for name in names.iter().filter(|name| crate::assets::valid_path(name)) {
        let mut data = Vec::new();
        zip.by_name(name)
            .map_err(CoreError::external)?
            .take(crate::assets::MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(CoreError::external)?;
        assets.insert(name.clone(), data.into());
    }
    let mut dependencies = std::collections::BTreeMap::new();
    for name in names
        .iter()
        .filter(|name| crate::packages::valid_path(name))
    {
        let mut data = Vec::new();
        zip.by_name(name)
            .map_err(CoreError::external)?
            .take(crate::assets::MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(CoreError::external)?;
        dependencies.insert(name.clone(), data.into());
    }
    let letter = StoredDocument {
        source: entry(&mut zip, "contenu.typ")?,
        metadata: serde_json::from_str(&entry(&mut zip, "metadonnees.json")?).map_err(|e| {
            CoreError::from(StorageError::MetadataUnreadable {
                error: (e.to_string()).to_string(),
            })
        })?,
        template: entry(&mut zip, "modele.typ")?,
        assets,
        dependencies: std::sync::Arc::new(dependencies),
    };
    if manifest.document_type != letter.metadata.document_type() {
        return Err(CoreError::from(StorageError::Unsupported));
    }
    validate(&letter)?;
    if entry(&mut zip, "main.typ")? != letter.metadata.main_source() {
        return Err(CoreError::from(StorageError::MainMismatch));
    }
    let extra = extra_name.map(|name| entry(&mut zip, name)).transpose()?;
    Ok((letter, extra))
}

fn entry<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Result<String, CoreError> {
    let file = zip.by_name(name).map_err(|e| {
        CoreError::from(StorageError::EntryMissing {
            name: (name).to_string(),
            error: (e.to_string()).to_string(),
        })
    })?;
    let mut value = String::new();
    file.take(LIMIT + 1)
        .read_to_string(&mut value)
        .map_err(|e| {
            CoreError::from(StorageError::EntryUnreadable {
                name: (name).to_string(),
                error: (e.to_string()).to_string(),
            })
        })?;
    if value.len() as u64 > LIMIT {
        return Err(CoreError::from(StorageError::EntryTooLarge {
            name: (name).to_string(),
        }));
    }
    Ok(value)
}

pub(crate) fn validate(letter: &StoredDocument) -> Result<(), CoreError> {
    crate::packages::validate(&letter.template, &letter.dependencies)?;
    let document = Document::from_source(&letter.source)?;
    if let Some(path) = letter.metadata.signature_path() {
        if !crate::assets::valid_path(path) {
            return Err(CoreError::from(StorageError::UnknownEntry));
        }
        if !letter.assets.contains_key(path) {
            return Err(CoreError::from(StorageError::ImageResourceMissing {
                path: (path).to_string(),
            }));
        }
    }
    for image in document.view.images.values() {
        if !letter.assets.contains_key(&image.path) {
            return Err(CoreError::from(StorageError::ImageResourceMissing {
                path: (image.path.clone()).to_string(),
            }));
        }
    }
    for (path, data) in &letter.assets {
        if !crate::assets::valid_path(path) {
            return Err(CoreError::from(StorageError::UnknownEntry));
        }
        crate::assets::decode(data, path.rsplit('.').next().unwrap())?;
    }
    if letter.template.trim().is_empty()
        || typst_syntax::Source::detached(&letter.template)
            .root()
            .diagnosis()
            .errors
    {
        return Err(CoreError::from(StorageError::InvalidTemplate));
    }
    Ok(())
}

/// Écrit et synchronise un fichier temporaire voisin, puis remplace la cible
/// par renommage atomique. Une erreur d'écriture conserve le courrier précédent.
pub fn save(path: &Path, letter: &StoredDocument) -> Result<(), CoreError> {
    let mut letter = letter.clone();
    letter.assets = crate::rendering::retained_assets(
        &letter.source,
        &letter.metadata,
        &letter.template,
        &letter.assets,
        &letter.dependencies,
    )?;
    save_archive(path, &letter, None)
}

/// Publication d’un nouveau fichier sans écrasement, y compris si une autre
/// application crée la cible pendant la préparation de l’archive.
/// L’archive passe par la même écriture atomique avant sa publication sans remplacement.
pub fn save_new(path: &Path, letter: &StoredDocument) -> Result<(), CoreError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(CoreError::external)?
        .into_temp_path();
    save(&temporary, letter)?;
    temporary
        .persist_noclobber(path)
        .map_err(CoreError::external)?;
    Ok(())
}

/// Une seule voie d’écriture atomique pour les documents et leur récupération.
/// Les copies de récupération conservent les ressources sans lancer de rendu.
pub(crate) fn save_archive(
    path: &Path,
    letter: &StoredDocument,
    extra: Option<(&str, &str)>,
) -> Result<(), CoreError> {
    validate(letter)?;
    if letter.assets.len() + letter.dependencies.len() + ENTRIES.len() > 4096 {
        return Err(CoreError::from(StorageError::FileCount));
    }
    if extra.is_some_and(|(_, text)| text.len() as u64 > EXTRA_ENTRY_LIMIT) {
        return Err(CoreError::from(StorageError::InvalidSize));
    }
    let manifest = serde_json::to_string_pretty(&Manifest::new(letter.metadata.document_type()))
        .map_err(CoreError::external)?;
    let metadata = serde_json::to_string_pretty(&letter.metadata).map_err(CoreError::external)?;
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
        + letter
            .dependencies
            .values()
            .map(|data| data.len() as u64)
            .sum::<u64>()
        > LIMIT
    {
        return Err(CoreError::from(StorageError::LetterTooLarge));
    }
    let options = atomic_write_file::AtomicWriteFile::options();
    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = options;
        options.mode(0o600);
        options
    };
    let mut temporary = options.open(path).map_err(|e| {
        CoreError::from(StorageError::TemporaryFailed {
            error: (e.to_string()).to_string(),
        })
    })?;
    {
        let mut zip = ZipWriter::new(&mut temporary);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in ENTRIES.into_iter().zip(files) {
            zip.start_file(name, options).map_err(CoreError::external)?;
            zip.write_all(text.as_bytes())
                .map_err(CoreError::external)?;
        }
        for (name, data) in letter.assets.iter().chain(letter.dependencies.iter()) {
            zip.start_file(name, options).map_err(CoreError::external)?;
            zip.write_all(data).map_err(CoreError::external)?;
        }
        if let Some((name, text)) = extra {
            zip.start_file(name, options).map_err(CoreError::external)?;
            zip.write_all(text.as_bytes())
                .map_err(CoreError::external)?;
        }
        zip.finish().map_err(CoreError::external)?;
    }
    temporary.commit().map_err(|e| {
        CoreError::from(StorageError::ReplaceFailed {
            error: (e.to_string()).to_string(),
        })
    })?;
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
        let bytes: std::sync::Arc<[u8]> = include_bytes!("../../../fixtures/parc.svg")
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
        assert!(
            letter.template.contains("image(\"images/\" + \"logo\""),
            "injection du logo dans le modèle"
        );
        let path = dir.path().join("courrier.typsto");
        let mut world = crate::rendering::TypstWorld::default();
        for with_image in [true, false] {
            if !with_image {
                letter.source = "Texte seul.".into();
            }
            save(&path, &letter).unwrap();
            let mut zip = ZipArchive::new(File::open(&path).unwrap()).unwrap();
            assert_eq!(
                zip.len(),
                6 + letter.dependencies.len() + usize::from(with_image)
            );
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
                    dependencies: letter.dependencies.clone(),
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
            clean_world.set_dependencies(&restored.dependencies);
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
            include_bytes!("../../../fixtures/parc.svg")
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
            dependencies: restored.dependencies.clone(),
            export_dir: Some(export.clone()),
        };
        let output = crate::rendering::render(&mut world, &request).unwrap();
        assert!(!output.pages.is_empty());
        assert_eq!(
            std::fs::read(export.join("images/parc.svg")).unwrap(),
            include_bytes!("../../../fixtures/parc.svg")
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
        let exported_dependencies = std::sync::Arc::new(
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
        other_world.set_dependencies(&exported_dependencies);
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
                0 => files.retain(|(name, _)| name != "images/parc.svg"),
                _ => {
                    let image = files
                        .iter_mut()
                        .find(|(name, _)| name == "images/parc.svg")
                        .unwrap();
                    match corrupt {
                        1 => image.0 = "images/../escape.svg".into(),
                        2 => image.1 = b"invalide".to_vec(),
                        _ => image.0 = "images/parc.png".into(),
                    }
                }
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
        assert_eq!(zip.len(), 5 + letter.dependencies.len());
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
                dependencies: restored.dependencies.clone(),
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
    #[test]
    fn datify_archive_and_export_use_their_own_complete_local_dependencies() {
        fn rendered_text(
            world: &mut crate::rendering::TypstWorld,
            letter: &StoredDocument,
        ) -> String {
            fn frame_text(frame: &typst::layout::Frame) -> String {
                frame
                    .items()
                    .map(|(_, item)| match item {
                        typst::layout::FrameItem::Group(group) => frame_text(&group.frame),
                        typst::layout::FrameItem::Text(text) => text.text.to_string(),
                        _ => String::new(),
                    })
                    .collect()
            }
            world.set_dependencies(&letter.dependencies);
            let doc = world
                .compile_with_template(&letter.source, &letter.metadata, &letter.template)
                .unwrap();
            doc.pages()
                .iter()
                .map(|page| frame_text(&page.frame))
                .collect()
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("autonome.typsto");
        let mut letter = StoredDocument::blank(DocumentType::Courrier);
        letter.source = format!(
            "{}Un courrier autonome.\n\n#postscript[Un complément.]\n\n#attachment[Formulaire signé]",
            crate::document::LETTER_NOTES_IMPORT
        );
        letter.metadata.courrier_mut().date = "2026-10-02".into();
        let locale = "packages/datify-core-2.1.0/src/translations/locales/fr.toml";
        // Un exemplaire différent de celui du binaire prouve l’origine des données utilisées.
        let original = letter.dependencies[locale].clone();
        std::sync::Arc::make_mut(&mut letter.dependencies).insert(
            locale.into(),
            String::from_utf8(original.to_vec())
                .unwrap()
                .replace("octobre", "MOISARCHIVE")
                .into_bytes()
                .into(),
        );
        save(&path, &letter).unwrap();
        let restored = read(&path).unwrap();
        assert!(restored == letter);
        assert!(
            restored
                .dependencies
                .contains_key("packages/datify-1.3.0/LICENSE")
        );
        assert!(
            restored
                .dependencies
                .contains_key("packages/datify-core-2.1.0/UNICODE-LICENSE.txt")
        );
        let mut world = crate::rendering::TypstWorld::default();
        world.set_dependencies(&Default::default());
        assert!(
            world
                .compile_with_template(&restored.source, &restored.metadata, &restored.template)
                .is_err()
        );
        assert!(rendered_text(&mut world, &restored).contains("MOISARCHIVE"));
        let mut english = restored.clone();
        english.metadata.options_mut().language = crate::document::DocumentLanguage::English;
        assert!(rendered_text(&mut world, &english).contains("October 2, 2026"));
        let export = dir.path().join("export");
        let request = crate::rendering::RenderRequest {
            revision: 1,
            source: restored.source.clone(),
            source_map: Document::from_source(&restored.source).unwrap().source_map,
            metadata: restored.metadata.clone(),
            template: restored.template.clone(),
            assets: restored.assets.clone(),
            dependencies: restored.dependencies.clone(),
            export_dir: Some(export.clone()),
        };
        crate::rendering::render(&mut world, &request).unwrap();
        let mut exported = restored.clone();
        exported.dependencies = std::sync::Arc::new(
            restored
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
        exported.source = std::fs::read_to_string(export.join("contenu.typ")).unwrap();
        exported.template = std::fs::read_to_string(export.join("modele.typ")).unwrap();
        assert!(rendered_text(&mut world, &exported).contains("MOISARCHIVE"));
        // Changer de copie dans le même moteur invalide aussi les données mises en cache.
        std::sync::Arc::make_mut(&mut exported.dependencies).insert(locale.into(), original);
        let text = rendered_text(&mut world, &exported);
        assert!(text.contains("octobre") && !text.contains("MOISARCHIVE"));
        // Une autre implémentation derrière la même signature ne change pas le modèle.
        let unchanged_model = exported.template.clone();
        std::sync::Arc::make_mut(&mut exported.dependencies).insert(
            "capabilities/dates.typ".into(),
            b"#let format-date(value, lang: \"fr\") = [DATEADAPTEE]"
                .as_slice()
                .into(),
        );
        save(&path, &exported).unwrap();
        let adapted = read(&path).unwrap();
        assert_eq!(adapted.template, unchanged_model);
        assert!(rendered_text(&mut world, &adapted).contains("DATEADAPTEE"));
    }

    #[test]
    fn incomplete_or_unsafe_package_resources_are_rejected_before_replacing_a_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("courrier.typsto");
        let letter = StoredDocument::blank(DocumentType::Courrier);
        save(&path, &letter).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        for bad_path in [
            "packages/../escape.typ",
            "packages/datify-1.3.0/src/../../escape.typ",
        ] {
            let mut invalid = letter.clone();
            std::sync::Arc::make_mut(&mut invalid.dependencies)
                .insert(bad_path.into(), b"escape".as_slice().into());
            assert!(save(&path, &invalid).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        // Même une locale inutilisée par ce courrier est indispensable à son autonomie.
        let unused = "packages/datify-core-2.1.0/src/translations/locales/ja.toml";
        let mut invalid = letter.clone();
        assert!(
            std::sync::Arc::make_mut(&mut invalid.dependencies)
                .remove(unused)
                .is_some()
        );
        assert!(save(&path, &invalid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        for missing in [unused, "capabilities/dates.typ", "dependencies.json"] {
            save(&path, &letter).unwrap();
            rewrite(&path, |files| files.retain(|(name, _)| name != missing));
            assert!(read(&path).is_err(), "{missing}");
        }
        save(&path, &letter).unwrap();
        rewrite(&path, |files| {
            let inventory = files
                .iter_mut()
                .find(|(name, _)| name == "dependencies.json")
                .unwrap();
            let mut value: serde_json::Value = serde_json::from_slice(&inventory.1).unwrap();
            value["packages"][0]["version"] = "inconnue".into();
            inventory.1 = serde_json::to_vec(&value).unwrap();
        });
        assert!(read(&path).is_err());
    }
}
