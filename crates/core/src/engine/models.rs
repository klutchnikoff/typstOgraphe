//! Modèles préparés, compilés et publiés avec leurs dépendances en une transaction.
use super::*;
use crate::document_types::{
    DocumentType, ModelPresentation,
    models::{ALL, Model},
};
use serde_json::json;
use std::io::Read;

pub const MAX_MODEL_BYTES: usize = 1024 * 1024;

impl ModelInput {
    fn read(&self) -> Result<String, CommandError> {
        let source = match self {
            Self::Source { source } => source.clone(),
            Self::Path { path } => {
                let file = std::fs::File::open(path)
                    .map_err(|e| CommandError::detail(ErrorCode::Io, "model_io", e))?;
                let mut bytes = Vec::new();
                file.take(MAX_MODEL_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| CommandError::detail(ErrorCode::Io, "model_io", e))?;
                if bytes.len() > MAX_MODEL_BYTES {
                    return Err(CommandError::new(
                        ErrorCode::InvalidParams,
                        "model_too_large",
                    ));
                }
                String::from_utf8(bytes)
                    .map_err(|e| CommandError::detail(ErrorCode::InvalidParams, "model_utf8", e))?
            }
        };
        if source.len() > MAX_MODEL_BYTES {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "model_too_large",
            ));
        }
        Ok(source)
    }
}

impl Engine {
    pub(super) fn check_model(&self, command: &Command) -> Result<(), CommandError> {
        match command {
            Command::SetModel(p) => {
                self.check_guard(p.guard)?;
                if let ModelInput::Source { source } = &p.model
                    && source.len() > MAX_MODEL_BYTES
                {
                    return Err(CommandError::new(
                        ErrorCode::InvalidParams,
                        "model_too_large",
                    ));
                }
            }
            Command::PersonalModel(p) => {
                self.check_guard(p.guard)?;
                if self.session.metadata.document_type() != DocumentType::Courrier {
                    return Err(CommandError::new(ErrorCode::Unavailable, "document_type"));
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn models_query(&self) -> Value {
        let kind = self.session.metadata.document_type();
        let models: Vec<_> = kind.models().map(|m| json!({
            "id":m.model.id(),"document_type":m.kind,"active":m.source == self.session.template,
            "name":m.name,"description":m.description,"keywords":m.keywords,
            "fixed_page":m.page.map(|(paper,landscape)| json!({"paper":paper,"landscape":landscape})),
        })).collect();
        json!({"guard":self.session.target_guard(),"document_type":kind,"models":models})
    }
    pub(super) fn personal_model(&mut self, params: GuardParams) -> Result<Value, CommandError> {
        self.set_model(SetModelParams {
            guard: params.guard,
            model: ModelInput::Source {
                source: Model::Personal.source(),
            },
        })
    }
    pub(super) fn set_model(&mut self, params: SetModelParams) -> Result<Value, CommandError> {
        // La garde est contrôlée par check_model avant toute lecture de fichier.
        let template = params.model.read()?;
        let builtin = ALL
            .iter()
            .find(|m| m.kind == self.session.metadata.document_type() && m.source == template);
        let dependencies = if builtin.is_some() {
            crate::packages::for_template(&template)
        } else {
            crate::packages::with_existing(&template, &self.session.dependencies)
        };
        crate::packages::validate(&template, &dependencies)
            .map_err(|e| CommandError::with_cause(ErrorCode::Template, "model_dependencies", e))?;
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        if template != before.template || dependencies != before.dependencies {
            candidate.metadata.options_mut().bullet_marker = None;
            if let Some((paper, landscape)) = builtin.and_then(|m| m.page) {
                candidate.metadata.options_mut().paper = paper;
                candidate.metadata.options_mut().landscape = landscape;
            }
        }
        candidate.template = template;
        candidate.dependencies = dependencies;
        let mut world = crate::rendering::TypstWorld::default();
        world.set_dependencies(&candidate.dependencies);
        world.set_assets(&candidate.document.assets);
        let presentation: ModelPresentation = world.validate_model(candidate.document.source(),&candidate.metadata,&candidate.template)
            .map_err(|e| CommandError { code:ErrorCode::Template,
                params:json!({"reason":"model_validation","detail":e.to_string(),"cause":e,"signature":candidate.metadata.model_signature()}) })?;
        super::resources::check_budget(&candidate)?;
        self.selection = self.session.commit_snapshot(
            params.guard,
            before,
            candidate,
            self.selection,
            EditGroup::Discrete,
        )?;
        self.cache_presentation(presentation.clone());
        Ok(json!({"presentation":presentation}))
    }
}
