//! Identités publiques des champs ; les paramètres Typst et archives restent inchangés.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum MetadataField {
    #[serde(rename = "article.references")]
    ArticleReferences,
    #[serde(rename = "article.citation_style")]
    ArticleCitationStyle,
    #[serde(rename = "article.show_outline")]
    ArticleOutline,
    #[serde(rename = "article.authors")]
    ArticleAuthors,
    #[serde(rename = "article.affiliations")]
    ArticleAffiliations,
    #[serde(rename = "article.localizations")]
    ArticleLocalizations,
    #[serde(rename = "sender.name")]
    SenderName,
    #[serde(rename = "sender.role")]
    SenderRole,
    #[serde(rename = "sender.department")]
    SenderDepartment,
    #[serde(rename = "sender.email")]
    SenderEmail,
    #[serde(rename = "sender.organisation")]
    SenderOrganisation,
    #[serde(rename = "sender.address")]
    SenderAddress,
    #[serde(rename = "sender.phone")]
    SenderPhone,
    #[serde(rename = "recipient.name")]
    RecipientName,
    #[serde(rename = "recipient.address")]
    RecipientAddress,
    #[serde(rename = "letter.subject")]
    LetterSubject,
    #[serde(rename = "letter.reference")]
    LetterReference,
    #[serde(rename = "letter.place")]
    LetterPlace,
    #[serde(rename = "letter.date")]
    LetterDate,
    #[serde(rename = "meeting.title")]
    MeetingTitle,
    #[serde(rename = "meeting.date")]
    MeetingDate,
    #[serde(rename = "meeting.place")]
    MeetingPlace,
    #[serde(rename = "meeting.author")]
    MeetingAuthor,
    #[serde(rename = "meeting.attendees")]
    MeetingAttendees,
    #[serde(rename = "meeting.apologies")]
    MeetingApologies,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldValue {
    pub field: MetadataField,
    pub value: serde_json::Value,
    pub parameter: &'static str,
    pub group: &'static str,
    pub multiline: bool,
    pub kind: super::FieldKind,
    pub required_for_export: bool,
    #[serde(skip)]
    pub editor_id: &'static str,
    #[serde(skip)]
    pub label_key: &'static str,
}

pub(super) fn values<T>(metadata: &T, fields: &[super::TextField<T>]) -> Vec<FieldValue> {
    fields
        .iter()
        .map(|f| FieldValue {
            field: f.field,
            value: serde_json::Value::String((f.get)(metadata).into()),
            parameter: f.parameter,
            group: f.group,
            multiline: f.multiline,
            kind: f.kind,
            required_for_export: f.required_for_export,
            editor_id: f.editor_id,
            label_key: f.label_key,
        })
        .collect()
}
pub(super) fn value_mut<'a, T>(
    metadata: &'a mut T,
    fields: &[super::TextField<T>],
    field: MetadataField,
) -> Option<&'a mut String> {
    fields
        .iter()
        .find(|f| f.field == field)
        .map(|f| (f.get_mut)(metadata))
}
