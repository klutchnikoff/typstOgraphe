//! Cycle de vie d’une session. L’UI décide de jeter un brouillon ; le cœur le vérifie.
use super::*;
use crate::{document::Document, document_types::DocumentType, storage::StoredDocument};
use serde_json::json;
use std::path::PathBuf;

impl Engine {
    pub fn empty() -> Self {
        let stored = StoredDocument::blank(DocumentType::Courrier);
        let mut engine: Self =
            Session::new(stored, Document::from_source("").expect("empty"), None, 1).into();
        engine.open = false;
        engine
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub(super) fn check_replacement(
        &self,
        guard: TargetGuard,
        discard: bool,
    ) -> Result<(), CommandError> {
        self.check_guard(guard)?;
        if self.open && self.session.is_dirty() && !discard {
            return Err(CommandError::new(ErrorCode::Unavailable, "unsaved_changes"));
        }
        Ok(())
    }
    pub(super) fn check_files(&self, command: &Command) -> Result<(), CommandError> {
        match command {
            Command::NewDocument(p) => self.check_replacement(p.guard, p.discard_changes)?,
            Command::OpenDocument(p) => self.check_replacement(p.guard, p.discard_changes)?,
            Command::CloseDocument(p) => self.check_replacement(p.guard, p.discard_changes)?,
            Command::SaveDocument(p) => self.check_guard(p.guard)?,
            Command::SaveAs(p) => self.check_guard(p.guard)?,
            Command::Export(p) => self.check_guard(p.guard)?,
            _ => {}
        }
        Ok(())
    }
    /// La préparation est entièrement terminée avant le remplacement de la session.
    /// Sert aussi aux fixtures et au service de récupération, jamais à désérialiser du JSON arbitraire.
    pub(crate) fn install_stored(
        &mut self,
        stored: StoredDocument,
        path: Option<PathBuf>,
        recovered: bool,
    ) -> Result<(), CommandError> {
        crate::storage::validate(&stored)
            .map_err(|e| CommandError::detail(ErrorCode::Document, "file_document", e))?;
        let document = Document::from_source(&stored.source)
            .map_err(|e| CommandError::detail(ErrorCode::Document, "file_document", e))?;
        let mut session = Session::new(stored, document, path, self.session.revision + 1);
        session.recovered = recovered;
        self.session = session;
        self.selection = Selection::default();
        self.presentation_cache = Default::default();
        *self.render.get_mut() = None;
        self.open = true;
        Ok(())
    }
    pub(super) fn new_document(&mut self, p: NewDocumentParams) -> Result<Value, CommandError> {
        let model = p
            .document_type
            .models()
            .find(|m| m.model.id() == p.model)
            .ok_or_else(|| CommandError::new(ErrorCode::Unavailable, "creation_model"))?;
        let mut stored = StoredDocument::blank(p.document_type);
        stored.template = model.source.into();
        stored.dependencies = crate::packages::for_template(model.source);
        if let Some((paper, landscape)) = model.page {
            stored.metadata.options_mut().paper = paper;
            stored.metadata.options_mut().landscape = landscape;
        }
        let mut draft: Engine =
            Session::new(stored, Document::from_source("").expect("empty"), None, 0).into();
        let mut fields = Vec::new();
        for initial in p.initial_values {
            if fields.contains(&initial.field) {
                return Err(CommandError::new(
                    ErrorCode::InvalidParams,
                    "duplicate_field",
                ));
            }
            fields.push(initial.field);
            draft.run(Command::SetMetadata(SetMetadataParams {
                guard: draft.target_guard(),
                field: initial.field,
                value: initial.value,
                input: MetadataInput::Discrete,
            }))?;
        }
        if p.signature.is_some() {
            draft.run(Command::SetSignature(SignatureParams {
                guard: draft.target_guard(),
                image: p.signature,
            }))?;
        }
        let stored = draft.stored_document();
        let mut world = crate::rendering::TypstWorld::default();
        world.set_dependencies(&stored.dependencies);
        world.set_assets(&stored.assets);
        let presentation = world
            .validate_model(&stored.source, &stored.metadata, &stored.template)
            .map_err(|e| CommandError::detail(ErrorCode::Template, "file_document", e))?;
        self.install_stored(stored, None, false)?;
        self.cache_presentation(presentation);
        Ok(json!({"open":true,"type":p.document_type}))
    }
    pub(super) fn open_document(&mut self, p: OpenDocumentParams) -> Result<Value, CommandError> {
        let stored = crate::storage::read(&p.path)
            .map_err(|e| CommandError::detail(ErrorCode::Io, "file_open", e))?;
        self.install_stored(stored, Some(p.path.clone()), false)?;
        Ok(json!({"path":p.path}))
    }
    pub(super) fn close_document(&mut self, _: CloseDocumentParams) -> Result<Value, CommandError> {
        let stored = StoredDocument::blank(DocumentType::Courrier);
        self.session = Session::new(
            stored,
            Document::from_source("").expect("empty"),
            None,
            self.session.revision + 1,
        );
        self.selection = Selection::default();
        self.presentation_cache = Default::default();
        *self.render.get_mut() = None;
        self.open = false;
        Ok(json!({"open":false}))
    }
    pub(super) fn save_document(&mut self, p: SaveDocumentParams) -> Result<Value, CommandError> {
        let path = p
            .path
            .or_else(|| self.session.file_path.clone())
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "save_path_required"))?;
        let overwrite = self.session.file_path.as_ref() == Some(&path);
        self.write_document(path, overwrite)
    }
    pub(super) fn save_as(&mut self, p: SaveAsParams) -> Result<Value, CommandError> {
        self.write_document(p.path, p.overwrite)
    }
    fn write_document(&mut self, path: PathBuf, overwrite: bool) -> Result<Value, CommandError> {
        if !overwrite
            && path
                .try_exists()
                .map_err(|e| CommandError::detail(ErrorCode::Io, "file_save", e))?
        {
            return Err(CommandError::new(ErrorCode::Io, "file_exists"));
        }
        let stored = self.session.stored_document();
        let result = if overwrite {
            crate::storage::save(&path, &stored)
        } else {
            crate::storage::save_new(&path, &stored)
        };
        result.map_err(|e| CommandError::detail(ErrorCode::Io, "file_save", e))?;
        // Le disque est nettoyé, mais ce repère conserve les ressources de l’historique.
        self.session.saved = stored;
        self.session.file_path = Some(path.clone());
        self.session.recovered = false;
        self.session.typing_group = false;
        Ok(json!({"path":path}))
    }
}
