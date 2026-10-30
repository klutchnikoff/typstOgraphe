use super::super::{catalogue as capabilities, commands};
use crate::document_types::{DocumentType, Metadata};
use serde_json::{Value, json};
pub fn type_description(kind: DocumentType) -> Value {
    let metadata = Metadata::blank(kind);
    json!({"id":kind,"can_create":kind.can_create(),"metadata":metadata,
        "fields":metadata.field_values(),"structured_metadata":kind.structured_metadata(),
        "capabilities":capabilities::for_type(kind).map(|c|describe(c.id).unwrap()).collect::<Vec<_>>(),
        "models":kind.models().map(|m|json!({"id":m.model.id(),"name":m.name,"description":m.description,"keywords":m.keywords})).collect::<Vec<_>>()})
}
pub fn types() -> Value {
    json!({"types":DocumentType::ALL.map(type_description)})
}
const QUERIES: &[(&str, &str)] = &[
    (
        "document_types.list",
        "List built-in document types and their interchangeable visual models.",
    ),
    (
        "document_types.describe",
        "Describe metadata, rich header collections and capabilities of one type.",
    ),
    (
        "commands.list",
        "Discover the common structured command and query catalogue.",
    ),
    (
        "capabilities.describe",
        "Describe an operation with its actual parameter schema.",
    ),
    (
        "document.inspect",
        "Read the confirmed structured state, guard and project information.",
    ),
    (
        "document.structure",
        "Read trees and all typed text zones; paths are revision-relative, offsets UTF-16.",
    ),
    (
        "document.text",
        "Read plain text of the body or a typed text target.",
    ),
    (
        "document.source",
        "Read generated Typst without changing the source of truth.",
    ),
    (
        "document.statistics",
        "Count Unicode words and signs in the body and optional target.",
    ),
    (
        "search.find",
        "Find exact text inside each zone, including rich header fields; no cross-zone matches.",
    ),
    (
        "document.validate",
        "Validate the structured draft and report export readiness separately.",
    ),
    (
        "metadata.get",
        "Read document metadata and field/collection definitions.",
    ),
    (
        "models.list",
        "Read brief visual model cards, including the active embedded model.",
    ),
    (
        "resources.list",
        "List admitted immutable image resources; no filesystem access.",
    ),
    (
        "spelling.issues",
        "Read local spelling issues as guarded typed ranges; never correct text.",
    ),
    (
        "spelling.suggestions",
        "Get local suggestions for a literal word and language.",
    ),
    (
        "bibliography.search",
        "Search the project bibliography by title and author.",
    ),
    (
        "context.get",
        "Read this participant's private selection, typing styles, history and selection-level capability availability. Form parameters and adapter permissions still require execution validation.",
    ),
    (
        "commands.available",
        "Check a concrete command through the same resolver and final validation as execution.",
    ),
    (
        "render.compile",
        "Compile confirmed content with embedded Typst and return page dimensions.",
    ),
    (
        "render.page",
        "Render one one-based page to PNG without writing files.",
    ),
    (
        "render.locate",
        "Map a typed body target to preview or a zero-based PagePoint back to a body zone. Template decorations may have no target.",
    ),
    (
        "render.selection_boxes",
        "Read rendered highlight rectangles for a typed body range. Header/annotation geometry is unavailable.",
    ),
];
const ACTIONS: &[(&str, &str)] = &[
    (
        "document.new",
        "Create a structured project of a built-in type; no personal preferences are implied.",
    ),
    (
        "document.open",
        "Open a self-contained structured .typsto; writable access acquires a cooperative lock before reading.",
    ),
    (
        "document.close",
        "Close the project; unsaved work requires explicit discard_changes.",
    ),
    (
        "document.save",
        "Save the current project under its existing lock, checking external changes.",
    ),
    (
        "document.save_as",
        "Save an independent copy; existing targets are refused.",
    ),
    (
        "document.export",
        "Export a confirmed snapshot to a new/empty directory; never mark the project saved.",
    ),
    (
        "document.batch",
        "Apply typed documentary steps atomically, with one history entry and optional final save. A late failure publishes and saves nothing.",
    ),
    (
        "edit.undo",
        "Undo this participant's last confirmed contribution.",
    ),
    ("edit.redo", "Redo this participant's contribution."),
    (
        "context.set",
        "Atomically set private directed selection and typing styles; never edit content, move another participant's focus or create history.",
    ),
    (
        "context.style",
        "Switch a private typing style on/off; stopping it does not alter already written text.",
    ),
    (
        "context.insert",
        "Insert literal prose at this participant's single-zone selection using its typing styles. For explicit edits use text.insert with target and marks.",
    ),
];
pub fn is_query(id: &str) -> bool {
    QUERIES.iter().any(|(key, _)| *key == id)
}
fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn extra_schema(id: &str) -> Value {
    let string = json!({"type":"string"});
    let mut defs = commands::schema()["$defs"].clone();
    let mut s = match id {
        "document.new" => object(
            json!({"type":{"enum":DocumentType::ALL.map(|k|serde_json::to_value(k).unwrap())},"model":string,"metadata":{"type":"object"},"discard_changes":{"type":"boolean"}}),
            &["type"],
        ),
        "document.open" => object(
            json!({"path":string,"read_only":{"type":"boolean","default":false},"discard_changes":{"type":"boolean"}}),
            &["path"],
        ),
        "document.save_as" => object(json!({"path":string}), &["path"]),
        "document.close" => object(json!({"discard_changes":{"type":"boolean"}}), &[]),
        "document.export" => object(
            json!({"directory":string,"format":{"enum":["pdf","typst","both"]},"page_images":{"type":"boolean"}}),
            &["directory", "format"],
        ),
        "document.batch" => object(
            json!({"steps":{"type":"array","minItems":1,"maxItems":super::MAX_BATCH_STEPS,"items":{"type":"object","properties":{"id":string,"params":{"type":"object"}},"required":["id"],"additionalProperties":false}},"save":{"type":"object","properties":{"kind":{"enum":["current","as"]},"path":string},"required":["kind"],"additionalProperties":false}}),
            &["steps"],
        ),
        "context.set" => {
            let schema =
                serde_json::to_value(schemars::schema_for!(super::super::editing::Selection))
                    .unwrap();
            for (k, v) in schema["$defs"].as_object().unwrap() {
                defs[k] = v.clone();
            }
            let mut selection = schema;
            selection.as_object_mut().unwrap().remove("$defs");
            selection.as_object_mut().unwrap().remove("$schema");
            object(
                json!({"selection":{"anyOf":[selection,{"type":"null"}]},"typing_styles":{"type":"array","maxItems":3,"items":{"$ref":"#/$defs/Style"}}}),
                &["selection", "typing_styles"],
            )
        }
        "context.style" => object(
            json!({"style":{"$ref":"#/$defs/Style"},"value":{"$ref":"#/$defs/Switch"}}),
            &["style", "value"],
        ),
        "context.insert" => object(json!({"text":string}), &["text"]),
        "document_types.describe" => object(json!({"type":string}), &["type"]),
        "capabilities.describe" => object(json!({"id":string}), &["id"]),
        "search.find" => object(
            json!({"query":string,"case_sensitive":{"type":"boolean","default":false}}),
            &["query"],
        ),
        "document.text" | "document.statistics" => {
            object(json!({"target":{"$ref":"#/$defs/TextTarget"}}), &[])
        }
        "spelling.suggestions" => object(
            json!({"word":string,"language":{"$ref":"#/$defs/DocumentLanguage"}}),
            &["word", "language"],
        ),
        "bibliography.search" => object(json!({"query":string}), &["query"]),
        "commands.available" => object(json!({"command":commands::schema()}), &["command"]),
        "render.locate" => {
            let mut value = object(
                json!({"target":{"$ref":"#/$defs/TextTarget"},"point":{"type":"object","properties":{"page":{"type":"integer","minimum":0},"point":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2}},"required":["page","point"],"additionalProperties":false}}),
                &[],
            );
            value["oneOf"] = json!([{"required":["target"],"not":{"required":["point"]}},{"required":["point"],"not":{"required":["target"]}}]);
            value
        }
        "render.selection_boxes" => {
            object(json!({"target":{"$ref":"#/$defs/TextTarget"}}), &["target"])
        }
        "render.page" => object(json!({"page":{"type":"integer","minimum":1}}), &["page"]),
        _ => object(json!({}), &[]),
    };
    s["$defs"] = defs;
    s
}
fn build_catalogue() -> Value {
    let schema = commands::schema();
    let mut entries: Vec<Value> = schema["oneOf"].as_array().unwrap().iter().map(|variant| {
        let id = variant["properties"]["id"]["const"].as_str().unwrap();
        let mut params = variant["properties"]["params"].clone();
        params["$defs"] = schema["$defs"].clone();
        if params["properties"].get("target").is_some() {
            params["properties"]["selector"] = json!({"type":"object","properties":{"select":{"type":"string","minLength":1},"occurrence":{"type":"integer","minimum":1},"block":{"type":"integer","minimum":1}},"additionalProperties":false,"oneOf":[{"required":["select"],"not":{"required":["block"]}},{"required":["block"],"not":{"anyOf":[{"required":["select"]},{"required":["occurrence"]}]}}]});
            params["required"].as_array_mut().unwrap().retain(|v|v!="target");
            params["oneOf"] = json!([{"required":["target"],"not":{"required":["selector"]}},{"required":["selector"],"not":{"required":["target"]}}]);
        }

        json!({"id":id,"family":"edit","batch_step":true,"params_schema":params,
            "description":command_description(id)})
    }).collect();
    entries.extend(QUERIES.iter().chain(ACTIONS).map(|(id, description)|json!({"id":id,"family":if is_query(id){"query"}else{"session"},"batch_step":false,"description":description,"params_schema":extra_schema(id)})));
    entries.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    json!({"version":super::super::contract::VERSION,"target_offsets":"utf16","entries":entries})
}
fn cached() -> &'static Value {
    static VALUE: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    VALUE.get_or_init(build_catalogue)
}
pub fn catalogue() -> Value {
    cached().clone()
}
pub fn describe(id: &str) -> Option<Value> {
    let mut value = cached()["entries"]
        .as_array()?
        .iter()
        .find(|s| s["id"] == id)
        .cloned()
        .or_else(|| capabilities::describe(id))?;
    value["examples"] = json!(super::examples::for_id(id));
    if capabilities::find(id).is_some() {
        value["capability"] = json!(capabilities::find(id).unwrap());
        value["editor"] = capabilities::editor(id);
        value["parameters"] = capabilities::parameters(id);
        value["execution_note"] = json!(
            "Examples are documentary steps, not direct semantic tool names. Resolve paths in the latest guarded structure. Admit image resources first; citations require existing bibliography file IDs and keys. Model presentation is independent of these nodes."
        );
    }
    Some(value)
}

fn command_description(id: &str) -> String {
    match id {
        "text.insert" => "Insert/replace literal prose in one typed range; marks are explicit and default to none. No Typst/Markdown input rules.",
        "text.bold" | "text.italic" => "Apply or remove this style on a nonempty typed range (value on/off). Never changes future typing; use context.style for that.",
        "inline.replace" => "Replace a typed inline range with one node (for example footnote or citation). Discover the node via capabilities.describe.",
        "block.insert" | "block.replace" | "block.splice" => "Edit a guarded tree with explicit nodes (quote, list, figure, postscript…). Discover examples via capabilities.describe.",
        "resource.add" => "Admit immutable image bytes (base64), validated before use; a batch may add the resource then insert its figure or signature.",
        "document.set_model" => "Set the model ID and full pure Typst source together. The model must implement this document type; publication validates its policy.",
        "metadata.set" => "Set a typed metadata, record field or document option. Values are validated against the type and model policy.",
        "record.add" | "record.remove" | "record.move" => "Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state.",
        _ => return capabilities::find(id).map(|c|c.description).unwrap_or("Typed structured document operation; discover targets and parameters in params_schema.").into(),
    }.into()
}
