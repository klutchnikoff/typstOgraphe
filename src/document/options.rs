//! Options documentaires et identités de libellés.
use serde::{Deserialize, Serialize};
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum PaperFormat {
    A3,
    #[default]
    A4,
    A5,
    #[serde(rename = "us-letter")]
    Letter,
    #[serde(rename = "us-legal")]
    Legal,
}
impl PaperFormat {
    pub const ALL: [Self; 5] = [Self::A3, Self::A4, Self::A5, Self::Letter, Self::Legal];
    pub fn typst_name(self) -> &'static str {
        match self {
            Self::A3 => "a3",
            Self::A4 => "a4",
            Self::A5 => "a5",
            Self::Letter => "us-letter",
            Self::Legal => "us-legal",
        }
    }
    pub fn label_key(self) -> &'static str {
        match self {
            Self::A3 => "paper-a3",
            Self::A4 => "paper-a4",
            Self::A5 => "paper-a5",
            Self::Letter => "paper-letter",
            Self::Legal => "paper-legal",
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema,
)]
pub enum DocumentLanguage {
    #[default]
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "nl")]
    Dutch,
}
impl DocumentLanguage {
    pub const ALL: [Self; 7] = [
        Self::French,
        Self::English,
        Self::German,
        Self::Spanish,
        Self::Italian,
        Self::Portuguese,
        Self::Dutch,
    ];
    pub fn code(self) -> &'static str {
        match self {
            Self::French => "fr",
            Self::English => "en",
            Self::German => "de",
            Self::Spanish => "es",
            Self::Italian => "it",
            Self::Portuguese => "pt",
            Self::Dutch => "nl",
        }
    }
    pub fn label_key(self) -> &'static str {
        match self {
            Self::French => "language-french",
            Self::English => "language-english",
            Self::German => "language-german",
            Self::Spanish => "language-spanish",
            Self::Italian => "language-italian",
            Self::Portuguese => "language-portuguese",
            Self::Dutch => "language-dutch",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentOptions {
    pub paper: PaperFormat,
    pub landscape: bool,
    pub language: DocumentLanguage,
    /// None laisse au modèle le choix du marqueur selon sa charte et la langue.
    pub bullet_marker: Option<String>,
    /// Exceptions de relecture du document, jamais transmises au modèle Typst.
    pub spelling_ignored: crate::spelling::WordLists,
}

pub fn valid_bullet_marker(marker: &str) -> bool {
    let mut chars = marker.chars();
    chars
        .next()
        .is_some_and(|c| !c.is_whitespace() && !c.is_control())
        && chars.next().is_none()
}
