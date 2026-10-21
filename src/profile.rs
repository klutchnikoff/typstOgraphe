//! Identité globale ; chaque nouveau document reçoit ses propres valeurs et ressources.
use crate::{
    assets::ImageData,
    document_types::{DocumentType, Metadata},
    storage::StoredDocument,
};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub role: String,
    pub organisation: String,
    pub signature: Option<String>,
}

impl Profile {
    pub fn load_signature(&self, directory: &Path) -> Result<Option<ImageData>, String> {
        self.signature
            .as_ref()
            .map(|name| {
                if !name.starts_with("signature-")
                    || !crate::assets::valid_path(&format!("images/{name}"))
                {
                    return Err(crate::tr!("profile-invalid-signature"));
                }
                ImageData::read(&directory.join("profile").join(name))
            })
            .transpose()
    }
    pub fn import_signature(
        directory: &Path,
        source: &Path,
    ) -> Result<(String, ImageData), String> {
        let image = ImageData::read(source)?;
        let name = format!(
            "signature-{:032x}.{}",
            typst::utils::hash128(&image.data),
            image.extension
        );
        let directory = directory.join("profile");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        // Fichiers immuables : une écriture différée des préférences ne peut pas
        // rendre leur ancienne référence illisible après un remplacement d'image.
        let options = atomic_write_file::AtomicWriteFile::options();
        #[cfg(unix)]
        let options = {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = options;
            options.mode(0o600);
            options
        };
        let mut file = options
            .open(directory.join(&name))
            .map_err(|e| e.to_string())?;
        file.write_all(&image.data).map_err(|e| e.to_string())?;
        file.commit().map_err(|e| e.to_string())?;
        Ok((name, image))
    }
    pub fn document(
        &self,
        kind: DocumentType,
        signature: Option<&ImageData>,
    ) -> Result<StoredDocument, String> {
        let mut document = StoredDocument::blank(kind);
        match &mut document.metadata {
            Metadata::Courrier(meta) => {
                meta.expediteur = self.name.clone();
                meta.adresse_expediteur = self.address.clone();
                meta.email = self.email.clone();
                meta.telephone = self.phone.clone();
                meta.fonction = self.role.clone();
                meta.organisme = self.organisation.clone();
                if self.signature.is_some() {
                    let signature =
                        signature.ok_or_else(|| crate::tr!("profile-signature-unavailable"))?;
                    meta.signature = Some(signature.insert(&mut document.assets, "signature"));
                }
            }
            Metadata::CompteRendu(meta) => meta.redacteur = self.name.clone(),
        }
        Ok(document)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn profile_resources_are_managed_and_documents_survive_original_and_global_image_removal() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.svg");
        std::fs::write(&original, include_bytes!("../tests/fixtures/signature.svg")).unwrap();
        let (name, image) = Profile::import_signature(dir.path(), &original).unwrap();
        let mut profile = Profile {
            name: "Zoé \"Martin\" #test".into(),
            address: "12 rue des Fleurs\nRennes".into(),
            email: "zoe@example.org".into(),
            role: "Présidente".into(),
            organisation: "Les Tilleuls".into(),
            phone: "0123456789".into(),
            signature: Some(name.clone()),
        };
        std::fs::remove_file(original).unwrap();
        assert_eq!(
            profile.load_signature(dir.path()).unwrap().unwrap().data,
            image.data
        );
        let mut letter = profile
            .document(DocumentType::Courrier, Some(&image))
            .unwrap();
        letter.source = "Bonjour.\n\nUn courrier signé.".into();
        let copied = letter.clone();
        assert!(letter.metadata.courrier().adresse.is_empty()); // L'adresse personnelle n'est pas celle du destinataire.
        profile.name = "Autre personne".into();
        profile.signature = None;
        std::fs::remove_file(dir.path().join("profile").join(&name)).unwrap();
        assert_eq!(letter, copied);
        assert_eq!(
            letter.metadata.signature_path(),
            Some("images/signature-1.svg")
        );
        assert!(
            !letter
                .metadata
                .main_source()
                .contains(dir.path().to_str().unwrap())
        );
        let path = dir.path().join("courrier.typsto");
        crate::storage::save(&path, &letter).unwrap();
        assert_eq!(crate::storage::read(&path).unwrap(), letter);
        let mut world = crate::rendering::TypstWorld::default();
        world.set_assets(&letter.assets);
        let pages = world
            .compile_with_template(&letter.source, &letter.metadata, &letter.template)
            .unwrap();
        fn contains_image(frame: &typst::layout::Frame) -> bool {
            frame.items().any(|(_, item)| match item {
                typst::layout::FrameItem::Image(_, _, _) => true,
                typst::layout::FrameItem::Group(group) => contains_image(&group.frame),
                _ => false,
            })
        }
        assert!(pages.pages().iter().any(|page| contains_image(&page.frame)));
        let mut hidden = letter.clone();
        hidden.template = hidden.template.replace("if signature != none", "if false");
        let hidden_path = dir.path().join("signature-masquee.typsto");
        crate::storage::save(&hidden_path, &hidden).unwrap();
        assert_eq!(
            crate::storage::read(&hidden_path).unwrap().assets,
            letter.assets
        );
        let document = crate::document::Document::from_source(&letter.source).unwrap();
        let export = dir.path().join("export");
        crate::rendering::render(
            &mut world,
            &crate::rendering::RenderRequest {
                revision: 1,
                source: letter.source.clone(),
                source_map: document.source_map,
                metadata: letter.metadata.clone(),
                template: letter.template.clone(),
                assets: letter.assets.clone(),
                dependencies: letter.dependencies.clone(),
                export_dir: Some(export.clone()),
            },
        )
        .unwrap();
        assert!(export.join("courrier.pdf").exists());
        assert_eq!(
            std::fs::read(export.join("images/signature-1.svg")).unwrap(),
            image.data.as_ref()
        );
        let mut no_image = letter.clone();
        no_image.assets.clear();
        assert!(crate::storage::save(&path, &no_image).is_err());
        assert_eq!(crate::storage::read(&path).unwrap(), letter);
        let meeting = profile.document(DocumentType::CompteRendu, None).unwrap();
        let Metadata::CompteRendu(meta) = meeting.metadata else {
            panic!()
        };
        assert_eq!(meta.redacteur, "Autre personne");
        assert!(meeting.assets.is_empty());
    }
    #[test]
    fn invalid_or_missing_profile_images_are_explicit_and_never_become_external_document_paths() {
        let dir = tempfile::tempdir().unwrap();
        let mut profile = Profile {
            signature: Some("../../outside.svg".into()),
            ..Default::default()
        };
        assert!(profile.load_signature(dir.path()).is_err());
        profile.signature = Some("signature-absent.svg".into());
        assert!(profile.load_signature(dir.path()).is_err());
        assert!(profile.document(DocumentType::Courrier, None).is_err());
        assert!(profile.document(DocumentType::CompteRendu, None).is_ok());
        let invalid = dir.path().join("invalid.svg");
        std::fs::write(&invalid, b"not an image").unwrap();
        assert!(Profile::import_signature(dir.path(), &invalid).is_err());
        assert!(!dir.path().join("profile").exists());
    }
}
