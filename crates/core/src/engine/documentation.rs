//! Documentation technique commune aux adaptateurs, sans SDK MCP ou chaîne Fluent.
use super::*;
use crate::document_types::{DocumentType, Metadata};
use serde_json::json;

impl Engine {
    pub fn type_documentation(kind: DocumentType) -> Value {
        let (name, description) = match kind {
            DocumentType::Courrier => (
                "Letter",
                "Administrative or personal correspondence with sender, recipient, subject, date, optional signature, postscript and enclosure mentions.",
            ),
            DocumentType::CompteRendu => (
                "Meeting minutes",
                "Meeting report with title, date, place, author, attendees and apologies, followed by freely structured text.",
            ),
        };
        let metadata = Metadata::blank(kind);
        let models: Vec<_> = kind.models().map(|m| json!({
            "id":m.model.id(),"name":m.name,"type":kind,"description":m.description,"keywords":m.keywords
        })).collect();
        json!({"id":kind,"name":name,"description":description,"can_create":kind.can_create(),
            "fields":metadata.field_values(),"default_options":metadata.options(),
            "capabilities":Self::capabilities(kind),"models":models,
            "model_contract":"Models of the same type share fields and capabilities; choosing a model changes presentation only.",
            "field_guidance":"Use metadata.set with a listed field ID. Dates use fixed ISO YYYY-MM-DD values. Empty fields may be valid in a draft; required_for_export applies only to export readiness."})
    }

    pub fn command_documentation(id: &str) -> Result<Value, CommandError> {
        let spec = catalogue()
            .iter()
            .find(|s| s.id.as_str() == id)
            .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
        let (details, examples) = guidance(id);
        Ok(json!({"command":spec,"details":details,"examples":examples,
            "defaults":"Defaults and required fields are declared in params_schema. A missing text target means the current logical selection; document.text defaults to all text. Never reuse revision-guarded references after a mutation.",
            "transaction":match spec.family { Family::Query => "Read only; no document undo entry.", Family::Selection => "Moves the selection; no document undo entry.", _ => "Document mutations are undoable. Batch steps form one transaction; save and export are explicit." }}))
    }
}

fn guidance(id: &str) -> (&'static str, Vec<Value>) {
    let (details, example) = match id {
        "document.new" => (
            "Choose a creatable type and a supplied model from discovery. Initial field values and an embedded signature are explicit; no personal profile is loaded. Replacing unsaved work requires discard_changes.",
            json!({"type":"courrier","model":"courrier.model.personal"}),
        ),
        "text.insert" => (
            "Insert literal text at the target, replacing any selection. A single newline is a space in the same paragraph; two newlines create a paragraph. Apply formatting through dedicated commands rather than inserting Typst code.",
            json!({"text":"Bonjour Élodie.\n\nUn nouveau paragraphe."}),
        ),
        "text.bold" | "text.italic" => (
            "Use on/off for predictable formatting, or toggle. An empty selection changes the subsequent typing style; a selected range changes that text.",
            json!({"target":{"kind":"all"},"value":"on"}),
        ),
        "text.link" => (
            "Apply an explicit URL to selected text. The visible label stays in the document.",
            json!({"target":{"kind":"all"},"url":"https://example.org"}),
        ),
        "metadata.set" => (
            "Set one field listed by the document type. The field identifier is independent of UI language. A date is a fixed ISO YYYY-MM-DD value.",
            json!({"field":"letter.subject","value":"Demande de renseignements"}),
        ),
        "document.set_option" => (
            "Set a document option, independently of UI language. The engine applies the active model presentation policy.",
            json!({"option":{"key":"language","value":"fr"}}),
        ),
        "letter.insert_postscript" => (
            "Create or select the letter's postscript and move the insertion point to its body; then insert literal text. This is a letter capability.",
            json!({}),
        ),
        "letter.insert_attachment" => (
            "Insert an enclosure mention and move to its text. Mentions do not embed the enclosed files.",
            json!({}),
        ),
        "block.quote" => (
            "Convert the target paragraphs to a block quotation. Use text.select with part=annotation to edit the optional attribution.",
            json!({"target":{"kind":"all"}}),
        ),
        "insert.image" => (
            "Insert an image with optional caption and alternative text. The caption is a logical annotation. The stored resource follows the common save/export retention policy.",
            json!({"image":{"kind":"embedded","extension":"svg","data":"PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxIiBoZWlnaHQ9IjEiLz4="},"alt":"Description de l’image","caption":"Légende facultative"}),
        ),
        "search.find" => (
            "Find exact literal text without moving the selection. Results contain revision-guarded logical targets. Matching is case-insensitive by default.",
            json!({"query":"Élodie","case_sensitive":true}),
        ),
        "document.validate" => (
            "Report document/model errors separately from fields missing for export. This check never corrects text and does not perform spelling checks.",
            json!({}),
        ),
        "spelling.issues" => (
            "List suspected spelling errors and dictionary availability. This is not grammar checking and does not correct anything automatically.",
            json!({}),
        ),
        "document.batch" => (
            "Execute bounded editing steps on the evolving private draft. Failure publishes nothing. On success the entire lot is one undo transaction; a final save is explicit and export is separate.",
            json!({"steps":[{"id":"letter.insert_postscript","params":{}},{"id":"text.insert","params":{"text":"Une précision."}}]}),
        ),
        _ => (
            "Use the typed parameter schema and the context availability returned by commands.list. Logical targets must belong to the current document revision.",
            Value::Null,
        ),
    };
    (
        details,
        if example.is_null() {
            vec![]
        } else {
            vec![example]
        },
    )
}
