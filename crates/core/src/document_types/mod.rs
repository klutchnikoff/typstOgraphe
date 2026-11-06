//! Types intégrés à l’application. Leurs modèles sont des implémentations Typst.
pub mod article;
pub mod authorship;
pub mod compte_rendu;
pub mod courrier;
pub mod definition;
pub mod discovery;
mod fields;
pub mod model_options;
pub mod models;
mod runtime;
pub use fields::{FieldValue, MetadataField};

use crate::document::DocumentOptions;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentType {
    Article,
    Courrier,
    CompteRendu,
}
impl DocumentType {
    pub const ALL: [Self; 3] = [Self::Courrier, Self::CompteRendu, Self::Article];
    pub fn label_key(self) -> &'static str {
        &self.definition().definition().label_key
    }
    pub fn definition(self) -> &'static definition::LoadedDefinition {
        &runtime::get(self).loaded
    }
    pub fn settings_heading_key(self) -> &'static str {
        self.definition()
            .definition()
            .settings_heading_key
            .as_deref()
            .unwrap_or_else(|| self.label_key())
    }
    pub fn composition_function(self) -> &'static str {
        &self.definition().definition().composition.function
    }
    pub fn file_stem(self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Courrier => "courrier",
            Self::CompteRendu => "compte-rendu",
        }
    }
    pub fn models(self) -> impl Iterator<Item = &'static models::Definition> + Clone {
        models::ALL.iter().filter(move |model| model.kind == self)
    }
    pub fn default_model(self) -> String {
        self.models()
            .next()
            .map_or(String::new(), |model| model.source.into())
    }
    pub fn default_model_manifest(self) -> &'static str {
        self.models().next().map_or(
            include_str!("../../templates/compte-rendu-internal.model.yaml"),
            |m| m.manifest,
        )
    }
    pub fn can_create(self) -> bool {
        self.models().next().is_some()
    }
    pub fn structured_metadata(self) -> Option<serde_json::Value> {
        runtime::get(self).structured.clone()
    }
}

/// Somme des métadonnées typées : ajouter un type exige de définir ses données
/// et de traiter explicitement ses opérations, sans carte de champs arbitraires.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "fields", rename_all = "kebab-case")]
pub enum Metadata {
    Article(article::Metadata),
    Courrier(courrier::Metadata),
    CompteRendu(compte_rendu::Metadata),
}
impl Metadata {
    pub fn demo(kind: DocumentType) -> Self {
        match kind {
            DocumentType::Article => Self::Article(article::Metadata::demo()),
            DocumentType::Courrier => Self::Courrier(courrier::Metadata::default()),
            DocumentType::CompteRendu => Self::CompteRendu(compte_rendu::Metadata::demo()),
        }
    }
    pub fn blank(kind: DocumentType) -> Self {
        serde_json::from_value(serde_json::json!({"type":kind,"fields":runtime::initial_fields(kind.definition(),&DocumentOptions::default())})).expect("declared defaults match domain metadata")
    }
    pub fn document_type(&self) -> DocumentType {
        match self {
            Self::Article(_) => DocumentType::Article,
            Self::Courrier(_) => DocumentType::Courrier,
            Self::CompteRendu(_) => DocumentType::CompteRendu,
        }
    }
    pub fn options(&self) -> &DocumentOptions {
        match self {
            Self::Article(meta) => &meta.options,
            Self::Courrier(meta) => &meta.options,
            Self::CompteRendu(meta) => &meta.options,
        }
    }
    /// All declared resource fields participate in validation, retention and exports.
    pub fn resource_paths(&self) -> Vec<String> {
        self.field_values()
            .into_iter()
            .filter(|f| f.kind == FieldKind::ResourceReference)
            .filter_map(|f| f.value.as_str().map(str::to_owned))
            .collect()
    }
    pub fn options_mut(&mut self) -> &mut DocumentOptions {
        match self {
            Self::Article(meta) => &mut meta.options,
            Self::Courrier(meta) => &mut meta.options,
            Self::CompteRendu(meta) => &mut meta.options,
        }
    }
    /// Source using the supplied default model; project callers resolve their embedded manifest.
    pub fn main_source(&self) -> String {
        model_options::load(self.document_type().default_model_manifest())
            .and_then(|m| m.resolve(self))
            .expect("default model options")
            .main_source()
    }
    pub(crate) fn raw_main_source(&self) -> String {
        let data = serde_json::to_value(self).expect("typed metadata");
        runtime::source(self.document_type(), &data["fields"], self.options())
    }
    pub fn editor_groups(&self) -> &'static [FieldGroup] {
        &runtime::get(self.document_type()).groups
    }
    pub fn field_values(&self) -> Vec<FieldValue> {
        let data = serde_json::to_value(self).expect("typed metadata");
        runtime::fields(self.document_type(), &data["fields"])
    }
    pub fn ready_for_export(&self) -> bool {
        let kind = self.document_type();
        let data = serde_json::to_value(self).expect("typed metadata");
        kind.definition()
            .export_diagnostics(
                &runtime::values(kind, &data["fields"]),
                self.options().language,
            )
            .is_empty()
    }
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Article(meta) = self {
            crate::bibliography::validate_sources(meta)?;
        }
        let kind = self.document_type();
        let data = serde_json::to_value(self).expect("typed metadata");
        kind.definition()
            .validate_values(&runtime::values(kind, &data["fields"]))
            .map_err(|issues| {
                format!(
                    "type_values: {}",
                    serde_json::to_string(&issues).expect("value diagnostics")
                )
            })
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn courrier(&self) -> &courrier::Metadata {
        let Self::Courrier(meta) = self else {
            panic!("métadonnées Courrier attendues")
        };
        meta
    }
    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn courrier_mut(&mut self) -> &mut courrier::Metadata {
        let Self::Courrier(meta) = self else {
            panic!("métadonnées Courrier attendues")
        };
        meta
    }
}

