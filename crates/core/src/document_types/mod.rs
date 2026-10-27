//! Types intégrés à l’application. Leurs modèles sont des implémentations Typst.
pub mod article;
pub mod compte_rendu;
pub mod courrier;
mod fields;
pub mod models;
pub use fields::{FieldValue, MetadataField};

use crate::document::{DocumentOptions, typst_string};
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
        match self {
            Self::Article => "article-type",
            Self::Courrier => "letter-type",
            Self::CompteRendu => "minutes-type",
        }
    }
    pub fn composition_function(self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Courrier => courrier::CONTRACT.function,
            Self::CompteRendu => compte_rendu::CONTRACT.function,
        }
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
    pub fn can_create(self) -> bool {
        self.models().next().is_some()
    }
    pub fn structured_metadata(self) -> Option<serde_json::Value> {
        match self {
            Self::Article => Some(article::schema()),
            Self::Courrier | Self::CompteRendu => None,
        }
    }
    pub fn demo_source(self) -> &'static str {
        match self {
            Self::Article => include_str!("../../examples/article.typ"),
            Self::Courrier => include_str!("../../examples/courrier.typ"),
            Self::CompteRendu => include_str!("../../examples/compte-rendu.typ"),
        }
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
        match kind {
            DocumentType::Article => Self::Article(article::Metadata::default()),
            DocumentType::Courrier => Self::Courrier(courrier::Metadata::blank()),
            DocumentType::CompteRendu => Self::CompteRendu(compte_rendu::Metadata::default()),
        }
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
    pub fn signature_path(&self) -> Option<&str> {
        match self {
            Self::Courrier(meta) => meta.signature.as_deref(),
            Self::CompteRendu(_) | Self::Article(_) => None,
        }
    }
    pub fn options_mut(&mut self) -> &mut DocumentOptions {
        match self {
            Self::Article(meta) => &mut meta.options,
            Self::Courrier(meta) => &mut meta.options,
            Self::CompteRendu(meta) => &mut meta.options,
        }
    }
    pub fn main_source(&self) -> String {
        match self {
            Self::Article(meta) => meta.main_source(),
            Self::Courrier(meta) => courrier::CONTRACT.main_source(meta),
            Self::CompteRendu(meta) => compte_rendu::CONTRACT.main_source(meta),
        }
    }
    pub fn model_signature(&self) -> String {
        match self {
            Self::Article(_) => article::SIGNATURE.into(),
            Self::Courrier(_) => courrier::CONTRACT.signature(),
            Self::CompteRendu(_) => compte_rendu::CONTRACT.signature(),
        }
    }
    pub fn editor_groups(&self) -> &'static [FieldGroup] {
        match self {
            Self::Article(_) => article::GROUPS,
            Self::Courrier(_) => courrier::GROUPS,
            Self::CompteRendu(_) => compte_rendu::GROUPS,
        }
    }
    pub fn field_values(&self) -> Vec<FieldValue> {
        match self {
            Self::Article(meta) => meta.field_values(),
            Self::Courrier(meta) => fields::values(meta, courrier::FIELDS),
            Self::CompteRendu(meta) => fields::values(meta, compte_rendu::FIELDS),
        }
    }
    pub fn field_mut(&mut self, field: MetadataField) -> Option<&mut String> {
        match self {
            Self::Article(_) => None,
            Self::Courrier(meta) => fields::value_mut(meta, courrier::FIELDS, field),
            Self::CompteRendu(meta) => fields::value_mut(meta, compte_rendu::FIELDS, field),
        }
    }
    pub fn ready_for_export(&self) -> bool {
        match self {
            Self::Article(meta) => meta.ready_for_export(),
            Self::Courrier(meta) => courrier::FIELDS
                .iter()
                .all(|field| !field.required_for_export || !(field.get)(meta).trim().is_empty()),
            Self::CompteRendu(meta) => compte_rendu::FIELDS
                .iter()
                .all(|field| !field.required_for_export || !(field.get)(meta).trim().is_empty()),
        }
    }
    /// Missing editorial data is a draft warning, separate from structural validity.
    pub fn missing_export_fields(&self) -> Vec<MetadataField> {
        if let Self::Article(meta) = self {
            let mut fields = Vec::new();
            if !meta
                .localizations
                .iter()
                .any(|l| l.language == meta.options.language && !l.title.is_empty())
            {
                fields.push(MetadataField::ArticleLocalizations);
            }
            if meta.authors.iter().any(|a| a.name.trim().is_empty()) {
                fields.push(MetadataField::ArticleAuthors);
            }
            if meta.affiliations.iter().any(|a| a.text.trim().is_empty()) {
                fields.push(MetadataField::ArticleAffiliations);
            }
            return fields;
        }
        self.field_values()
            .into_iter()
            .filter(|f| {
                f.required_for_export && f.value.as_str().is_none_or(|s| s.trim().is_empty())
            })
            .map(|f| f.field)
            .collect()
    }
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Article(meta) = self {
            meta.validate()?;
        }
        Ok(())
    }
    pub fn ensure_primary(&mut self) {
        if let Self::Article(meta) = self {
            meta.ensure_primary();
        }
    }
    pub fn set_field_value(
        &mut self,
        field: MetadataField,
        value: serde_json::Value,
    ) -> Result<(), String> {
        if let Self::Article(meta) = self {
            return meta.set_field(field, value);
        }
        *self.field_mut(field).ok_or("Champ absent du type.")? =
            value.as_str().ok_or("Valeur texte attendue.")?.into();
        Ok(())
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
    pub id: &'static str,
    pub label_key: &'static str,
    pub sections: &'static [FieldSection],
}
/// Layout of a metadata section, independent of its field identities.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldSectionLayout {
    Editorial,
    Compact,
    Inline,
}
/// Ordered fields of a section; values keep their existing identities.
pub struct FieldSection {
    pub layout: FieldSectionLayout,
    pub id: &'static str,
    pub label_key: &'static str,
    pub fields: &'static [MetadataField],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Records,
    Text,
    Date,
}
pub struct TextField<T: 'static> {
    pub field: MetadataField,
    pub parameter: &'static str,
    pub editor_id: &'static str,
    pub label_key: &'static str,
    pub group: &'static str,
    pub multiline: bool,
    pub kind: FieldKind,
    pub required_for_export: bool,
    pub get: fn(&T) -> &str,
    pub get_mut: fn(&mut T) -> &mut String,
}

