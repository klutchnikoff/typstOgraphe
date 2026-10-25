//! La fiche commune reste dans le cœur ; seule cette couche décrit l’enveloppe MCP.
use super::{catalogue, policy::Policy};
use serde_json::{Value, json};
use typstographe_core::engine::{CommandError, Engine, catalogue as core_catalogue};

pub fn describe(id: &str, context: &Value, policy: &Policy) -> Result<Value, CommandError> {
    let tool = catalogue::for_session(policy.is_attached())
        .iter()
        .find(|t| t.name == id);
    let spec = core_catalogue().iter().find(|s| s.id.as_str() == id);
    let mut info = if spec.is_some() {
        Engine::command_documentation(id)?
    } else {
        let (details, params, restriction) = match id {
            "document_types.describe" => (
                "Describe a built-in type's purpose, fields, defaults, export requirements, capabilities and brief visual model cards.",
                json!({"document_type":"courrier"}),
                "A known type is required; can_create=false remains possible.",
            ),
            "capabilities.describe" => (
                "Describe any known core capability or MCP helper, its effective MCP schema, examples, context availability and server authorization. A documented capability may be unexposed or unavailable.",
                json!({"id":"text.insert"}),
                "Use IDs from tools/list or commands.list. Discovery is available without a document.",
            ),
            "render.page" => (
                "Compile the current document and return one page as inline image/png content plus page count and dimensions in Typst points, without creating files or modifying content or selection.",
                json!({"page":1}),
                "Page numbers start at 1, unlike the 0-based page indices in geometry tools. Requires a current outer document reference; out-of-range pages are refused.",
            ),
            "proposals.submit" => (
                "Prepare an isolated atomic lot for whole-lot human approval. Never publishes or saves. The human can continue writing; any published edit makes this candidate stale. Identical retries with the same request_id return the same status, including after acceptance. At most 64 submissions per invitation and one pending candidate. JSON 4 MiB, 256 steps, summary 2048 bytes, request_id 1–128 bytes.",
                json!({"request_id":"correction-1","summary":"Correct the salutation","steps":[{"id":"text.insert","params":{"selector":{"select":"Cheres collègues"},"text":"Chères collègues"}}]}),
                "Attached only. No file/history/recovery/nested batch steps or final save. Imports require authorization; no arbitrary local paths. No direct edit and no tool to approve. Finish IME/dialogues before resubmitting; nothing is applied later automatically. Return control to the user; do not poll in a loop.",
            ),
            "proposals.get" | "proposals.cancel" => (
                "Read a proposal status, or withdraw it while pending. Uses the opaque proposal ID in this invitation, without a fresh document guard. Terminal states accepted/rejected/stale/cancelled cannot be changed or applied again.",
                json!({"proposal":"ID returned by proposals.submit"}),
                "Attached only. No approval and no undo of an accepted proposal. Disconnect/revoke/close ends the invitation and destroys pending candidates.",
            ),
            _ => return Engine::command_documentation(id),
        };
        let tool = tool.expect("helper tool");
        json!({"command":{"id":id,"description":tool.description,"family":if id.starts_with("proposals.") {"proposal"} else {"query"},"params_schema":tool.input_schema,"batch_step":false},
            "details":details,"examples":[{"params":params,"bindings":[]}],"defaults":{},
            "restrictions":restriction,"result":if id == "render.page" {"A structured page result and inline PNG image content."} else {"A structured discovery result under outcome."},
            "example_format":"Supply the current outer document reference when required by the effective MCP schema.",
            "effects":{"batch_step":false,"requires_document":id=="render.page","filesystem":"No file IO requested.","render_cache":if id=="render.page" {"May compile a derived current render."} else {"No rendering."}},
            "transaction":if id.starts_with("proposals.") {"No publication; whole-lot human acceptance in the application creates one undo entry."} else {"Read only; no undo entry."}})
    };
    info["prototype_support"] = spec.map(|s| policy.support(s))
        .unwrap_or_else(|| json!({"exposed":tool.is_some(),"authorized":!id.starts_with("proposals.") || policy.allows_proposals(),"filesystem":false,"read_only":policy.describe()["read_only"]}));
    info["mcp_input_schema"] = tool.map(|t| json!(t.input_schema)).unwrap_or(Value::Null);
    info["mcp_example_guidance"] = json!(
        "batch_examples (when present) describe calls to their own id (document.batch), with preconditions and expected results; do not send their steps to the described formatting tool. Their root document reference follows the same rules. examples[].params are templates: replace every binding with current values from the named tool's outcome. Supply document as the outer reference from the latest response, not a core guard; omit a separate root guard. Never invent IDs or references. render may be omitted for automatic current compilation; explicit references are validated. A selector can supply target/issue/occurrence where the schema allows it. Paths are illustrative and still require declared permissions."
    );
    if policy.allows_proposals() {
        info["attached_usage"] = json!(
            "Editing tools and document.batch are not callable directly. Put their allowed editing steps and selectors into proposals.submit with a unique request_id and current document reference. For batch_examples reuse only params.steps, never the optional save. The result is pending/published=false, not an edit. Only the human can accept the whole lot in the application. Return control after submission; no polling loop."
        );
        if matches!(id, "document.save" | "document.export" | "document.save_as") {
            info["attached_usage"] = json!(
                "Only document.save and document.export may execute directly when explicitly granted by the user for this invitation. Inspect permissions.attached_files: current_file is an exact file grant, export_roots authorize only exports into a new/empty folder, not document saves or imports. Omit path for document.save; Save As, GUI file dialogs and file steps/final save in proposals remain refused. Use the latest document reference. Only published content is written; accepting a proposal does not save or export. IME/dialogues cause an immediate failure without retry. Background exports revalidate before write; once writing begins, the identified snapshot is independent of later typing. Refresh document.inspect afterwards."
            );
        }
        if matches!(
            id,
            "render.compile" | "render.page" | "render.locate" | "render.selection_boxes"
        ) {
            info["attached_usage"] = json!(
                "Reads use published content, including during IME/dialogues. Cache-miss compilation and PNG rasterization run in the background, with one expensive read in flight per invitation. Current geometry targets are captured in the agent context at dispatch. A changed revision or cancelled/ended request discards the late result; refresh before any explicit new attempt, never retry in a loop. A render read neither edits nor saves nor exports."
            );
        }
        if id == "text.select" {
            info["attached_usage"] = json!(
                "This direct command navigates the agent context only; it never captures or changes the human selection, focus, document or history. Current is the agent selection. The human can explicitly hand off a selection in the application; it does not follow subsequent human navigation. Requires a fresh reference and no IME/dialogue or pending proposal. Other edits still require proposals.submit. Selectors may also target steps inside a proposal."
            );
        }
    }
    info["context_availability"] = spec.map(|_| context["entries"].as_array().expect("entries").iter()
        .find(|e| e["id"]==id).expect("registered entry")["context_availability"].clone())
        .unwrap_or_else(|| if id == "render.page" && context["guard"].is_null() {
            json!({"state":"disabled","reason":{"code":"unavailable","params":{"reason":"no_document"}}})
        } else {json!({"state":"enabled"})});
    Ok(info)
}
