use super::types::DocumentRef;
use rmcp::model::{Tool, ToolAnnotations};
use serde_json::json;
use std::sync::Arc;
use typstographe_core::content::api;
pub fn tools(attached: bool) -> Vec<Tool> {
    let mut entries:Vec<_>=api::catalogue()["entries"].as_array().unwrap().iter().filter(|s|!attached||attached_support(s["id"].as_str().unwrap())).map(|spec|{
        let id=spec["id"].as_str().unwrap();let mut schema=spec["params_schema"].clone();
        let reference=serde_json::to_value(schemars::schema_for!(DocumentRef)).unwrap();
        for (key,value) in reference["$defs"].as_object().unwrap(){schema["$defs"][key]=value.clone();}
        schema["properties"]["document"]=json!({"$ref":"#/$defs/DocumentRef"});
        let mut doc=reference;doc.as_object_mut().unwrap().remove("$schema");doc.as_object_mut().unwrap().remove("$defs");schema["$defs"]["DocumentRef"]=doc;
        if !matches!(id,"document.inspect"|"document.new"|"document.open"|"commands.list"|"document_types.list"|"document_types.describe"|"capabilities.describe"|"spelling.suggestions") {schema.as_object_mut().unwrap().entry("required").or_insert_with(||json!([])).as_array_mut().unwrap().push(json!("document"));}
        let mut tool=Tool::new(id.to_owned(),format!("{} Pass the latest outer document reference to document calls. See capabilities.describe. Attached edits require a human-reviewed proposal; the human may accept independent groups in the shared comparator.",spec["description"].as_str().unwrap()),Arc::new(schema.as_object().unwrap().clone()));
        tool.annotations=Some(ToolAnnotations::new().read_only(api::is_query(id)).open_world(false));tool
    }).collect();
    if attached {
        let defs = serde_json::to_value(schemars::schema_for!(DocumentRef)).unwrap();
        for (name, description, properties, required) in [
            (
                "proposals.submit",
                "Prepare a body-editing lot; only the human can accept it. One pending lot, idempotent request_id. Header/resources/files are not editable in attached mode yet.",
                json!({"document":{"$ref":"#/$defs/DocumentRef"},"request_id":{"type":"string","minLength":1,"maxLength":128},"summary":{"type":"string","maxLength":2048},"steps":api::describe("document.batch").unwrap()["params_schema"]["properties"]["steps"]}),
                json!(["document", "request_id", "steps"]),
            ),
            (
                "proposals.get",
                "Read proposal status; return control to the human after submitting, do not poll.",
                json!({"proposal":{"type":"string"}}),
                json!(["proposal"]),
            ),
            (
                "proposals.cancel",
                "Cancel your own pending proposal. Never approves or publishes.",
                json!({"proposal":{"type":"string"}}),
                json!(["proposal"]),
            ),
        ] {
            let mut schema = json!({"type":"object","properties":properties,"required":required,"additionalProperties":false,"$defs":defs["$defs"]});
            let mut d = defs.clone();
            d.as_object_mut().unwrap().remove("$defs");
            d.as_object_mut().unwrap().remove("$schema");
            schema["$defs"]["DocumentRef"] = d;
            if name == "proposals.submit" {
                schema["properties"]["steps"]["maxItems"] = json!(128);
            }
            let mut tool = Tool::new(
                name,
                description,
                Arc::new(schema.as_object().unwrap().clone()),
            );
            tool.annotations = Some(
                ToolAnnotations::new()
                    .read_only(name == "proposals.get")
                    .open_world(false),
            );
            entries.push(tool);
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

pub fn attached_support(id: &str) -> bool {
    (api::is_query(id)
        && !id.starts_with("render.")
        && !id.starts_with("versions.")
        && id != "commands.available")
        || id == "context.set"
}
pub fn attached_id(id: &str) -> bool {
    matches!(
        id,
        "text.insert"
            | "text.delete"
            | "text.bold"
            | "text.italic"
            | "text.link"
            | "text.remove_link"
            | "text.clear"
            | "inline.replace"
            | "block.insert"
            | "block.replace"
            | "block.splice"
            | "node.set_attrs"
            | "block.remove"
            | "block.split"
            | "block.join"
            | "list.item.split"
            | "list.item.join"
            | "list.item.exit"
            | "block.paragraph"
            | "block.heading_1"
            | "block.heading_2"
            | "block.heading_3"
            | "insert.table"
            | "table.row.insert"
            | "table.row.remove"
            | "table.column.insert"
            | "table.column.remove"
            | "table.header"
    )
}
pub fn attached_step(call: &api::Call) -> bool {
    if !attached_id(&call.id) {
        return false;
    }
    match serde_json::from_value(serde_json::to_value(call).unwrap()) {
        Ok(command) => attached_command(&command),
        Err(_) => call.params.get("selector").is_some(),
    }
}
pub fn attached_command(command: &typstographe_core::content::commands::Command) -> bool {
    use typstographe_core::content::commands::*;
    match command {
        Command::JoinBlocks { left, right } | Command::JoinListItems { left, right } => {
            left.root == TreeRoot::Body && right.root == TreeRoot::Body
        }
        Command::Insert { target, .. }
        | Command::Delete { target }
        | Command::Bold { target, .. }
        | Command::Italic { target, .. }
        | Command::Link { target, .. }
        | Command::RemoveLink { target }
        | Command::Clear { target }
        | Command::ReplaceInline { target, .. }
        | Command::SplitListItem { target }
        | Command::SplitBlock { target } => matches!(
            target.zone,
            TextZone::Body { .. } | TextZone::Annotation { .. }
        ),
        Command::InsertBlock { parent, .. }
        | Command::SpliceBlocks { parent, .. }
        | Command::InsertTable { parent, .. } => parent.root == TreeRoot::Body,
        Command::ReplaceBlock { target, .. }
        | Command::ExitListItem { target }
        | Command::SetNodeAttrs { target, .. }
        | Command::RemoveBlock { target }
        | Command::Paragraph { target }
        | Command::Heading1 { target }
        | Command::Heading2 { target }
        | Command::Heading3 { target }
        | Command::InsertTableRow { target, .. }
        | Command::RemoveTableRow { target, .. }
        | Command::InsertTableColumn { target, .. }
        | Command::RemoveTableColumn { target, .. }
        | Command::TableHeader { target, .. } => target.root == TreeRoot::Body,
        _ => false,
    }
}