/// Contrat de la fonction de composition : champs nommés, options communes,
/// contenu positionnel final. L’implémentation de cette fonction reste en Typst.
pub struct ModelContract<T: 'static> {
    pub function: &'static str,
    pub fields: &'static [TextField<T>],
    pub options: fn(&T) -> &DocumentOptions,
    pub signature_image: Option<fn(&T) -> Option<&str>>,
}

/// Libertés explicitement accordées par le créateur du modèle, en pur Typst.
/// Par défaut la charte reste entièrement automatique.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPresentation {
    pub bullet_markers: Vec<String>,
    pub custom_bullet_marker: bool,
}
impl ModelPresentation {
    pub fn allows_bullet_marker(&self, marker: &str) -> bool {
        crate::document::valid_bullet_marker(marker)
            && (self.custom_bullet_marker || self.bullet_markers.iter().any(|s| s == marker))
    }
}
impl<T> ModelContract<T> {
    pub fn signature(&self) -> String {
        let mut parameters: Vec<_> = self.fields.iter().map(|field| field.parameter).collect();
        if self.signature_image.is_some() {
            parameters.push("signature");
        }
        parameters.extend(["options", "body"]);
        format!("{}({})", self.function, parameters.join(", "))
    }
    pub fn main_source(&self, metadata: &T) -> String {
        let name = self.function;
        let mut source = format!("#import \"modele.typ\": {name}\n#show: {name}.with(\n");
        for field in self.fields {
            source.push_str(&format!(
                "  {}: {},\n",
                field.parameter,
                typst_string((field.get)(metadata))
            ));
        }
        if let Some(signature_image) = self.signature_image {
            source.push_str(&format!(
                "  signature: {},\n",
                signature_image(metadata).map_or_else(|| "none".into(), typst_string)
            ));
        }
        let options = (self.options)(metadata);
        source.push_str(&format!(
            "  options: (paper: {}, landscape: {}, language: {}, bullet-marker: {}),\n)\n#include \"contenu.typ\"\n",
            typst_string(options.paper.typst_name()),
            options.landscape,
            typst_string(options.language.code()),
            options.bullet_marker.as_deref().map_or_else(|| "auto".into(), typst_string),
        ));
        source
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{
        document::Document,
        rendering::{RenderRequest, TypstWorld},
        storage::{self, StoredDocument},
    };
    #[test]
    fn every_type_validates_its_model_archives_exports_and_rejects_wrong_contracts() {
        let dir = tempfile::tempdir().unwrap();
        for kind in DocumentType::ALL
            .into_iter()
            .filter(|kind| kind.can_create())
        {
            let mut letter = StoredDocument::demo(kind);
            letter.source.push_str("\n\n#quote(block: true, attribution: [Service])[Une citation.]\n\n/ Échéance: Vendredi\n\n#figure(image(\"images/parc.svg\"), caption: [Un parc.])");
            letter.assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../../../../fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
            let mut world = TypstWorld::default();
            world.set_assets(&letter.assets);
            world
                .validate_model(&letter.source, &letter.metadata, &letter.template)
                .unwrap();
            let blank = Metadata::blank(kind);
            assert!(!blank.ready_for_export());
            world.validate_model("", &blank, &letter.template).unwrap();
            let other = if kind == DocumentType::Courrier {
                DocumentType::CompteRendu
            } else {
                DocumentType::Courrier
            };
            assert!(
                world
                    .validate_model(
                        &letter.source,
                        &letter.metadata,
                        &format!("#let {}(..args) = []", other.composition_function())
                    )
                    .unwrap_err()
                    .to_string()
                    .contains(kind.composition_function())
            );
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
            storage::save(&archive, &letter).unwrap();
            let restored = storage::read(&archive).unwrap();
            assert_eq!(restored, letter);
            let document = Document::from_source(&restored.source).unwrap();
            let export = dir.path().join(kind.file_stem());
            crate::rendering::render(
                &mut world,
                &RenderRequest {
                    revision: 1,
                    source: restored.source,
                    source_map: document.source_map,
                    metadata: restored.metadata,
                    template: restored.template,
                    assets: restored.assets,
                    dependencies: restored.dependencies.clone(),
                    export_dir: Some(export.clone()),
                },
            )
            .unwrap();
            assert!(export.join(format!("{}.pdf", kind.file_stem())).exists());
        }
    }
}
