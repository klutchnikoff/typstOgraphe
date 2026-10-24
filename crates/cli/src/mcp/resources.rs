use super::INSTRUCTIONS;
use rmcp::model::Resource;
use serde_json::json;
use typstographe_core::{
    document_types::DocumentType,
    engine::{Engine, catalogue},
};

pub fn list() -> Vec<Resource> {
    [
        ("guide", "Usage guide"),
        ("types", "Document types and brief model cards"),
        ("commands", "Command catalogue"),
    ]
    .into_iter()
    .map(|(id, name)| {
        Resource::new(format!("typstographe://{id}"), name).with_mime_type("application/json")
    })
    .collect()
}
pub fn read(uri: &str) -> Option<String> {
    let value = match uri {
        "typstographe://guide" => {
            json!({"instructions":INSTRUCTIONS,"session":"autonomous","permissions":"Call document.inspect and read the outer permissions object. No roots grants no document file access."})
        }
        "typstographe://types" => {
            json!({"types":DocumentType::ALL.map(Engine::type_documentation)})
        }
        "typstographe://commands" => json!({"entries":catalogue()}),
        _ => return None,
    };
    Some(serde_json::to_string_pretty(&value).expect("resource"))
}
