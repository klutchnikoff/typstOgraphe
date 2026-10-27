//! Archive structurée autonome. Les écritures passent par une garde de fichier.
use super::{Node, source};
use crate::{
    DocumentLease,
    assets::{Assets, MAX_DOCUMENT_BYTES},
    document_types::Metadata,
    packages::Dependencies,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
#[derive(Clone)]
pub struct Archive {
    pub document: Node,
    pub metadata: Metadata,
    pub template: String,
    pub model_id: String,
    pub assets: Assets,
    pub dependencies: Dependencies,
    pub yjs: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    model_id: String,
}
impl Archive {
    pub fn validate(&self) -> Result<String, String> {
        self.metadata.validate()?;
        crate::packages::validate(&self.template, &self.dependencies).map_err(|e| e.to_string())?;
        for (path, data) in &self.assets {
            if !crate::assets::valid_path(path) {
                return Err("Chemin de ressource invalide.".into());
            }
            crate::assets::decode(data, path.rsplit('.').next().unwrap_or(""))
                .map_err(|e| e.to_string())?;
        }
        if let Some(signature) = self.metadata.signature_path()
            && !self.assets.contains_key(signature)
        {
            return Err("Signature interne absente.".into());
        }
        source(&self.document, &self.assets)
    }
    pub fn save(&self, lease: &mut DocumentLease) -> Result<(), String> {
        let body = self.validate()?;
        let assets = super::retained_assets(
            &self.document,
            &self.metadata,
            &self.template,
            &self.assets,
            &self.dependencies,
        )?;
        lease.check().map_err(|e| e.to_string())?;
        let entries = [
            (
                "manifest.json",
                serde_json::to_vec(&Manifest {
                    format: "typstOgraphe".into(),
                    version: 10,
                    model_id: self.model_id.clone(),
                })
                .unwrap(),
            ),
            ("document.json", serde_json::to_vec(&self.document).unwrap()),
            ("main.typ", self.metadata.main_source().into_bytes()),
            ("contenu.typ", body.into_bytes()),
            (
                "metadonnees.json",
                serde_json::to_vec(&self.metadata).unwrap(),
            ),
            ("modele.typ", self.template.as_bytes().to_vec()),
            ("state.yjs", self.yjs.clone()),
        ];
        let size: usize = entries
            .iter()
            .map(|(_, b)| b.len())
            .chain(assets.values().map(|b| b.len()))
            .chain(self.dependencies.values().map(|b| b.len()))
            .sum();
        if size as u64 > MAX_DOCUMENT_BYTES
            || assets.len() + self.dependencies.len() + entries.len() > 4096
        {
            return Err("Document trop volumineux.".into());
        }
        let mut options = atomic_write_file::AtomicWriteFile::options();
        options.read(true);
        #[cfg(unix)]
        let options = {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = options;
            options.mode(0o600);
            options
        };
        let mut temporary = options.open(lease.path()).map_err(|e| e.to_string())?;
        {
            let mut zip = ZipWriter::new(&mut temporary);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            for (name, data) in &entries {
                zip.start_file(*name, options).map_err(|e| e.to_string())?;
                zip.write_all(data).map_err(|e| e.to_string())?;
            }
            for (name, data) in assets.iter().chain(self.dependencies.iter()) {
                zip.start_file(name, options).map_err(|e| e.to_string())?;
                zip.write_all(data).map_err(|e| e.to_string())?;
            }
            zip.finish().map_err(|e| e.to_string())?;
        }
        lease.check().map_err(|e| e.to_string())?;
        let fingerprint = lease
            .prepare_fingerprint(temporary.as_file_mut())
            .map_err(|e| e.to_string())?;
        temporary.commit().map_err(|e| e.to_string())?;
        lease.record_fingerprint(fingerprint);
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() > MAX_DOCUMENT_BYTES + 2 * 1024 * 1024
        {
            return Err("Archive trop volumineuse.".into());
        }
        let mut zip = ZipArchive::new(file).map_err(|e| e.to_string())?;
        let allowed = [
            "manifest.json",
            "document.json",
            "main.typ",
            "contenu.typ",
            "metadonnees.json",
            "modele.typ",
            "state.yjs",
        ];
        if zip.len() > 4096 {
            return Err("Trop de fichiers.".into());
        }
        let mut files = std::collections::BTreeMap::new();
        let mut total = 0;
        for index in 0..zip.len() {
            let entry = zip.by_index(index).map_err(|e| e.to_string())?;
            let name = entry.name().to_owned();
            if !allowed.contains(&name.as_str())
                && !crate::assets::valid_path(&name)
                && !crate::packages::valid_path(&name)
            {
                return Err("Entrée inconnue.".into());
            }
            total += entry.size();
            if total > MAX_DOCUMENT_BYTES {
                return Err("Archive décompressée trop volumineuse.".into());
            }
            let mut bytes = Vec::new();
            entry
                .take(MAX_DOCUMENT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if files.insert(name, bytes).is_some() {
                return Err("Entrée dupliquée.".into());
            }
        }
        let get = |name: &str| {
            files
                .get(name)
                .map(Vec::as_slice)
                .ok_or_else(|| format!("Entrée absente : {name}"))
        };
        let manifest: Manifest =
            serde_json::from_slice(get("manifest.json")?).map_err(|e| e.to_string())?;
        if manifest.format != "typstOgraphe" || manifest.version != 10 {
            return Err("Version structurée non prise en charge.".into());
        }
        let result = Self {
            document: serde_json::from_slice(get("document.json")?).map_err(|e| e.to_string())?,
            metadata: serde_json::from_slice(get("metadonnees.json")?)
                .map_err(|e| e.to_string())?,
            template: String::from_utf8(get("modele.typ")?.to_vec()).map_err(|e| e.to_string())?,
            model_id: manifest.model_id,
            yjs: get("state.yjs")?.to_vec(),
            assets: files
                .iter()
                .filter(|(name, _)| crate::assets::valid_path(name))
                .map(|(name, data)| (name.clone(), data.as_slice().into()))
                .collect(),
            dependencies: std::sync::Arc::new(
                files
                    .iter()
                    .filter(|(name, _)| crate::packages::valid_path(name))
                    .map(|(name, data)| (name.clone(), data.as_slice().into()))
                    .collect(),
            ),
        };
        if result.validate()?.as_bytes() != get("contenu.typ")?
            || result.metadata.main_source().as_bytes() != get("main.typ")?
        {
            return Err("Archive incohérente.".into());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Archive {
        let metadata = Metadata::blank(crate::document_types::DocumentType::Courrier);
        let template = metadata.document_type().default_model();
        Archive{document:serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Un texte é😀"},{"type":"footnote","attrs":{"text":"Une note."}}]}]})).unwrap(),metadata,dependencies:crate::packages::for_template(&template),template,model_id:"courrier.model.personal".into(),assets:Assets::default(),yjs:vec![0,0]}
    }
    #[test]
    fn saving_prunes_deleted_images_without_mutating_live_resources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("images.typsto");
        let mut archive = fixture();
        let svg:std::sync::Arc<[u8]>=br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1"/></svg>"#.as_slice().into();
        archive
            .assets
            .insert("images/unused.svg".into(), svg.clone());
        archive.assets.insert("images/used.svg".into(), svg);
        archive.document.content.push(serde_json::from_value(serde_json::json!({"type":"figure","attrs":{"src":"images/used.svg","alt":"A dot"}})).unwrap());
        let mut lease = DocumentLease::acquire(&path).unwrap();
        archive.save(&mut lease).unwrap();
        let saved = Archive::read(&path).unwrap();
        assert!(saved.assets.contains_key("images/used.svg"));
        assert!(!saved.assets.contains_key("images/unused.svg"));
        assert_eq!(archive.assets.len(), 2);
    }
    #[test]
    fn archive_roundtrip_and_guard_protect_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.typsto");
        let archive = fixture();
        let mut lease = DocumentLease::acquire(&path).unwrap();
        archive.save(&mut lease).unwrap();
        let restored = Archive::read(&path).unwrap();
        assert_eq!(restored.document, archive.document);
        assert_eq!(restored.metadata, archive.metadata);
        assert_eq!(restored.yjs, archive.yjs);
        assert!(DocumentLease::acquire(&path).is_err());
        std::fs::write(&path, b"modification externe").unwrap();
        assert!(archive.save(&mut lease).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"modification externe");
    }
    #[test]
    fn invalid_document_cannot_replace_a_saved_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.typsto");
        let mut archive = fixture();
        let mut lease = DocumentLease::acquire(&path).unwrap();
        archive.save(&mut lease).unwrap();
        let before = std::fs::read(&path).unwrap();
        archive.document.kind = "unknown".into();
        assert!(archive.save(&mut lease).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
