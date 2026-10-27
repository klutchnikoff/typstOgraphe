use serde::{Deserialize, Serialize};
use typstographe_core::document_types::{DocumentType, Metadata};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub metadata: Metadata,
    pub template: String,
    pub model_id: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            metadata: Metadata::demo(DocumentType::Courrier),
            template: DocumentType::Courrier.default_model(),
            model_id: "courrier.model.personal".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.template.len() > 2 * 1024 * 1024 {
            return Err("Modèle trop volumineux.".into());
        }
        // An embedded model belongs to the project, not necessarily to the catalogue.
        // Its Typst contract is checked by Renderer::validate before publication.
        if self.model_id == "embedded" {
            return Ok(());
        }
        let kind = self.metadata.document_type();
        let model = kind
            .models()
            .find(|m| m.model.id() == self.model_id)
            .ok_or("Modèle absent du type de document.")?;
        if model.source != self.template {
            return Err("Le modèle ne correspond pas au catalogue.".into());
        }
        if let Some((paper, landscape)) = model.page
            && (self.metadata.options().paper != paper
                || self.metadata.options().landscape != landscape)
        {
            return Err("Le modèle impose le format de page.".into());
        }
        Ok(())
    }
}

#[tauri::command]
pub async fn validate_model(
    state: tauri::State<'_, crate::Bridge>,
    snapshot: crate::session::Snapshot,
) -> Result<(), String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let s = session.lock().map_err(|e| e.to_string())?;
        let source = s.validate(&snapshot)?;
        renderer.validate(
            &source,
            &s.assets,
            snapshot.settings.as_ref().unwrap_or(&s.settings),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn embedded_models_need_not_be_installed_but_must_compile_against_the_type() {
        let settings = Settings {
            model_id: "embedded".into(),
            template: format!("{}\n// Modèle personnel", Settings::default().template),
            ..Default::default()
        };
        settings.validate().unwrap();
        let renderer = crate::rendering::Renderer::default();
        let assets = Default::default();
        renderer
            .validate("Texte du courrier", &assets, &settings)
            .unwrap();
        let invalid = Settings {
            template: "#let autre(body) = body".into(),
            ..settings.clone()
        };
        assert!(renderer.validate("Texte", &assets, &invalid).is_err());
        let not_embedded = Settings {
            model_id: Settings::default().model_id,
            ..settings
        };
        assert!(not_embedded.validate().is_err());
    }
}
