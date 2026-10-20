//! Format local v1 : archive ZIP autonome, jamais extraite sur le disque.
use crate::{
    document::{Document, Metadata},
    rendering::TEMPLATE,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const EXTENSION: &str = "typsto";
const LIMIT: u64 = 16 * 1024 * 1024;
const ENTRIES: [&str; 5] = [
    "manifest.json",
    "main.typ",
    "contenu.typ",
    "metadonnees.json",
    "modele.typ",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Letter {
    pub source: String,
    pub metadata: Metadata,
    pub template: String,
    pub compact: bool,
}
impl Letter {
    pub fn demo() -> Self {
        Self {
            source: Document::demo().source().into(),
            metadata: Metadata::default(),
            template: TEMPLATE.into(),
            compact: false,
        }
    }
    pub fn blank() -> Self {
        Self {
            source: String::new(),
            metadata: Metadata {
                destinataire: String::new(),
                adresse: String::new(),
                objet: String::new(),
                reference: String::new(),
                date: String::new(),
                ..Metadata::default()
            },
            template: TEMPLATE.into(),
            compact: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    document_type: String,
    compact: bool,
}
impl Manifest {
    fn new(compact: bool) -> Self {
        Self {
            format: "typstOgraphe".into(),
            version: 1,
            document_type: "courrier".into(),
            compact,
        }
    }
}

pub fn read(path: &Path) -> Result<Letter, String> {
    let file = File::open(path).map_err(|e| format!("Impossible d’ouvrir le courrier : {e}"))?;
    if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT + 65536 {
        return Err("Le fichier de courrier dépasse la taille autorisée.".into());
    }
    decode(file).map_err(|e| format!("Courrier invalide : {e}"))
}

fn decode(reader: impl Read + std::io::Seek) -> Result<Letter, String> {
    let mut zip = ZipArchive::new(reader).map_err(|e| format!("archive ZIP illisible ({e})"))?;
    if zip.len() != ENTRIES.len() {
        return Err("l’archive doit contenir exactement les cinq fichiers du format v1".into());
    }
    let mut total = 0_u64;
    let mut names = std::collections::HashSet::new();
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(|e| e.to_string())?;
        if !ENTRIES.contains(&file.name()) || !names.insert(file.name().to_owned()) {
            return Err("fichier inconnu ou présent plusieurs fois dans l’archive".into());
        }
        total = total
            .checked_add(file.size())
            .ok_or("taille d’archive invalide")?;
        if total > LIMIT {
            return Err("le contenu décompressé dépasse 16 Mio".into());
        }
    }
    let manifest: Manifest = serde_json::from_str(&entry(&mut zip, "manifest.json")?)
        .map_err(|e| format!("manifeste illisible ({e})"))?;
    if manifest.format != "typstOgraphe"
        || manifest.version != 1
        || manifest.document_type != "courrier"
    {
        return Err("format, version ou type de document non pris en charge".into());
    }
    let letter = Letter {
        source: entry(&mut zip, "contenu.typ")?,
        metadata: serde_json::from_str(&entry(&mut zip, "metadonnees.json")?)
            .map_err(|e| format!("métadonnées illisibles ({e})"))?,
        template: entry(&mut zip, "modele.typ")?,
        compact: manifest.compact,
    };
    validate(&letter)?;
    if entry(&mut zip, "main.typ")? != letter.metadata.main_source(letter.compact) {
        return Err("l’en-tête main.typ ne correspond pas aux métadonnées du courrier".into());
    }
    Ok(letter)
}

fn entry<R: Read + std::io::Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<String, String> {
    let file = zip
        .by_name(name)
        .map_err(|e| format!("{name} manquant ou illisible ({e})"))?;
    let mut value = String::new();
    file.take(LIMIT + 1)
        .read_to_string(&mut value)
        .map_err(|e| format!("{name} illisible ({e})"))?;
    if value.len() as u64 > LIMIT {
        return Err(format!("{name} dépasse la taille autorisée"));
    }
    Ok(value)
}

fn validate(letter: &Letter) -> Result<(), String> {
    Document::from_source(&letter.source)?;
    if letter.template.trim().is_empty()
        || typst_syntax::Source::detached(&letter.template)
            .root()
            .diagnosis()
            .errors
    {
        return Err("le modèle Typst est vide ou contient une erreur de syntaxe".into());
    }
    Ok(())
}

/// Écrit et synchronise un fichier temporaire voisin, puis remplace la cible
/// par renommage atomique. Une erreur d'écriture conserve le courrier précédent.
pub fn save(path: &Path, letter: &Letter) -> Result<(), String> {
    validate(letter)?;
    let manifest =
        serde_json::to_string_pretty(&Manifest::new(letter.compact)).map_err(|e| e.to_string())?;
    let metadata = serde_json::to_string_pretty(&letter.metadata).map_err(|e| e.to_string())?;
    let main = letter.metadata.main_source(letter.compact);
    let files = [
        manifest.as_str(),
        main.as_str(),
        letter.source.as_str(),
        metadata.as_str(),
        letter.template.as_str(),
    ];
    if files.iter().map(|text| text.len() as u64).sum::<u64>() > LIMIT {
        return Err("Le courrier dépasse la taille autorisée (16 Mio).".into());
    }
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Impossible de créer le fichier temporaire : {e}"))?;
    {
        let mut zip = ZipWriter::new(temporary.as_file_mut());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in ENTRIES.into_iter().zip(files) {
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            zip.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| format!("Écriture du courrier impossible : {e}"))?;
    temporary
        .persist(path)
        .map_err(|e| format!("Impossible de remplacer le courrier : {}", e.error))?;
    Ok(())
}

#[cfg(test)]
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
    fn archive_round_trip_preserves_exact_source_metadata_and_custom_template() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Mon courrier éà.typsto");
        let mut letter = Letter::demo();
        letter.metadata.objet = "Une demande « particulière » # \\".into();
        letter.compact = true;
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
            letter.metadata.main_source(true)
        );
        let restored = read(&path).unwrap();
        let mut world = crate::rendering::TypstWorld::default();
        let document = Document::from_source(&restored.source).unwrap();
        let export = dir.path().join("export");
        let rendered = crate::rendering::render(
            &mut world,
            &crate::rendering::RenderRequest {
                revision: 1,
                source: restored.source.clone(),
                source_map: document.source_map,
                metadata: restored.metadata,
                template: restored.template.clone(),
                compact: restored.compact,
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
        let mut letter = Letter::demo();
        save(&path, &letter).unwrap();
        letter.metadata.objet = "Un objet modifié".into();
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
                &Letter::demo()
            )
            .is_err()
        );
        let target = dir.path().join("dossier");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("conserver"), "intact").unwrap();
        assert!(save(&target, &Letter::demo()).is_err());
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
        for invalid in 0..8 {
            save(&path, &Letter::demo()).unwrap();
            rewrite(&path, |files| {
                match invalid {
                0=>files[0].1=br#"{"format":"typstOgraphe","version":42,"document_type":"courrier","compact":false}"#.to_vec(),
                1=>{files.pop();}
                2=>files[0].0="../hors-archive".into(),
                3=>files[2].1=b"#unknown[construction non prise en charge]".to_vec(),
                4=>files[3].1=b"{json invalide".to_vec(),
                5=>files[4].1=b"#let courrier(".to_vec(),
                6=>files[1].1=b"en-tete incoherent".to_vec(),
                _=>files[2].1=vec![b'x';LIMIT as usize+1],
            }
            });
            assert!(read(&path).is_err(), "cas {invalid}");
        }
        assert!(!dir.path().parent().unwrap().join("hors-archive").exists());
    }
}
