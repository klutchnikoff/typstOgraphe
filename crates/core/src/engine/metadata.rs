//! Champs du contrat actif et options contrôlées par le modèle, puis commit commun.
use super::*;
use crate::{
    document::PaperFormat,
    document_types::{FieldKind, ModelPresentation},
};
use serde_json::json;
use std::cell::RefCell;

#[derive(Default)]
pub(super) struct PresentationCache(RefCell<Option<(TargetGuard, ModelPresentation)>>);

impl Engine {
    fn metadata_value(&self, params: &SetMetadataParams) -> Result<Value, CommandError> {
        self.check_guard(params.guard)?;
        let field = self
            .session
            .metadata
            .field_values()
            .into_iter()
            .find(|f| f.field == params.field)
            .ok_or_else(|| CommandError::new(ErrorCode::Unavailable, "metadata_field"))?;
        if field.kind == FieldKind::Records {
            let mut next = self.session.metadata.clone();
            next.set_field_value(params.field, params.value.clone())
                .map_err(|_| CommandError::new(ErrorCode::InvalidParams, "invalid_text"))?;
            return Ok(params.value.clone());
        }
        let text = params
            .value
            .as_str()
            .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "invalid_text"))?;
        if text.chars().any(|c| {
            (c.is_control() && !(field.multiline && matches!(c, '\n' | '\r' | '\t')))
                || matches!(c, crate::document::IMAGE_OBJECT | '\u{2028}' | '\u{2029}')
        }) {
            return Err(CommandError::new(ErrorCode::InvalidParams, "invalid_text"));
        }
        let mut value = text.replace("\r\n", "\n").replace('\r', "\n");
        if field.kind == FieldKind::Date {
            if crate::dates::is_today(&value) {
                return Err(CommandError::new(
                    ErrorCode::InvalidParams,
                    "metadata_fixed_date",
                ));
            }
            if params.input == MetadataInput::Discrete || value.trim().len() == 10 {
                crate::dates::normalize_explicit(&mut value);
            }
        }
        Ok(Value::String(value))
    }
    pub fn fixed_page(&self) -> Option<(PaperFormat, bool)> {
        crate::document_types::models::ALL
            .iter()
            .find(|m| {
                m.kind == self.session.metadata.document_type() && m.source == self.session.template
            })
            .and_then(|m| m.page)
    }
    pub(super) fn option_policy(&self) -> Result<ModelPresentation, CommandError> {
        let guard = self.session.target_guard();
        let mut cached = self.presentation_cache.0.borrow_mut();
        if let Some((previous, policy)) = cached.as_ref()
            && *previous == guard
        {
            return Ok(policy.clone());
        }
        let mut world = crate::rendering::TypstWorld::default();
        world.set_dependencies(&self.session.dependencies);
        world.set_assets(&self.session.document.assets);
        let policy = world
            .model_presentation(&self.session.template)
            .map_err(|e| CommandError::with_cause(ErrorCode::Template, "model_presentation", e))?;
        *cached = Some((guard, policy.clone()));
        Ok(policy)
    }
    pub(super) fn cache_presentation(&self, policy: ModelPresentation) {
        *self.presentation_cache.0.borrow_mut() = Some((self.session.target_guard(), policy));
    }
    pub(super) fn presentation_query(&self) -> Result<Value, CommandError> {
        let policy = self.option_policy()?;
        let page = self
            .fixed_page()
            .map(|(paper, landscape)| json!({"paper":paper,"landscape":landscape}));
        Ok(
            json!({"guard":self.session.target_guard(), "fixed_page":page,
            "bullet_markers":policy.bullet_markers,"custom_bullet_marker":policy.custom_bullet_marker}),
        )
    }
    pub(super) fn check_option(&self, params: &SetOptionParams) -> Result<(), CommandError> {
        self.check_guard(params.guard)?;
        let allowed = match &params.option {
            DocumentOption::Paper(paper) => self.fixed_page().is_none_or(|(p, _)| p == *paper),
            DocumentOption::Landscape(landscape) => {
                self.fixed_page().is_none_or(|(_, l)| l == *landscape)
            }
            DocumentOption::Language(_) | DocumentOption::BulletMarker(None) => true,
            DocumentOption::BulletMarker(Some(marker)) => {
                if !crate::document::valid_bullet_marker(marker) {
                    return Err(CommandError::new(
                        ErrorCode::InvalidParams,
                        "bullet_marker_invalid",
                    ));
                }
                if !self.option_policy()?.allows_bullet_marker(marker) {
                    return Err(CommandError::new(
                        ErrorCode::Unavailable,
                        "bullet_marker_policy",
                    ));
                }
                true
            }
        };
        if !allowed {
            return Err(CommandError::new(
                ErrorCode::Unavailable,
                "model_fixed_page",
            ));
        }
        Ok(())
    }
    pub(super) fn check_metadata(&self, command: &Command) -> Result<(), CommandError> {
        match command {
            Command::SetMetadata(p) => {
                self.metadata_value(p)?;
            }
            Command::SetOption(p) => self.check_option(p)?,
            _ => {}
        }
        Ok(())
    }
    pub(super) fn set_metadata(&mut self, params: SetMetadataParams) -> Result<(), CommandError> {
        let value = self.metadata_value(&params)?;
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        candidate
            .metadata
            .set_field_value(params.field, value)
            .map_err(|_| CommandError::new(ErrorCode::InvalidParams, "invalid_text"))?;
        let group = if params.input == MetadataInput::Typing {
            EditGroup::Metadata(params.field)
        } else {
            EditGroup::Discrete
        };
        self.selection =
            self.session
                .commit_snapshot(params.guard, before, candidate, self.selection, group)?;
        Ok(())
    }
    pub(super) fn set_option(&mut self, params: SetOptionParams) -> Result<(), CommandError> {
        let before = self.session.snapshot(self.selection);
        let mut candidate = before.clone();
        let options = candidate.metadata.options_mut();
        match params.option {
            DocumentOption::Paper(value) => options.paper = value,
            DocumentOption::Landscape(value) => options.landscape = value,
            DocumentOption::Language(value) => options.language = value,
            DocumentOption::BulletMarker(value) => options.bullet_marker = value,
        }
        candidate.metadata.ensure_primary();
        self.selection = self.session.commit_snapshot(
            params.guard,
            before,
            candidate,
            self.selection,
            EditGroup::Discrete,
        )?;
        Ok(())
    }
}
