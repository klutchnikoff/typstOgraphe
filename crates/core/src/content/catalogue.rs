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
    pub legacy: bool,
    pub description: &'static str,
}
macro_rules! capability {
    ($id:literal,$label:literal,$icon:literal,$group:literal,$node:expr,$mark:expr,$letter:expr,$legacy:expr,$description:literal) => {
        Capability {
            id: $id,
            label_key: $label,
            icon: $icon,
            group: $group,
            node: $node,
            mark: $mark,
            letter_only: $letter,
            legacy: $legacy,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        true,
        "Internal image, optional editable caption and alternative text. No external URL is loaded."
    ),
    capability!(
        "insert.table",
        "cmd-insert-table",
        "table-2",
        "insert",
        Some("table"),
        None,
        false,
        false,
        "Rectangular table, 1–12 columns and 1–100 rows. No merged cells; header row optional."
    ),
    capability!(
        "insert.footnote",
        "cmd-insert-footnote",
        "footprints",
        "insert",
        Some("footnote"),
        None,
        false,
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
        true,
        "Enclosure mention after the signature. Does not embed the mentioned file."
    ),
];
pub fn for_type(kind: DocumentType) -> impl Iterator<Item = &'static Capability> {
    ALL.iter()
        .filter(move |c| !c.letter_only || kind == DocumentType::Courrier)
}
pub fn find(id: &str) -> Option<&'static Capability> {
    ALL.iter().find(|c| c.id == id)
}
pub fn describe(id: &str) -> Option<Value> {
    let c = find(id)?;
    Some(json!({"capability":c,"parameters":parameters(id),
        "implementations":{"structured_web":true,"legacy_engine":c.legacy,"legacy_mcp":c.legacy},
        "guidance":"Semantics are shared. Availability and execution are adapter-specific: documentation alone never grants execution. Models change presentation, not the capability contract."}))
}
pub fn parameters(id: &str) -> Value {
    match id {
        "insert.table" => {
            json!({"type":"object","properties":{"rows":{"type":"integer","minimum":1,"maximum":100,"default":3},"columns":{"type":"integer","minimum":1,"maximum":12,"default":2},"header":{"type":"boolean","default":true}},"additionalProperties":false})
        }
        "insert.footnote" => {
            json!({"type":"object","properties":{"text":{"type":"string","default":""}},"additionalProperties":false})
        }
        "block.quote" => {
            json!({"type":"object","properties":{"attribution":{"type":"string","default":""}},"additionalProperties":false})
        }
        "insert.image" => {
            json!({"type":"object","properties":{"src":{"type":"string"},"alt":{"type":"string","default":""}},"required":["src"],"additionalProperties":false})
        }
        "block.numbered" => {
            json!({"type":"object","properties":{"start":{"type":"integer","minimum":1,"default":1}},"additionalProperties":false})
        }
        _ => json!({"type":"object","additionalProperties":false}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiers_and_contracts_are_unique_and_type_specific() {
        let mut ids = std::collections::HashSet::new();
        for c in ALL {
            assert!(ids.insert(c.id));
            assert!(c.id.is_ascii());
            assert!(describe(c.id).is_some());
            if c.legacy {
                assert!(
                    crate::engine::catalogue()
                        .iter()
                        .any(|spec| spec.id.as_str() == c.id)
                );
            }
        }
        assert!(!for_type(DocumentType::CompteRendu).any(|c| c.letter_only));
        assert!(for_type(DocumentType::Courrier).any(|c| c.id == "insert.footnote"));
    }
}
