use super::super::{catalogue as capabilities, commands};
use crate::document_types::{DocumentType, Metadata, discovery};
use serde_json::{Value, json};
pub fn type_description(kind: DocumentType) -> Value {
    let metadata = Metadata::blank(kind);
    json!({"id":kind,"can_create":kind.can_create(),"metadata":metadata,
        "definition":kind.definition().definition(),
        "values_schema":kind.definition().values_schema(),
        "profile":discovery::catalogue().types.iter().find(|p|p.runtime_type==Some(kind)).map(discovery::describe),
        "fields":metadata.field_values(),"structured_metadata":kind.structured_metadata(),
        "capabilities":capabilities::for_type(kind).map(|c|describe(c.id).unwrap()).collect::<Vec<_>>(),
        "models":kind.models().map(|m|json!({"id":m.model.id(),"name":m.name,"description":m.description,"keywords":m.keywords})).collect::<Vec<_>>()})
}
pub fn describe_type(id: &str) -> Option<Value> {
    let profile = discovery::find(id)?;
    Some(if let Some(kind) = profile.runtime_type {
        type_description(kind)
    } else {
        json!({"id":id,"can_create":false,"profile":discovery::describe(profile),"metadata":null,"capabilities":[],"models":[]})
    })
}
pub fn types() -> Value {
    json!({"types":DocumentType::ALL.map(type_description), "discovery":discovery::describe_all()})
}
const QUERIES: &[(&str, &str)] = &[
    (
        "bibliography.review_import",
        "Resolve explicit decisions against the frozen library fingerprint; return the reviewed plan without publication. No silent rebase of import decisions.",
    ),
    (
        "bibliography.form",
        "Project native record fields into simple editable controls, preserving unexposed metadata. The supplied record is a frozen draft base, never a write instruction.",
    ),
    (
        "bibliography.prepare_form",
        "Validate changed form controls against a frozen native record, then optionally change its native classification with profile. Returns the proposed native record and its derived editing profile, preserving inactive publication containers. Does not publish. Use bibliography.apply (replace) with the original record fingerprint for a prepared form, or create for a new notice; a sequence of private form changes still publishes one undoable edit.",
    ),
    (
        "bibliography.browse",
        "Search all project notices by title/authors with compact summaries, derived profile and completeness. No 80-item suggestion limit.",
    ),
    (
        "bibliography.usage",
        "Return typed tree targets of a notice’s citations in the document body, including widget text. Header profiles currently exclude citation objects.",
    ),
    (
        "bibliography.record",
        "Read one native record by stable id, its content fingerprint and completeness diagnostics. Use its fingerprint as the record guard for edit, rename or remove within bibliography.apply.",
    ),
    (
        "bibliography.inspect",
        "Read active native records with stable identities, editable keys and library fingerprint, plus immutable source summaries. Article-only. Use bibliography.apply for guarded, undoable changes; create-and-cite uses one atomic batch with inline.replace. Imports require the reviewed preparation fingerprint and explicit conflict decisions; no direct file write occurs.",
    ),
    (
        "references.list",
        "List cross-reference targets in document order with stable opaque IDs, kinds, titles and current paths. IDs survive title edits; paths are revision-scoped. Use inline.replace with a crossReference node (target, include_label, include_page). Labels and page formatting belong to the model; at least one component is required. Deleting a referenced target requires removing or retargeting its references in the same atomic batch.",
    ),
    (
        "versions.list",
        "List retained versions with their immutable IDs, creation dates and editable labels. Internal review references are excluded. Read history for index-guarded mutations.",
    ),
    (
        "versions.read",
        "Read one immutable version, including structured content, metadata and original image resources. Never changes the active document.",
    ),
    (
        "document_types.list",
        "List integrated document contracts and models, plus grouped product profiles. Discovery entries marked planned cannot be created; capability needs in profiles are intentions, not supported operations.",
    ),
    (
        "document_types.describe",
        "Describe one type by ID. Integrated types include actual metadata, values_schema, capabilities and models; types without an implemented contract return only a product profile. Always check can_create.",
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
        "Read metadata, fields and resolved model_options: choice (null = follow model), effective, regime, allowed/default and substituted warning. Language remains documentary data.",
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
        "bibliography.profiles",
        "Describe common editing profiles derived from native Hayagriva types and active parents. Profiles are never stored as independent classifications. Article-only; completeness recommendations are not mandatory bibliographic fields.",
    ),
    (
        "bibliography.prepare_import",
        "Prepare an in-memory BibLaTeX/Hayagriva import with stable proposal identities, source digest and conversion diagnostics. UTF-8 contents are supplied explicitly, never a disk path. Does not add active references, create a citation, publish, save or change undo history. Returned IDs identify candidates: reconciliation can keep an existing target ID. Inspect diagnostics and plan; pass the report fingerprint, library_fingerprint and explicit decisions to bibliography.apply, then read inspect/search for active citation targets.",
    ),
    (
        "bibliography.prepare_record",
        "Prepare a new native bibliographic notice with a proposed available key and nonblocking completeness diagnostics. Does not edit active project notices or create a citation. Native fields overlay the profile's starting structure; the returned profile is derived from the resulting native entry; unexposed fields are retained. Explicit occupied keys and invalid fields are refused. Publish through bibliography.apply (create) before using record_id in a citation, or do both in one atomic batch.",
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
        "versions.change",
        "Create, relabel or delete a retained version and atomically save the current confirmed project under its existing lock. Requires the latest versions.list history token. Does not change text, private input or undo history; cannot delete internal review references.",
    ),
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
    if let Some(schema) = super::bibliography::parameter_schema(id) {
        return schema;
    }
    let string = json!({"type":"string"});
    let mut defs = commands::schema()["$defs"].clone();
    let mut s = match id {
        "versions.change" => {
            return serde_json::to_value(schemars::schema_for!(
                crate::content::project::VersionChange
            ))
            .unwrap();
        }
        "versions.read" => object(json!({"id":{"type":"string","format":"uuid"}}), &["id"]),
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
    for entry in &mut entries {
        super::prune_schema(&mut entry["params_schema"]);
    }
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
    if let Some(schema) = super::bibliography::result_schema(id) {
        value["result_schema"] = schema;
        value["result_schema_scope"] = json!("query_outcome");
    }
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
        "bibliography.apply" => "Publish one guarded native-library change in memory, never save. expected is the library fingerprint from bibliography.inspect. Edit, replace, rename and remove also require the record fingerprint from bibliography.record. Edit profile changes native type/parents; no independent genre is stored. Replace accepts a prepared native record while preserving identity/provenance, including retained inactive parents. Import requires the reviewed preparation fingerprint and explicit same/distinct/skip decisions; unresolved plans are refused. Use one atomic batch with inline.replace for create-and-cite. Undo restores notices, citations and provenance together.",
        "text.insert" => "Insert/replace literal prose in one typed range; marks are explicit and default to none. No Typst/Markdown input rules.",
        "block.join" => "Join explicit adjacent left/right paragraphs or headings in the same container. Two nonempty paragraphs retain a single soft break at the seam (a Typst space); an existing break is reused. Joins involving headings concatenate directly. The nonempty left block keeps its kind and attributes. Remove an empty neighbor without converting its survivor. Widgets, cells and annotation boundaries are protected; the mandatory final editing slot is a no-op.",
        "list.item.split" => "Split a flat bullet/numbered list item at one paragraph range, or a description item at its definition range. Preserve the left item and prefix; the suffix and following paragraphs become a new item. Description terms remain separate, and the new term is empty. Empty-item exit is a separate operation.",
        "list.item.join" => "Join adjacent bullet/numbered items of the same list, concatenating the last left paragraph and first right paragraph without an added space or break. Keep the left item and prefix. An empty right item is removed. Refuse description term/definition boundaries and different lists.",
        "list.item.exit" => "Remove an entirely empty item and create/reuse a continuation paragraph after its list. Remove the whole list only when it was the last item. A badge is content: an item containing a note cannot be exited as empty.",
        "text.bold" | "text.italic" => "Apply or remove this style on a nonempty typed range (value on/off). Never changes future typing; use context.style for that.",
        "inline.replace" => "Replace one typed rich range with a footnote, citation, crossReference, editable mathInline, softBreak (a Typst space) or hardBreak (forced line break). Breaks are unavailable in single-line header fields and plain fields. A break is one UTF-16 unit; paragraph separation uses block.split. No implicit CR/LF interpretation in text.insert. Editable mathInline has nested plain text content: use text.insert on its own zone, not a parent offset inside its envelope. Formula delimiters are input conventions, not stored source. Discover objects via capabilities.describe.",
        "block.insert" | "block.replace" | "block.splice" => "Edit a guarded tree with explicit nodes (quote, list, figure, postscript…). Discover examples via capabilities.describe.",
        "resource.add" => "Admit immutable image bytes (base64), validated before use; a batch may add the resource then insert its figure or signature.",
        "document.set_model" => "Set model_id, full pure Typst source and YAML manifest together. Every option regime and supported language is validated. Retained choices survive model switches; metadata.get reports effective values and substitutions.",
        "metadata.set" => "Set a typed metadata, record field or document option. Values are validated against the type and model policy. Date fields accept null, {kind: calendar, iso: YYYY-MM-DD}, or {kind: custom, text: literal text}; never a raw string. Today must be resolved to a fixed local date by the caller. Discover documentary values_schema with document_types.describe.",
        "insert.table" => "Insert a table figure. header marks the first row as column headers; row_headers marks the first column as row headers. They are independent, including a one-cell table. No merged cells.",
        "table.header" => "Set a header axis on an existing tableFigure: axis columns (default) labels columns using the first row; axis rows labels rows using the first column. Text and other cells are preserved. Leading-track insertions and removals retain the chosen roles.",
        "record.add" | "record.remove" | "record.move" => "Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state.",
        _ => return capabilities::find(id).map(|c|c.description).unwrap_or("Typed structured document operation; discover targets and parameters in params_schema.").into(),
    }.into()
}
