//! Validated document payload; CAS ownership and IO live in `content::project`.
use super::{Node, source};
use crate::{
    assets::Assets,
    document_types::Metadata,
    errors::{CoreError, StorageError},
    packages::Dependencies,
};
/// Typed causes survive until an adapter renders them through Fluent.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Content(#[from] super::contract::Error),
    #[error("{0}")]
    Core(#[from] CoreError),
    #[error("{0}")]
    Access(#[from] crate::protocol::CommandError),
}
impl From<StorageError> for Error {
    fn from(error: StorageError) -> Self {
        Self::Core(error.into())
    }
}
impl Error {
    fn document(reason: String) -> Self {
        super::contract::Error::InvalidDocument { reason }.into()
    }
}
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
impl Archive {
    /// Isolated read-only payload shared by Versions and proposal comparisons.
    pub fn view(&self) -> Result<serde_json::Value, Error> {
        use base64::Engine;
        let references = if let Metadata::Article(metadata) = &self.metadata {
            crate::bibliography::index(&metadata.references).map_err(Error::document)?
        } else {
            vec![]
        };
        Ok(
            serde_json::json!({"state":self.state(), "references":references,
            "resources":self.assets.iter().map(|(path,data)|serde_json::json!({"path":path,"data":base64::engine::general_purpose::STANDARD.encode(data)})).collect::<Vec<_>>()}),
        )
    }
    pub fn state(&self) -> super::contract::State {
        super::contract::State::new(
            self.document.clone(),
            super::contract::Settings {
                metadata: self.metadata.clone(),
                template: self.template.clone(),
                model_id: self.model_id.clone(),
            },
        )
    }
    /// Verify the project's pinned inventory, independently of the installed catalogue.
    pub fn validate(&self) -> Result<String, Error> {
        crate::packages::validate_embedded(&self.template, &self.dependencies)?;
        self.validate_content()
    }
    fn validate_content(&self) -> Result<String, Error> {
        self.metadata.validate().map_err(Error::document)?;
        crate::bibliography::validate_document(&self.document, &self.metadata)
            .map_err(Error::document)?;
        for (path, data) in &self.assets {
            if !crate::assets::valid_path(path) {
                return Err(StorageError::UnexpectedEntry { name: path.clone() }.into());
            }
            crate::assets::decode(data, path.rsplit('.').next().unwrap_or(""))
                .map_err(CoreError::from)?;
        }
        if let Some(signature) = self.metadata.signature_path()
            && !self.assets.contains_key(signature)
        {
            return Err(StorageError::ImageResourceMissing {
                path: signature.into(),
            }
            .into());
        }
        let source = source(&self.document, &self.assets).map_err(Error::document)?;
        let state = self.state();
        state.settings.validate()?;
        super::catalogue::validate_document(self.metadata.document_type(), &self.document)?;
        if !self.yjs.is_empty() {
            super::shared::SharedDocument::from_update(&self.yjs)
                .and_then(|shared| shared.verify(&state))?;
        }
        Ok(source)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{DocumentLease, content::project::Project};
    fn save(archive: &Archive, lease: &mut DocumentLease) -> Result<(), Error> {
        Project::new(archive.clone())?.save(lease)
    }
    fn read(path: &std::path::Path) -> Archive {
        Project::read(path).unwrap().current().clone()
    }
    fn fixture() -> Archive {
        let metadata = Metadata::blank(crate::document_types::DocumentType::Courrier);
        let template = metadata.document_type().default_model();
        Archive{document:serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Un texte é😀"},{"type":"footnote","attrs":{"text":"Une note."}}]}]})).unwrap(),metadata,dependencies:crate::packages::for_template(&template),template,model_id:"courrier.model.personal".into(),assets:Assets::default(),yjs:vec![]}
    }
    #[test]
    fn saved_model_snapshot_remains_self_contained_after_catalogue_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapshot.typsto");
        let mut archive = fixture();
        archive
            .template
            .push_str("\n// A project-pinned version of the bundled model.\n");
        archive.yjs = super::super::shared::SharedDocument::from_state(&archive.state())
            .unwrap()
            .update();
        let mut lease = DocumentLease::acquire(&path).unwrap();
        save(&archive, &mut lease).unwrap();
        let saved = read(&path);
        assert_eq!(saved.template, archive.template);
        assert_eq!(saved.model_id, archive.model_id);
        assert_eq!(saved.yjs, archive.yjs);
        assert_eq!(saved.document, archive.document);
        assert_eq!(saved.metadata, archive.metadata);
        // Changing only the archive model cannot conceal divergence in its shared state.
        let mut changed = saved;
        changed.template.push_str("\n// divergent checkpoint");
        assert!(
            matches!(changed.validate(), Err(Error::Content(super::super::contract::Error::InconsistentState { field })) if field == "settings")
        );
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
        save(&archive, &mut lease).unwrap();
        let saved = read(&path);
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
        save(&archive, &mut lease).unwrap();
        let restored = read(&path);
        assert_eq!(restored.document, archive.document);
        assert_eq!(restored.metadata, archive.metadata);
        assert!(!restored.yjs.is_empty());
        super::super::shared::SharedDocument::from_update(&restored.yjs)
            .unwrap()
            .verify(&archive.state())
            .unwrap();
        assert!(DocumentLease::acquire(&path).is_err());
        std::fs::write(&path, b"modification externe").unwrap();
        assert!(save(&archive, &mut lease).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"modification externe");
    }
    #[test]
    fn invalid_document_cannot_replace_a_saved_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.typsto");
        let mut archive = fixture();
        let mut lease = DocumentLease::acquire(&path).unwrap();
        save(&archive, &mut lease).unwrap();
        let before = std::fs::read(&path).unwrap();
        archive.document.kind = "unknown".into();
        assert!(save(&archive, &mut lease).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
