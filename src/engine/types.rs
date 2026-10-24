use super::batch_types::*;
use super::file_types::*;
use super::metadata_types::*;
use super::model_types::*;
use super::recovery_types::*;
use super::search_types::*;
use super::spelling_types::*;
use super::{CommandError, registry::define_calls};
use super::{
    ImageAltParams, ImageTargetParams, InsertImageParams, LocateParams, ReplaceImageParams,
    SelectionBoxesParams, SignatureParams, StatisticsParams,
};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectionPart {
    Body,
    Annotation,
    Term,
    Definition,
}

/// Une partie relative est résolue par le moteur dans la structure ciblée.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectTextParams {
    #[serde(default)]
    pub target: TextTarget,
    #[serde(default)]
    pub part: Option<SelectionPart>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InputRules {
    #[default]
    Literal,
    Typing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InsertTextParams {
    #[serde(default)]
    pub target: TextTarget,
    pub text: String,
    #[serde(default)]
    pub input_rules: InputRules,
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
    Batch(BatchParams) => "document.batch", "Apply bounded editing steps on a private session as one undoable transaction, with an optional single final atomic save.", Batch, Document, false;
    RestoreRecovery(RestoreRecoveryParams) => "recovery.restore", "Restore a configured recovery reference as an unsaved copy, protecting current work.", File, Always, false;
    DeleteRecovery(DeleteRecoveryParams) => "recovery.delete", "Explicitly delete one configured recovery copy without modifying document history.", File, Always, false;
    NewDocument(NewDocumentParams) => "document.new", "Create an empty document from a supplied type/model and explicit initial values, protecting unsaved work.", File, Always, false;
    OpenDocument(OpenDocumentParams) => "document.open", "Open an autonomous archive from an explicit path, protecting unsaved work.", File, Always, false;
    CloseDocument(CloseDocumentParams) => "document.close", "Close the current document after an explicit decision about unsaved work.", File, Document, false;
    SaveDocument(SaveDocumentParams) => "document.save", "Atomically save the guarded document to an explicit or current path without changing editing history.", File, Document, false;
    SaveAs(SaveAsParams) => "document.save_as", "Atomically save the guarded document to a required path, preserving the previous file.", File, Document, false;
    Export(ExportParams) => "document.export", "Export PDF, autonomous Typst sources or both into a new or empty directory, with optional page PNGs.", File, Document, false;
    SetModel(SetModelParams) => "document.set_model", "Validate and install Typst model source or a UTF-8 file with controlled dependencies in one guarded transaction.", Edit, Document, true;
    PersonalModel(GuardParams) => "courrier.model.personal", "Apply the supplied personal-letter model using the shared guarded model operation.", Letter, Letter, true;
    SetMetadata(SetMetadataParams) => "metadata.set", "Set an active document-type field, with a guarded value and optional typing history grouping.", Edit, Document, true;
    SetOption(SetOptionParams) => "document.set_option", "Set a typed document option while enforcing the model page and marker policy.", Edit, Document, true;
    NextSpelling(TargetParams) => "spelling.next", "Select the next spelling issue in the current document, wrapping around.", Selection, Document, true;
    PreviousSpelling(TargetParams) => "spelling.previous", "Select the previous spelling issue in the current document, wrapping around.", Selection, Document, true;
    CorrectSpelling(SpellingCorrectionParams) => "spelling.correct", "Correct a revision-guarded spelling issue while preserving inline styles.", Edit, Document, true;
    IgnoreSpelling(SpellingIssueParams) => "spelling.ignore", "Accept a validated word for the document language in this document only.", Edit, Document, true;
    ResetSpelling(GuardParams) => "spelling.reset", "Clear spelling exceptions for all languages of the guarded document.", Edit, Document, true;
    FindNext(FindParams) => "edit.find_next", "Select the next literal match, wrapping around the editable document.", Selection, Document, true;
    FindPrevious(FindParams) => "edit.find_previous", "Select the previous literal match, wrapping around the editable document.", Selection, Document, true;
    ReplaceCurrent(ReplaceCurrentParams) => "edit.replace_current", "Replace a revision-guarded occurrence with expected text, then select the next match.", Edit, Document, true;
    ReplaceAll(ReplaceAllParams) => "edit.replace_all", "Replace all literal matches from one guarded document revision in one transaction.", Edit, Document, true;
    InsertText(InsertTextParams) => "text.insert", "Insert text or replace a selection, with literal input by default and optional typing rules.", Edit, Document, true;
    DeleteText(TargetParams) => "text.delete", "Delete text within validated structural boundaries.", Edit, Document, true;
    SelectText(SelectTextParams) => "text.select", "Select text or move to a body, annotation, description term or definition.", Selection, Document, true;
    Bold(StyleParams) => "text.bold", "Set or toggle bold text or the typing style.", Edit, Document, true;
    Italic(StyleParams) => "text.italic", "Set or toggle italic text or the typing style.", Edit, Document, true;
    Clear(TargetParams) => "text.clear", "Remove inline formatting from text or the typing style.", Edit, Document, true;
    SetLink(LinkParams) => "text.link", "Apply an explicit URL to a text selection.", Edit, LinkSelection, true;
    RemoveLink(TargetParams) => "text.remove_link", "Remove links from a text selection.", Edit, LinkSelection, true;
    Paragraph(TargetParams) => "block.paragraph", "Convert selected paragraphs to ordinary text.", Edit, Document, true;
    Heading1(TargetParams) => "block.heading_1", "Set level-one headings.", Edit, Document, true;
    Heading2(TargetParams) => "block.heading_2", "Set level-two headings.", Edit, Document, true;
    Heading3(TargetParams) => "block.heading_3", "Set level-three headings.", Edit, Document, true;
    Bullet(TargetParams) => "block.bullet", "Set an unordered list using the document marker policy.", Edit, Document, true;
    Numbered(TargetParams) => "block.numbered", "Set an ordered list.", Edit, Document, true;
    Description(TargetParams) => "block.description", "Set description items with term and definition zones.", Edit, Document, true;
    Quote(TargetParams) => "block.quote", "Set a block quotation with an optional attribution.", Edit, Document, true;
    InsertImage(InsertImageParams) => "insert.image", "Insert a validated image with optional literal caption and alternative text.", Image, ImageInsertion, true;
    ReplaceImage(ReplaceImageParams) => "image.replace", "Replace an image resource while preserving its caption and alternative text.", Image, Image, true;
    RemoveImage(ImageTargetParams) => "image.remove", "Remove an image block and its caption in one transaction.", Image, Image, true;
    SetImageAlt(ImageAltParams) => "image.set_alt", "Set alternative text on a revision-guarded image block.", Image, Image, true;
    SetSignature(SignatureParams) => "document.set_signature", "Set or remove the letter signature using explicit image data or a file.", Letter, Letter, true;
    SplitBlock(TargetParams) => "block.split", "Split a quote, list item or letter note, or exit an empty item or optional annotation.", Edit, StructuredBlock, true;
    ExitBlock(TargetParams) => "block.exit", "Create a paragraph after an annotated structure or letter note without replacing its content.", Edit, StructuredBlock, true;
    Postscript(Empty) => "letter.insert_postscript", "Create or select the letter postscript.", Letter, Letter, true;
    Attachment(Empty) => "letter.insert_attachment", "Insert an attachment mention into a letter.", Letter, Letter, true;
    Undo(Empty) => "edit.undo", "Undo the previous document transaction.", History, Undo, false;
    Redo(Empty) => "edit.redo", "Redo the next document transaction.", History, Redo, false;
}

define_calls! {
    Query, query_specs;
    Validate(GuardParams) => "document.validate", "Validate a guarded draft, model, resources and options; report errors separately from fields missing for export, without editing or installing a render.", Query, Read, false;
    Compile(GuardParams) => "render.compile", "Compile a guarded document synchronously into a cached in-memory render reference and diagnostics, without rasterization, export or document edits.", Query, Read, false;
    DocumentTypes(Empty) => "document_types.list", "List built-in types, supplied models, creation availability and documentary command capabilities without requiring an open document.", Query, Always, false;
    Source(Empty) => "document.source", "Read the committed documentary Typst source with its revision guard, without executing code.", Query, Read, false;
    Assets(Empty) => "document.assets", "Inventory retained document/model resources and packaged dependencies using the save/export retention policy, without returning file contents or undo-only assets.", Query, Read, false;
    Locate(LocateParams) => "render.locate", "Locate a logical text target in an identified render or return the guarded text target at a page point, without moving selection.", Query, Read, false;
    SelectionBoxes(SelectionBoxesParams) => "render.selection_boxes", "Read page quadrilaterals for a guarded text selection on an identified render, in Typst points.", Query, Read, false;
    Statistics(StatisticsParams) => "document.statistics", "Read Unicode word and grapheme counts for the document and an optional text target, excluding structural markers and private fields.", Query, Read, false;
    RecoveryList(Empty) => "recovery.list", "List opaque references from the configured recovery service without opening documents.", Query, Read, false;
    Models(Empty) => "models.list", "List supplied models for the active document type without changing the document.", Query, Read, false;
    ModelPresentation(Empty) => "models.presentation", "Read the current model page constraints and permitted bullet markers.", Query, Read, false;
    SpellingIssues(Empty) => "spelling.issues", "Read revision-guarded spelling issues and dictionary availability using the configured personal words.", Query, Read, false;
    SpellingSuggestions(SpellingSuggestionsParams) => "spelling.suggestions", "Suggest corrections for an explicit word and language or a validated spelling issue, without changing selection.", Query, Read, false;
    Inspect(Empty) => "document.inspect", "Read session state, document type, documentary command capabilities, guarded selection and typing style; document context is null when closed.", Query, Always, false;
    Text(TextQueryParams) => "document.text", "Read editable text without Typst markup.", Query, Read, false;
    Structure(Empty) => "document.structure", "Read blocks, text zones, roles and their opaque identities.", Query, Read, false;
    Metadata(Empty) => "metadata.get", "Read the document metadata.", Query, Read, false;
    Commands(Empty) => "commands.list", "Read command/query schemas and context availability with reasons, distinguishing calls that still require parameters; no execution or file access.", Query, Always, false;
    Find(SearchParams) => "search.find", "Find literal Unicode text without modifying the selection.", Query, Read, false;
}

impl Command {
    pub(super) fn target(&self) -> Option<TextTarget> {
        Some(match self {
            Self::InsertText(p) => p.target,
            Self::InsertImage(p) => p.target,
            Self::Bold(p) | Self::Italic(p) => p.target,
            Self::SetLink(p) => p.target,
            Self::SelectText(p) => p.target,
            Self::FindNext(p) | Self::FindPrevious(p) => p.target,
            Self::NextSpelling(p) | Self::PreviousSpelling(p) => p.target,
            Self::DeleteText(p)
            | Self::SplitBlock(p)
            | Self::ExitBlock(p)
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
