//! Contrat des capacités. Chaque adaptateur annonce séparément ce qu'il exécute.
use crate::document_types::DocumentType;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Copy, Serialize)]
pub struct Capability {
    pub id: &'static str,
    pub label_key: &'static str,
    pub icon: &'static str,
    pub group: &'static str,
    pub node: Option<&'static str>,
    pub mark: Option<&'static str>,
    pub letter_only: bool,
    pub description: &'static str,
}
macro_rules! capability {
    ($id:literal,$label:literal,$icon:literal,$group:literal,$node:expr,$mark:expr,$letter:expr,$description:literal) => {
        Capability {
            id: $id,
            label_key: $label,
            icon: $icon,
            group: $group,
            node: $node,
            mark: $mark,
            letter_only: $letter,
            description: $description,
        }
    };
}
pub const ALL: &[Capability] = &[
    capability!(
        "block.paragraph",
        "cmd-paragraph",
        "pilcrow",
        "headings",
        Some("paragraph"),
        None,
        false,
        "Ordinary prose. A soft return stays inside the paragraph; two returns create a paragraph."
    ),
    capability!(
        "block.heading_1",
        "cmd-heading-1",
        "heading-1",
        "headings",
        Some("heading"),
        None,
        false,
        "Level-one heading; final presentation belongs to the Typst model."
    ),
    capability!(
        "block.heading_2",
        "cmd-heading-2",
        "heading-2",
        "headings",
        Some("heading"),
        None,
        false,
        "Level-two heading."
    ),
    capability!(
        "block.heading_3",
        "cmd-heading-3",
        "heading-3",
        "headings",
        Some("heading"),
        None,
        false,
        "Level-three heading."
    ),
    capability!(
        "text.bold",
        "cmd-bold",
        "bold",
        "inline",
        None,
        Some("bold"),
        false,
        "Bold selection or subsequent typing. Disable explicitly to stop subsequent bold text."
    ),
    capability!(
        "text.italic",
        "cmd-italic",
        "italic",
        "inline",
        None,
        Some("italic"),
        false,
        "Emphasize selection or subsequent typing. Disable explicitly to stop subsequent emphasis."
    ),
    capability!(
        "text.link",
        "cmd-link",
        "link",
        "inline",
        None,
        Some("link"),
        false,
        "Editable label and http, https or mailto URL. Never fetch the URL."
    ),
    capability!(
        "block.bullet",
        "cmd-bullet",
        "list",
        "lists",
        Some("bulletList"),
        None,
        false,
        "Unordered list. The model chooses the marker; initially no nesting."
    ),
    capability!(
        "block.numbered",
        "cmd-numbered",
        "list-ordered",
        "lists",
        Some("orderedList"),
        None,
        false,
        "Ordered list with optional initial number; initially no nesting."
    ),
    capability!(
        "block.description",
        "cmd-description",
        "list-collapse",
        "lists",
        Some("descriptionList"),
        None,
        false,
        "Term and definition zones; Tab moves to the definition."
    ),
    capability!(
        "block.quote",
        "cmd-quote",
        "quote",
        "insert",
        Some("quote"),
        None,
        false,
        "Block quote with optional editable attribution; final appearance belongs to the model."
    ),
    capability!(
        "insert.image",
        "cmd-insert-image",
        "image",
        "insert",
        Some("figure"),
        None,
        false,
        "Internal image, optional editable caption and alternative text. No external URL is loaded."
    ),
    capability!(
        "insert.table",
        "cmd-insert-table",
        "table-2",
        "insert",
        Some("tableFigure"),
        None,
        false,
        "Table figure with an optional rich caption, 1–12 columns and 1–100 rows. No merged cells; header row optional."
    ),
    capability!(
        "insert.citation",
        "cmd-insert-citation",
        "book-open",
        "insert",
        Some("citation"),
        None,
        false,
        "Article-only bibliographic citation. Choose references from the published project metadata; items use file_id and key, with optional literal supplement. Rendering belongs to Typst and the model."
    ),
    capability!(
        "insert.footnote",
        "cmd-insert-footnote",
        "footprints",
        "insert",
        Some("footnote"),
        None,
        false,
        "Inline reference with editable plain-text content; numbering and placement belong to Typst."
    ),
    capability!(
        "letter.insert_postscript",
        "cmd-insert-postscript",
        "message-square",
        "letter",
        Some("postscript"),
        None,
        true,
        "Postscript collected and rendered after the signature by the letter model."
    ),
    capability!(
        "letter.insert_attachment",
        "cmd-insert-attachment",
        "paperclip",
        "letter",
        Some("attachment"),
        None,
        true,
        "Enclosure mention after the signature. Does not embed the mentioned file."
    ),
];
pub fn for_type(kind: DocumentType) -> impl Iterator<Item = &'static Capability> {
    ALL.iter().filter(move |c| {
        (!c.letter_only || kind == DocumentType::Courrier)
            && (c.id != "insert.citation" || kind == DocumentType::Article)
    })
}
/// The type's capability catalogue is an execution boundary, not only a UI filter.
pub(crate) fn validate_document(
    kind: DocumentType,
    node: &super::Node,
) -> Result<(), super::contract::Error> {
    if let Some(capability) = ALL.iter().find(|c| c.node == Some(node.kind.as_str()))
        && !for_type(kind).any(|c| c.node == Some(node.kind.as_str()))
    {
        return Err(super::contract::Error::CommandUnavailable {
            command: capability.id.into(),
            reason: super::contract::UnavailableReason::UnsupportedDocumentType,
        });
    }
    for child in &node.content {
        validate_document(kind, child)?;
    }
    Ok(())
}

