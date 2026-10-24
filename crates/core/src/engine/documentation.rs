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
        let guide = super::documentation_guides::guide(id)
            .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "documentation_missing"))?;
        let defaults: serde_json::Map<String, Value> = spec.params_schema["properties"]
            .as_object()
            .into_iter()
            .flat_map(|p| p.iter())
            .filter_map(|(key, schema)| schema.get("default").map(|v| (key.clone(), v.clone())))
            .collect();
        Ok(
            json!({"command":spec,"details":guide.details,"examples":[guide.example],
            "example_format":"examples[].params are parameters. Before calling, replace every null reference listed in bindings with a value read from the indicated operation; never invent IDs or guards. Root guard is supplied by the caller or adapter. Examples may require a suitable current selection or structure; see restrictions.",
            "defaults":defaults,"restrictions":guide.restrictions,"result":guide.result,
            "target_guidance":"Text targets default to the current logical selection except document.text (all text) and document.statistics (no selection counter). Explicit targets retain their revision guard; refresh after document edits. Structural IDs are opaque and never character offsets.",
            "effects":effects(spec),
            "batch_examples":super::documentation_examples::style_batches(id),
            "transaction":match spec.family { Family::Query => "Read only; no document undo entry. A read may populate derived caches.", Family::Selection => "Moves the selection; no document undo entry.", Family::File => "Lifecycle, save and export operations are explicit and are not undone by editing history. Export does not mark a document saved.", Family::History => "Moves through existing history; does not add a new edit. Disk writes are not undone.", _ => "Changes to document content are undoable. Discrete calls form an edit; typing inputs may group. A batch is one transaction with optional final save; export is separate." }}),
        )
    }
}
fn effects(spec: &CommandSpec) -> Value {
    let id = spec.id.as_str();
    json!({"batch_step":spec.batch_step,"requires_document":spec.availability != AvailabilityRule::Always,
        "document":"Mutations publish only after validation; selection-only and read operations do not change document content.",
        "filesystem":match id {
            "document.open" => "Reads a saved archive; writable open may create/acquire its lock companion.",
            "document.save" | "document.save_as" => "Explicit atomic archive write, lock acquisition and fingerprint check.",
            "document.export" => "Explicit new/empty export directory; may leave a partial export after an IO error.",
            "document.batch" => "No writes in steps; optional single final atomic archive save. Path imports are reads.",
            "recovery.delete" => "Deletes one configured recovery copy explicitly.",
            "recovery.restore" => "Reads a configured recovery copy into an unsaved document.",
            "document.new" | "insert.image" | "image.replace" | "document.set_signature" | "document.set_model" => "May read an explicit resource path; imports are embedded in memory. No automatic save.",
            _ => "No document filesystem write. Closing/replacing a session may release a cooperative lock."
        },
        "render_cache":if id == "render.compile" {"Installs a derived in-memory render for this revision; no disk write or undo entry."} else {"Document changes invalidate derived rendering; other reads do not install an edited document."}})
}
