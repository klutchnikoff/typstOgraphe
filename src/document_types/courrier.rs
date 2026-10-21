//! Le type Courrier : données, contrat du modèle et commandes propres au type.
use super::{FieldGroup, ModelContract, TextField};
use crate::{
    commands::{self, Definition, Scope},
    document::DocumentOptions,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    pub options: DocumentOptions,
    pub expediteur: String,
    pub fonction: String,
    pub service: String,
    pub email: String,
    pub organisme: String,
    pub adresse_expediteur: String,
    pub telephone: String,
    pub signature: Option<String>,
    pub destinataire: String,
    pub adresse: String,
    pub objet: String,
    pub reference: String,
    pub lieu: String,
    pub date: String,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            options: DocumentOptions::default(),
            expediteur: "Camille Martin".into(),
            fonction: "Responsable de la vie associative".into(),
            service: "Vie associative".into(),
            email: "camille.martin@valmont.example".into(),
            organisme: "Ville de Valmont".into(),
            adresse_expediteur: "1, place de la Mairie\n35000 Valmont".into(),
            telephone: String::new(),
            signature: None,
            destinataire: "Service des équipements".into(),
            adresse: "12, place de la République\n35000 Valmont".into(),
            objet: "Demande de réservation d’une salle".into(),
            reference: "VA / 2026 / 042".into(),
            lieu: "Valmont".into(),
            date: "19 octobre 2026".into(),
        }
    }
}

impl Metadata {
    pub fn blank() -> Self {
        Self {
            options: DocumentOptions::default(),
            expediteur: String::new(),
            fonction: String::new(),
            service: String::new(),
            email: String::new(),
            organisme: String::new(),
            adresse_expediteur: String::new(),
            telephone: String::new(),
            signature: None,
            destinataire: String::new(),
            adresse: String::new(),
            objet: String::new(),
            reference: String::new(),
            date: String::new(),
            lieu: String::new(),
        }
    }
}

macro_rules! field {
    ($name:ident, $editor:literal, $label:literal, $group:literal, $multiline:literal, $required:literal) => {
        TextField {
            parameter: stringify!($name),
            editor_id: $editor,
            label_key: $label,
            group: $group,
            multiline: $multiline,
            required_for_export: $required,
            get: |m: &Metadata| &m.$name,
            get_mut: |m: &mut Metadata| &mut m.$name,
        }
    };
}
pub const FIELDS: &[TextField<Metadata>] = &[
    field!(
        expediteur,
        "sender",
        "field-full-name",
        "sender",
        false,
        false
    ),
    field!(fonction, "role", "field-role", "sender", false, false),
    field!(service, "service", "field-service", "sender", false, false),
    field!(email, "email", "field-email", "sender", false, false),
    field!(
        organisme,
        "organisation",
        "field-organisation",
        "sender",
        false,
        false
    ),
    field!(
        adresse_expediteur,
        "sender-address",
        "field-sender-address",
        "sender",
        true,
        false
    ),
    field!(
        telephone,
        "sender-phone",
        "field-phone",
        "sender",
        false,
        false
    ),
    field!(
        destinataire,
        "recipient",
        "field-recipient",
        "correspondence",
        false,
        true
    ),
    field!(
        adresse,
        "address",
        "field-address",
        "correspondence",
        true,
        false
    ),
    field!(
        objet,
        "subject",
        "field-subject",
        "correspondence",
        false,
        true
    ),
    field!(
        reference,
        "reference",
        "field-reference",
        "correspondence",
        false,
        false
    ),
    field!(lieu, "place", "field-place", "correspondence", false, false),
    field!(date, "date", "field-date", "correspondence", false, false),
];
pub const GROUPS: &[FieldGroup] = &[
    FieldGroup {
        id: "correspondence",
        label_key: "group-correspondence",
    },
    FieldGroup {
        id: "sender",
        label_key: "group-sender",
    },
];
pub const CONTRACT: ModelContract<Metadata> = ModelContract {
    function: "courrier",
    fields: FIELDS,
    options: |m| &m.options,
    signature_image: Some(|m| m.signature.as_deref()),
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Institutional,
    Service,
}
impl Model {
    #[cfg(test)]
    pub const ALL: [Self; 2] = [Self::Institutional, Self::Service];
    pub fn source(self) -> String {
        let base = include_str!("../../templates/courrier.typ");
        match self {
            Self::Institutional => base.into(),
            Self::Service => format!(
                "{base}\n{}",
                include_str!("../../templates/courrier-service.typ")
            ),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Model(Model),
}
impl Command {
    pub fn definition(self) -> &'static Definition {
        COMMANDS
            .iter()
            .find(|d| d.command == commands::Command::Courrier(self))
            .expect("commande Courrier enregistrée")
    }
}
pub const COMMANDS: &[Definition] = &[
    commands::short(
        commands::define(
            "courrier.model.institutional",
            "cmd-institutional",
            commands::Command::Courrier(Command::Model(Model::Institutional)),
            Scope::Global,
            &[],
        ),
        "cmd-institutional-short",
    ),
    commands::short(
        commands::define(
            "courrier.model.service",
            "cmd-service",
            commands::Command::Courrier(Command::Model(Model::Service)),
            Scope::Global,
            &[],
        ),
        "cmd-service-short",
    ),
];

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::document_types::{DocumentType, Metadata as DocumentMetadata};
    use crate::rendering::TypstWorld;

