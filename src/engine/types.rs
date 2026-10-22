use super::{CommandError, registry::define_calls};
use crate::{
    document::structure::LogicalSelection,
    session::{SelectionTarget, TargetGuard},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Les cibles symboliques sont résolues à l'appel ; une sélection explicite
/// conserve obligatoirement sa garde, même si l'utilisateur déplace le curseur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TextTarget {
    Current {},
    All {},
    Selection {
        guard: TargetGuard,
        selection: LogicalSelection,
    },
}
impl Default for TextTarget {
    fn default() -> Self {
        Self::Current {}
    }
}
impl From<SelectionTarget> for TextTarget {
    fn from(target: SelectionTarget) -> Self {
        Self::Selection {
            guard: target.guard,
            selection: target.selection,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetParams {
    #[serde(default)]
    pub target: TextTarget,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InsertTextParams {
    #[serde(default)]
    pub target: TextTarget,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StyleValue {
    #[default]
    Toggle,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StyleParams {
    #[serde(default)]
    pub target: TextTarget,
    #[serde(default)]
    pub value: StyleValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkParams {
    #[serde(default)]
    pub target: TextTarget,
    pub url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextQueryParams {
    #[serde(default = "all_text")]
    pub target: TextTarget,
}
fn all_text() -> TextTarget {
    TextTarget::All {}
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchParams {
    pub query: String,
    #[serde(default)]
    pub case_sensitive: bool,
}

define_calls! {
    Command, command_specs;
    InsertText(InsertTextParams) => "text.insert", "Insert literal text or replace a selection.", Edit, Document;
    DeleteText(TargetParams) => "text.delete", "Delete text within validated structural boundaries.", Edit, Document;
    SelectText(TargetParams) => "text.select", "Set a continuous, oriented text selection.", Selection, Document;
    Bold(StyleParams) => "text.bold", "Set or toggle bold text or the typing style.", Edit, Document;
    Italic(StyleParams) => "text.italic", "Set or toggle italic text or the typing style.", Edit, Document;
    Clear(TargetParams) => "text.clear", "Remove inline formatting from text or the typing style.", Edit, Document;
    SetLink(LinkParams) => "text.link", "Apply an explicit URL to a text selection.", Edit, LinkSelection;
    RemoveLink(TargetParams) => "text.remove_link", "Remove links from a text selection.", Edit, LinkSelection;
    Paragraph(TargetParams) => "block.paragraph", "Convert selected paragraphs to ordinary text.", Edit, Document;
    Heading1(TargetParams) => "block.heading_1", "Set level-one headings.", Edit, Document;
    Heading2(TargetParams) => "block.heading_2", "Set level-two headings.", Edit, Document;
    Heading3(TargetParams) => "block.heading_3", "Set level-three headings.", Edit, Document;
    Bullet(TargetParams) => "block.bullet", "Set an unordered list using the document marker policy.", Edit, Document;
    Numbered(TargetParams) => "block.numbered", "Set an ordered list.", Edit, Document;
    Description(TargetParams) => "block.description", "Set description items with term and definition zones.", Edit, Document;
    Quote(TargetParams) => "block.quote", "Set a block quotation with an optional attribution.", Edit, Document;
    Postscript(Empty) => "letter.insert_postscript", "Create or select the letter postscript.", Letter, Letter;
    Attachment(Empty) => "letter.insert_attachment", "Insert an attachment mention into a letter.", Letter, Letter;
    Undo(Empty) => "edit.undo", "Undo the previous document transaction.", History, Undo;
    Redo(Empty) => "edit.redo", "Redo the next document transaction.", History, Redo;
}

define_calls! {
    Query, query_specs;
    Inspect(Empty) => "document.inspect", "Read the current revision, selection and history availability.", Query, Read;
    Text(TextQueryParams) => "document.text", "Read editable text without Typst markup.", Query, Read;
    Structure(Empty) => "document.structure", "Read blocks, text zones, roles and their opaque identities.", Query, Read;
    Metadata(Empty) => "metadata.get", "Read the document metadata.", Query, Read;
    Commands(Empty) => "commands.list", "Read the implemented command and query catalogue with JSON schemas.", Query, Read;
    Find(SearchParams) => "search.find", "Find literal Unicode text without modifying the selection.", Query, Read;
}

impl Command {
    pub(super) fn target(&self) -> Option<TextTarget> {
        Some(match self {
            Self::InsertText(p) => p.target,
            Self::Bold(p) | Self::Italic(p) => p.target,
            Self::SetLink(p) => p.target,
            Self::DeleteText(p)
            | Self::SelectText(p)
            | Self::Clear(p)
            | Self::RemoveLink(p)
            | Self::Paragraph(p)
            | Self::Heading1(p)
            | Self::Heading2(p)
            | Self::Heading3(p)
            | Self::Bullet(p)
            | Self::Numbered(p)
            | Self::Description(p)
            | Self::Quote(p) => p.target,
            _ => return None,
        })
    }
}