/// Presentation of one editor tab. Empty sections use the matching field group.
pub struct FieldGroup {
    pub id: String,
    pub label_key: String,
    pub sections: Vec<FieldSection>,
}
/// Layout of a metadata section, independent of its field identities.
pub use definition::SectionLayout as FieldSectionLayout;
/// Ordered fields of a section; values keep their existing identities.
pub struct FieldSection {
    pub layout: FieldSectionLayout,
    pub id: String,
    pub label_key: String,
    pub fields: Vec<MetadataField>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    #[serde(rename = "resource-reference")]
    ResourceReference,
    Records,
    Text,
    Date,
}
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::content::api::{Call, Session};
    use serde_json::json;
    #[test]
    fn every_type_validates_its_model_archives_exports_and_rejects_wrong_contracts() {
        let dir = tempfile::tempdir().unwrap();
        for kind in DocumentType::ALL
            .into_iter()
            .filter(|kind| kind.can_create())
        {
            let mut session = Session::new();
            session
                .execute(
                    "document.new",
                    json!({"type":kind,"metadata":Metadata::demo(kind)}),
                )
                .unwrap();
            session.batch(&[Call { id:"text.insert".into(), params:json!({"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Texte commun 日本 et caractères # [] * littéraux."}) }],None).unwrap();
            let expected = session.state().unwrap().clone();
            let mut world = crate::rendering::TypstWorld::default();
            let blank = Metadata::blank(kind);
            assert!(!blank.ready_for_export());
            world
                .validate_model("", &blank, &expected.settings.template)
                .unwrap();
            assert!(
                world
                    .validate_model(
                        "",
                        &blank,
                        &format!("#let {}(body) = body", kind.composition_function())
                    )
                    .is_err()
            );
            let archive = dir.path().join(format!("{}.typsto", kind.file_stem()));
            session.save_as(&archive).unwrap();
            session.execute("document.close", json!({})).unwrap();
            session
                .execute("document.open", json!({"path":archive}))
                .unwrap();
            assert_eq!(session.state().unwrap(), &expected);
            let export = dir.path().join(kind.file_stem());
            session
                .export(&export, crate::protocol::ExportFormat::Both, true)
                .unwrap();
            assert!(export.join(format!("{}.pdf", kind.file_stem())).exists());
            assert!(export.join("main.typ").exists());
        }
    }
}