    #[test]
    fn personal_window_letter_compiles_in_app_and_keeps_addresses_on_first_page() {
        use typst::layout::{Frame, FrameItem};
        fn text(frame: &Frame) -> String {
            let mut result = String::new();
            for (_, item) in frame.items() {
                match item {
                    FrameItem::Group(group) => result.push_str(&text(&group.frame)),
                    FrameItem::Text(run) => result.push_str(&run.text),
                    _ => {}
                }
            }
            result
        }
        let template = include_str!("../../templates/courrier-personnel.typ");
        let mut world = TypstWorld::default();
        let mut metadata = DocumentMetadata::blank(DocumentType::Courrier);
        world
            .validate_model("Bonjour.", &metadata, template)
            .unwrap();
        metadata.courrier_mut().expediteur = "EXPEDITEURTEST".into();
        metadata.courrier_mut().destinataire = "DESTINATAIRETEST".into();
        metadata.courrier_mut().adresse =
            "Maison des associations\n4 avenue des Tilleuls\n35000 RENNES".into();
        let body = "Un paragraphe de courrier assez long pour éprouver le passage sur la page suivante et la conservation de la mise en page.\n\n".repeat(45);
        let pages = world
            .compile_with_template(&body, &metadata, template)
            .unwrap();
        assert!(pages.pages().len() > 1);
        assert!(text(&pages.pages()[0].frame).contains("DESTINATAIRETEST"));
        for page in &pages.pages()[1..] {
            assert!(!text(&page.frame).contains("DESTINATAIRETEST"));
        }
        // Une adresse excessive est refusée, jamais tronquée dans la fenêtre.
        metadata.courrier_mut().adresse = "Une ligne d'adresse\n".repeat(12);
        assert!(
            world
                .compile_with_template("Bonjour.", &metadata, template)
                .is_err()
        );
        metadata.courrier_mut().adresse = "W".repeat(120);
        assert!(
            world
                .compile_with_template("Bonjour.", &metadata, template)
                .is_err()
        );
        metadata.courrier_mut().adresse.clear();
        metadata.options_mut().landscape = true;
        assert!(
            world
                .compile_with_template("Bonjour.", &metadata, template)
                .is_err()
        );
    }

    #[test]
    fn courrier_contract_accepts_independent_typst_models_without_presentation_flags() {
        let mut world = TypstWorld::default();
        let metadata = DocumentMetadata::demo(DocumentType::Courrier);
        let header = metadata.main_source();
        assert!(header.contains("#show: courrier.with("));
        assert!(!header.contains("compact"));
        for model in Model::ALL {
            assert!(
                world
                    .compile_with_template("Bonjour.", &metadata, &model.source())
                    .is_ok()
            );
        }
        // Une implémentation différente peut employer des fonctions privées
        // élaborées sans aucune déclaration supplémentaire dans l’application.
        let arguments = FIELDS
            .iter()
            .map(|f| format!("{}: \"\"", f.parameter))
            .collect::<Vec<_>>()
            .join(", ");
        let custom = format!(
            r#"
#let decoration-interne(body) = block(width: 100%, inset: 8pt, stroke: 0.5pt)[#body]
#let courrier({arguments}, signature: none, options: (paper: "a4", landscape: false, language: "fr"), body) = {{
  set page(paper: options.paper, flipped: options.landscape)
  set text(lang: options.language)
  decoration-interne[#text(weight: "bold")[#objet] #body]
}}
"#
        );
        let document = world
            .compile_with_template("Bonjour.", &metadata, &custom)
            .unwrap();
        assert!(!document.pages().is_empty());
        assert!(
            world
                .compile_with_template("Bonjour.", &metadata, "#let courrier(body) = body")
                .is_err()
        );
        assert!(
            world
                .compile_with_template("Bonjour.", &metadata, "#let autre(body) = body")
                .is_err()
        );
    }

    #[test]
    fn courrier_field_definitions_drive_header_and_required_export_fields() {
        let mut metadata = DocumentMetadata::blank(DocumentType::Courrier);
        assert_eq!(metadata.document_type(), DocumentType::Courrier);
        assert!(metadata.courrier().destinataire.is_empty());
        assert!(!metadata.ready_for_export());
        metadata.courrier_mut().destinataire = "Destinataire".into();
        assert!(!metadata.ready_for_export());
        metadata.courrier_mut().objet = "Objet".into();
        assert!(metadata.ready_for_export());
        let mut parameters = std::collections::HashSet::new();
        let mut editor_ids = std::collections::HashSet::new();
        for field in FIELDS {
            assert!(parameters.insert(field.parameter));
            assert!(editor_ids.insert(field.editor_id));
            assert!(GROUPS.iter().any(|group| group.id == field.group));
            let DocumentMetadata::Courrier(fields) = &mut metadata else {
                unreachable!()
            };
            (field.get_mut)(fields).push_str(" é\\#");
            let value = (field.get)(fields);
            assert!(CONTRACT.main_source(fields).contains(&format!(
                "  {}: {},",
                field.parameter,
                crate::document::typst_string(value)
            )));
        }
        assert_eq!(
            serde_json::from_str::<DocumentMetadata>(&serde_json::to_string(&metadata).unwrap())
                .unwrap(),
            metadata
        );
    }
}
