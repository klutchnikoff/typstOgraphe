//! Types intégrés à l’application. Leurs modèles sont des implémentations Typst.
pub mod courrier;

use crate::{
    commands::Definition,
    document::{DocumentOptions, typst_string},
};
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentType {
    Courrier,
}
impl DocumentType {
    #[cfg(test)]
    pub const ALL: [Self; 1] = [Self::Courrier];
    pub fn label(self) -> String {
        match self {
            Self::Courrier => crate::tr!("letter-type"),
        }
    }
    pub fn commands(self) -> &'static [Definition] {
        match self {
            Self::Courrier => courrier::COMMANDS,
        }
    }
    pub fn default_model(self) -> String {
        match self {
            Self::Courrier => courrier::Model::Institutional.source(),
        }
    }
    pub fn demo_source(self) -> &'static str {
        match self {
            Self::Courrier => include_str!("../../examples/courrier.typ"),
        }
    }
}

/// Somme des métadonnées typées : ajouter un type exige de définir ses données
/// et de traiter explicitement ses opérations, sans carte de champs arbitraires.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "fields", rename_all = "kebab-case")]
pub enum Metadata {
    Courrier(courrier::Metadata),
}
impl Metadata {
    pub fn demo(kind: DocumentType) -> Self {
        match kind {
            DocumentType::Courrier => Self::Courrier(courrier::Metadata::default()),
        }
    }
    pub fn blank(kind: DocumentType) -> Self {
        match kind {
            DocumentType::Courrier => Self::Courrier(courrier::Metadata::blank()),
        }
    }
    pub fn document_type(&self) -> DocumentType {
        match self {
            Self::Courrier(_) => DocumentType::Courrier,
        }
    }
    pub fn options(&self) -> &DocumentOptions {
        match self {
            Self::Courrier(meta) => &meta.options,
        }
    }
    pub fn options_mut(&mut self) -> &mut DocumentOptions {
        match self {
            Self::Courrier(meta) => &mut meta.options,
        }
    }
    pub fn main_source(&self) -> String {
        match self {
            Self::Courrier(meta) => courrier::CONTRACT.main_source(meta),
        }
    }
    pub fn editor_ui(&mut self, ui: &mut egui::Ui) {
        match self {
            Self::Courrier(meta) => fields_ui(ui, meta, courrier::FIELDS, courrier::GROUPS),
        }
    }
    pub fn ready_for_export(&self) -> bool {
        match self {
            Self::Courrier(meta) => courrier::FIELDS
                .iter()
                .all(|field| !field.required_for_export || !(field.get)(meta).trim().is_empty()),
        }
    }
    #[cfg(test)]
    pub fn courrier(&self) -> &courrier::Metadata {
        let Self::Courrier(meta) = self;
        meta
    }
    #[cfg(test)]
    pub fn courrier_mut(&mut self) -> &mut courrier::Metadata {
        let Self::Courrier(meta) = self;
        meta
    }
}

pub struct FieldGroup {
    pub id: &'static str,
    pub label_key: &'static str,
    pub default_open: bool,
}
pub struct TextField<T: 'static> {
    pub parameter: &'static str,
    pub editor_id: &'static str,
    pub label_key: &'static str,
    pub group: &'static str,
    pub multiline: bool,
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
}
impl<T> ModelContract<T> {
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
        let options = (self.options)(metadata);
        source.push_str(&format!(
            "  options: (paper: {}, landscape: {}, language: {}),\n)\n#include \"contenu.typ\"\n",
            typst_string(options.paper.typst_name()),
            options.landscape,
            typst_string(options.language.code()),
        ));
        source
    }
}

fn fields_ui<T>(
    ui: &mut egui::Ui,
    metadata: &mut T,
    fields: &[TextField<T>],
    groups: &[FieldGroup],
) {
    let width = (ui.available_width() - 120.0).max(80.0);
    for group in groups {
        egui::CollapsingHeader::new(crate::i18n::text(group.label_key))
            .id_salt(format!("information-{}", group.id))
            .default_open(group.default_open)
            .show(ui, |ui| {
                egui::Grid::new(format!("{}-fields", group.id))
                    .num_columns(2)
                    .show(ui, |ui| {
                        for field in fields.iter().filter(|f| f.group == group.id) {
                            let label = ui.label(crate::i18n::text(field.label_key));
                            let value = (field.get_mut)(metadata);
                            let widget = if field.multiline {
                                egui::TextEdit::multiline(value).desired_rows(3)
                            } else {
                                egui::TextEdit::singleline(value)
                            };
                            ui.add(widget.id_salt(field.editor_id).desired_width(width))
                                .labelled_by(label.id);
                            ui.end_row();
                        }
                    });
            });
    }
}
