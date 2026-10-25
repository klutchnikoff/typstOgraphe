use super::{policy, types::*};
use crate::args::Target;
use rmcp::model::{Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde_json::{Map, Value, json};
use std::sync::{Arc, OnceLock};
use typstographe_core::engine::*;

pub fn tools() -> &'static [Tool] {
    static TOOLS: OnceLock<Vec<Tool>> = OnceLock::new();
    TOOLS.get_or_init(|| {
        let mut tools: Vec<_> = catalogue().iter().filter(|s| policy::exposed(s)).map(|spec| {
            let mut schema = spec.params_schema.clone();
            if spec.id.as_str() == "document.batch" {
                // Le cœur garde Call.params ouvert (Value) ; MCP accepte déjà
                // uniquement des objets, avant de résoudre les sélecteurs.
                schema["$defs"]["Call"]["properties"]["params"] = json!({
                    "type":"object", "additionalProperties":true,
                    "description":"Parameters of the named editing tool, including its optional selector; omit document and guard inside steps. See capabilities.describe for that tool."
                });
            }
            schema.as_object_mut().expect("schema object").entry("properties").or_insert_with(|| json!({}));
            schema["properties"].as_object_mut().expect("properties").remove("guard");
            if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
                required.retain(|key| key != "guard");
            }
            add_property::<DocumentRef>(&mut schema,"document");
            let selector = supports_selector(spec);
            if selector { add_property::<Target>(&mut schema,"selector"); }
            // Le résolveur MCP peut fournir ces champs depuis un sélecteur,
            // et compiler le rendu si l’appelant ne l’a pas fourni.
            if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
                required.retain(|key| key != "render");
                if selector {
                    let resolved: Vec<_> = required.iter().filter(|k| matches!(k.as_str(), Some("target" | "issue" | "occurrence"))).cloned().collect();
                    required.retain(|key| !resolved.contains(key));
                    if !resolved.is_empty() { schema["oneOf"] = json!([{ "required":resolved }, { "required":["selector"] }]); }
                }
            }
            if selector {
                let conflicts: Vec<_> = ["target", "issue", "occurrence"].into_iter()
                    .filter(|key| !schema["properties"][key].is_null())
                    .map(|key| json!({"not":{"required":["selector",key]}})).collect();
                if !conflicts.is_empty() {schema["allOf"] = json!(conflicts);}
            }
            if needs_document(spec) {
                schema.as_object_mut().expect("object").entry("required")
                    .or_insert_with(|| json!([])).as_array_mut().expect("required").push(json!("document"));
            }
            let mut tool = Tool::new(spec.id.as_str().to_owned(), format!("{} Supply document from the latest response; guard is prepared by the adapter. See capabilities.describe for details. File paths require declared server roots; discover current permissions with document.inspect.", spec.description), object(schema));
            tool.annotations = Some(ToolAnnotations::new().read_only(spec.family == Family::Query).open_world(false));
            tool
        }).collect();
        tools.extend([
            extra::<DescribeType>("document_types.describe", "Describe a type: purpose, fields, required export fields, capabilities and brief visual model cards."),
            extra::<DescribeCapability>("capabilities.describe", "Describe a capability from the shared registry: typed parameters, defaults, examples, effects and prototype support."),
            extra::<RenderPage>("render.page", "Render one page as an inline PNG, without creating files. Page numbers start at 1; the document reference must be current."),
        ]);
        tools.sort_by(|a,b| a.name.cmp(&b.name));
        tools
    })
}
pub fn needs_document(spec: &CommandSpec) -> bool {
    spec.availability != AvailabilityRule::Always
}
pub(super) fn for_session(attached: bool) -> &'static [Tool] {
    if !attached {
        return tools();
    }
    static ATTACHED: OnceLock<Vec<Tool>> = OnceLock::new();
    ATTACHED.get_or_init(|| {
        let mut result = tools().to_vec();
        result.extend(super::proposals::tools());
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    })
}
pub(super) fn extra<P: JsonSchema>(name: &'static str, description: &'static str) -> Tool {
    let mut tool = Tool::new(
        name,
        description,
        object(serde_json::to_value(schemars::schema_for!(P)).expect("schema")),
    );
    tool.annotations = Some(ToolAnnotations::new().read_only(true).open_world(false));
    tool
}
fn object(schema: Value) -> Arc<Map<String, Value>> {
    Arc::new(schema.as_object().expect("schema object").clone())
}
fn add_property<P: JsonSchema>(schema: &mut Value, name: &str) {
    let mut added = serde_json::to_value(schemars::schema_for!(P)).expect("schema");
    if let Some(definitions) = added
        .as_object_mut()
        .expect("schema object")
        .remove("$defs")
    {
        let root = schema
            .as_object_mut()
            .expect("schema object")
            .entry("$defs")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .expect("definitions");
        for (key, value) in definitions.as_object().expect("definitions") {
            if let Some(previous) = root.insert(key.clone(), value.clone()) {
                assert_eq!(previous, *value, "shared schema definition");
            }
        }
    }
    added.as_object_mut().expect("object").remove("$schema");
    schema["properties"][name] = added;
}

pub fn supports_selector(spec: &CommandSpec) -> bool {
    ["target", "occurrence", "issue", "position"]
        .iter()
        .any(|key| !spec.params_schema["properties"][key].is_null())
}
