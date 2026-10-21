//! Compte-rendu de réunion : contrat intégré, présentation en pur Typst.
use super::{FieldGroup, ModelContract, TextField};
use crate::document::DocumentOptions;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub options: DocumentOptions,
    pub titre: String,
    pub date: String,
    pub lieu: String,
    pub redacteur: String,
    pub participants: String,
    pub excuses: String,
}
impl Metadata {
    pub fn demo() -> Self {
        Self {
            titre: "Préparation de la rencontre associative".into(),
            date: "21 octobre 2026".into(),
            lieu: "Salle des Tilleuls".into(),
            redacteur: "Camille Martin".into(),
            participants: "Camille Martin\nAlex Dupont".into(),
            excuses: "Lou Bernard".into(),
            ..Self::default()
        }
    }
}
macro_rules! field {
    ($name:ident, $label:literal, $group:literal, $multiline:literal, $required:literal) => {
        TextField {
            parameter: stringify!($name),
            editor_id: concat!("minutes-", stringify!($name)),
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
    field!(titre, "field-meeting-title", "meeting", false, true),
    field!(date, "field-date", "meeting", false, false),
    field!(lieu, "field-meeting-place", "meeting", false, false),
    field!(redacteur, "field-minutes-author", "meeting", false, false),
    field!(participants, "field-attendees", "attendees", true, false),
    field!(excuses, "field-apologies", "attendees", true, false),
];
pub const GROUPS: &[FieldGroup] = &[
    FieldGroup {
        id: "meeting",
        label_key: "group-meeting",
    },
    FieldGroup {
        id: "attendees",
        label_key: "group-attendees",
    },
];
pub const CONTRACT: ModelContract<Metadata> = ModelContract {
    function: "compte-rendu",
    fields: FIELDS,
    options: |m| &m.options,
    signature_image: None,
};
