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
            schema.as_object_mut().expect("schema object").entry("properties").or_insert_with(|| json!({}));
            schema["properties"].as_object_mut().expect("properties").remove("guard");
            if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
                required.retain(|key| key != "guard");
            }
            add_property::<DocumentRef>(&mut schema,"document");
            add_property::<Target>(&mut schema,"selector");
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
fn extra<P: JsonSchema>(name: &'static str, description: &'static str) -> Tool {
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