pub fn find(id: &str) -> Option<&'static Capability> {
    ALL.iter().find(|c| c.id == id)
}
pub fn describe(id: &str) -> Option<Value> {
    let c = find(id)?;
    Some(
        json!({"capability":c,"parameters":parameters(id),"editor":editor(id),
        "guidance":"Semantics and selection-level availability are shared (context.get availability). Form parameters still require concrete commands.available validation; adapter permissions remain separate. Models change presentation, not the capability contract."}),
    )
}
pub fn parameters(id: &str) -> Value {
    match id {
        "text.link" => {
            json!({"type":"object","properties":{"url":{"type":"string","pattern":"^(https?://|mailto:).+","x-editor":{"placement":"panel","attribute":"href","label_key":"web-link-address"}}},"required":["url"],"additionalProperties":false})
        }
        "insert.table" => {
            json!({"type":"object","properties":{"rows":{"type":"integer","minimum":1,"maximum":100,"default":3,"x-editor":{"placement":"structure"}},"columns":{"type":"integer","minimum":1,"maximum":12,"default":2,"x-editor":{"placement":"structure"}},"header":{"type":"boolean","default":true,"x-editor":{"placement":"creation"}}},"additionalProperties":false})
        }
        "insert.citation" => {
            let mut schema =
                serde_json::to_value(schemars::schema_for!(crate::bibliography::Citation)).unwrap();
            schema["properties"]["items"]["x-editor"] = json!({"placement":"body"});
            schema["properties"]["form"] = json!({"type":"string","enum":["normal","prose"],"default":"normal","x-editor":{"label_key":"web-citation-form"}});
            schema
        }
        "insert.footnote" => {
            json!({"type":"object","properties":{"text":{"type":"string","default":"","x-editor":{"label_key":"web-footnote-text","multiline":true,"label_visibility":"accessible"}}},"additionalProperties":false})
        }
        "block.quote" => {
            json!({"type":"object","properties":{"attribution":{"type":"string","default":"","x-editor":{"placement":"body"}}},"additionalProperties":false})
        }
        "insert.image" => {
            json!({"type":"object","properties":{"src":{"type":"string","x-editor":{"placement":"panel","control":"image"}},"alt":{"type":"string","default":"","x-editor":{"label_key":"web-alt-text"}}},"required":["src"],"additionalProperties":false})
        }
        "block.numbered" => {
            json!({"type":"object","properties":{"start":{"type":"integer","minimum":1,"default":1,"x-editor":{"label_key":"web-list-start"}}},"additionalProperties":false})
        }
        _ => json!({"type":"object","properties":{},"additionalProperties":false}),
    }
}

