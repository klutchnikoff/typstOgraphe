//! Progressive MCP views derived from the shared registries, never a second contract.
use serde_json::{Value, json};
use typstographe_core::{
    content::{api, catalogue},
    document_types::{DocumentType, discovery as profiles},
};

fn model_cards(kind: DocumentType) -> Vec<Value> {
    kind.models()
        .map(|model| {
            json!({
                "id":model.model.id(), "name":model.name,
                "description":model.description, "keywords":model.keywords
            })
        })
        .collect()
}

pub fn types() -> Value {
    json!({"types":DocumentType::ALL.map(|kind| {
        let profile = profiles::catalogue().types.iter().find(|p|p.runtime_type==Some(kind));
        json!({"id":kind,"name":profile.map(|p|p.name.as_str()),
            "description":profile.map(|p|p.description.as_str()),"can_create":kind.can_create(),
            "capability_count":catalogue::for_type(kind).count(),"models":model_cards(kind)})
    }), "discovery":profiles::describe_all(),
    "details":"Use document_types.describe with a type ID for fields and capability summaries; capabilities.describe with one capability ID for signatures, constraints and examples."})
}

pub fn capabilities(kind: Option<DocumentType>) -> Value {
    let entries: Vec<_> = catalogue::ALL
        .iter()
        .filter(|cap| {
            kind.is_none_or(|kind| catalogue::for_type(kind).any(|item| item.id == cap.id))
        })
        .map(|cap| json!({"id":cap.id,"group":cap.group,"description":cap.description}))
        .collect();
    json!({"document_type":kind,"entries":entries,
        "details":"Use capabilities.describe with one ID. Type capabilities do not grant adapter permissions or guarantee that a particular edit is currently available."})
}

pub fn describe_type(id: &str) -> Option<Value> {
    let mut value = api::describe_type(id)?;
    if let Some(kind) = profiles::find(id)?.runtime_type {
        value["capabilities"] = capabilities(Some(kind))["entries"].clone();
    }
    value["details"] = json!(
        "Use capabilities.describe for one capability's signature and executable examples. Models are interchangeable visual presentations."
    );
    Some(value)
}

fn execution(id: &str, attached: bool, batch_step: bool) -> Value {
    json!({"direct_tool":!attached || crate::attached_support(id),
        "proposal_step":attached && batch_step && crate::attached_id(id)})
}

pub fn commands(attached: bool) -> Value {
    let mut index = api::catalogue();
    for entry in index["entries"].as_array_mut().unwrap() {
        entry.as_object_mut().unwrap().remove("params_schema");
        entry["execution"] = execution(
            entry["id"].as_str().unwrap(),
            attached,
            entry["batch_step"] == true,
        );
    }
    let entries = index["entries"].as_array_mut().unwrap();
    for tool in crate::tools(attached) {
        if !entries
            .iter()
            .any(|entry| entry["id"] == tool.name.as_ref())
        {
            entries.push(json!({"id":tool.name,"family":"mcp","batch_step":false,
                "description":tool.description,"execution":{"direct_tool":true,"proposal_step":false}}));
        }
    }
    entries.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    index["details"] = json!(
        "Use capabilities.describe with one ID for its schema and examples. execution describes adapter support, not current permissions or edit availability."
    );
    index
}

pub fn describe(id: &str, attached: bool) -> Option<Value> {
    let tool = crate::tools(attached)
        .into_iter()
        .find(|tool| tool.name == id);
    let mut value = api::describe(id).or_else(|| {
        tool.as_ref().map(|tool|json!({
        "id":id,"family":"mcp","batch_step":false,"description":tool.description,"examples":[]
    }))
    })?;
    value["id"] = json!(id);
    // Semantic IDs describe a feature, not an invocable operation. Their
    // canonical examples point to the operation IDs whose support can be read.
    if value.get("family").is_some() {
        let mut support = execution(id, attached, value["batch_step"] == true);
        support["direct_tool"] = json!(tool.is_some());
        value["execution"] = support;
    }
    if let Some(tool) = tool {
        value["params_schema"] = json!(tool.input_schema);
    }
    for key in ["params_schema", "parameters", "result_schema"] {
        if let Some(schema) = value.get_mut(key) {
            prune_schema(schema);
        }
    }
    Some(value)
}

pub fn overview(attached: bool, kind: Option<DocumentType>) -> Value {
    json!({"mode":if attached {"attached"}else{"autonomous"},
        "purpose":"Write structured documents without Typst code; the visual model controls presentation.",
        "document_open":kind.is_some(),"document_type":kind,
        "capabilities":kind.map(|kind|capabilities(Some(kind))["entries"].clone()),
        "rules":if attached {
            "Read published content. Submit body edits as proposals; only the human can decide. Metadata, resources and files are read-only. Use the outer permissions and latest document reference."
        } else {
            "Edits stay in memory until explicit save. Files require configured read/write roots. Export is separate. Use the outer permissions and latest document reference."
        },
        "next":{"types":"document_types.list","type_details":{"tool":"document_types.describe","argument":"type"},
            "capability_details":{"tool":"capabilities.describe","argument":"id"},
            "operations":"commands.list","editing_guide":"typstographe://guide",
            "content":"document.inspect"}})
}

pub use typstographe_core::content::api::prune_schema;
