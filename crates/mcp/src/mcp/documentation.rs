//! La fiche commune reste dans le cœur ; seule cette couche décrit l’enveloppe MCP.
use super::{catalogue, policy::Policy};
use serde_json::{Value, json};
use typstographe_core::engine::{CommandError, Engine, catalogue as core_catalogue};

pub fn describe(id: &str, context: &Value, policy: &Policy) -> Result<Value, CommandError> {
    let tool = catalogue::tools().iter().find(|t| t.name == id);
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
            _ => return Engine::command_documentation(id),
        };
        let tool = tool.expect("helper tool");
        json!({"command":{"id":id,"description":tool.description,"family":"query","params_schema":tool.input_schema,"batch_step":false},
            "details":details,"examples":[{"params":params,"bindings":[]}],"defaults":{},
            "restrictions":restriction,"result":if id == "render.page" {"A structured page result and inline PNG image content."} else {"A structured discovery result under outcome."},
            "example_format":"Supply the current outer document reference when required by the effective MCP schema.",
            "effects":{"batch_step":false,"requires_document":id=="render.page","filesystem":"No file IO requested.","render_cache":if id=="render.page" {"May compile a derived current render."} else {"No rendering."}},
            "transaction":"Read only; no undo entry."})
    };
    info["prototype_support"] = spec.map(|s| policy.support(s))
        .unwrap_or_else(|| json!({"exposed":true,"authorized":true,"filesystem":false,"read_only":policy.describe()["read_only"]}));
    info["mcp_input_schema"] = tool.map(|t| json!(t.input_schema)).unwrap_or(Value::Null);
    info["mcp_example_guidance"] = json!(
        "batch_examples (when present) describe calls to their own id (document.batch), with preconditions and expected results; do not send their steps to the described formatting tool. Their root document reference follows the same rules. examples[].params are templates: replace every binding with current values from the named tool's outcome. Supply document as the outer reference from the latest response, not a core guard; omit a separate root guard. Never invent IDs or references. render may be omitted for automatic current compilation; explicit references are validated. A selector can supply target/issue/occurrence where the schema allows it. Paths are illustrative and still require declared permissions."
    );
    info["context_availability"] = spec.map(|_| context["entries"].as_array().expect("entries").iter()
        .find(|e| e["id"]==id).expect("registered entry")["context_availability"].clone())
        .unwrap_or_else(|| if id == "render.page" && context["guard"].is_null() {
            json!({"state":"disabled","reason":{"code":"unavailable","params":{"reason":"no_document"}}})
        } else {json!({"state":"enabled"})});
    Ok(info)
}