/// Presentation hints complement the signature; they never authorize an edit.
/// Optional parameters default to panel controls. Body/structure/creation bindings
/// prevent duplicating controls already provided elsewhere by the editor.
pub fn editor(id: &str) -> Value {
    let mut value = editor_hints(id);
    value["availability"] = json!(super::availability::policy(id));
    value
}
fn editor_hints(id: &str) -> Value {
    if id == "insert.table" {
        return json!({"actions":[
            {"id":"table.row.remove","label_key":"web-table-row-remove","command":"table.row.remove","axis":"row","offset":0,"remove":true},
            {"id":"table.row.before","label_key":"web-table-row-before","command":"table.row.insert","axis":"row","offset":0,"remove":false},
            {"id":"table.row.after","label_key":"web-table-row-after","command":"table.row.insert","axis":"row","offset":1,"remove":false},
            {"id":"table.column.remove","label_key":"web-table-column-remove","command":"table.column.remove","axis":"column","offset":0,"remove":true},
            {"id":"table.column.before","label_key":"web-table-column-before","command":"table.column.insert","axis":"column","offset":0,"remove":false},
            {"id":"table.column.after","label_key":"web-table-column-after","command":"table.column.insert","axis":"column","offset":1,"remove":false}
        ]});
    }
    find(id)
        .filter(|capability| capability.group == "lists")
        .map(|capability| json!({"conversion_group":capability.group}))
        .unwrap_or_else(|| json!({}))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn identifiers_and_contracts_are_unique_and_type_specific() {
        let mut ids = std::collections::HashSet::new();
        for c in ALL {
            assert!(ids.insert(c.id));
            assert!(c.id.is_ascii());
            assert!(describe(c.id).is_some());
            assert_eq!(
                crate::content::api::describe(c.id).unwrap()["capability"]["id"],
                c.id
            );
        }
        assert!(!for_type(DocumentType::CompteRendu).any(|c| c.letter_only));
        assert!(for_type(DocumentType::Courrier).any(|c| c.id == "insert.footnote"));
    }

    #[test]
    fn editor_bindings_are_part_of_the_shared_contract() {
        for capability in ALL {
            let description = describe(capability.id).unwrap();
            assert_eq!(description["parameters"], parameters(capability.id));
            assert_eq!(description["editor"], editor(capability.id));
            let schema = parameters(capability.id);
            for parameter in schema["properties"].as_object().unwrap().values() {
                if let Some(placement) = parameter["x-editor"]["placement"].as_str() {
                    assert!(["body", "structure", "creation", "panel"].contains(&placement));
                }
            }
        }
        assert_eq!(
            parameters("block.quote")["properties"]["attribution"]["x-editor"]["placement"],
            "body"
        );
        assert_eq!(
            parameters("insert.table")["properties"]["rows"]["x-editor"]["placement"],
            "structure"
        );
        assert_eq!(
            parameters("text.link")["properties"]["url"]["x-editor"]["attribute"],
            "href"
        );
        assert_eq!(editor("block.bullet"), editor("block.numbered"));
        assert_eq!(editor("block.bullet"), editor("block.description"));
        assert_eq!(editor("block.quote"), json!({"availability":"block"}));
        let link = crate::content::api::describe("text.link").unwrap();
        assert_eq!(
            link["params_schema"]["properties"]["url"]["type"],
            parameters("text.link")["properties"]["url"]["type"]
        );
        assert!(
            link["params_schema"]["required"]
                .as_array()
                .unwrap()
                .contains(&json!("url"))
        );
    }
}
