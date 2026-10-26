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
