//! Validated editorial content: paragraphs and inline styles, never arbitrary Typst.
use super::Node;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct RichText(Node);
impl Default for RichText {
    fn default() -> Self {
        Self::plain("")
    }
}
impl<'de> Deserialize<'de> for RichText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let node = Node::deserialize(deserializer)?;
        Self::new(node).map_err(serde::de::Error::custom)
    }
}
impl RichText {
    pub fn new(node: Node) -> Result<Self, String> {
        fn allowed(node: &Node, parent: &str) -> bool {
            match (parent, node.kind.as_str()) {
                ("", "doc") | ("doc", "paragraph") => {
                    node.text.is_none()
                        && node.marks.is_empty()
                        && node.content.iter().all(|child| allowed(child, &node.kind))
                }
                ("paragraph", "text") => node.content.is_empty(),
                ("paragraph", "hardBreak" | "softBreak") => {
                    node.content.is_empty() && node.text.is_none()
                }
                _ => false,
            }
        }
        if !allowed(&node, "") {
            return Err("Cette zone accepte uniquement des paragraphes, du gras, de l’italique et des liens.".into());
        }
        super::source(&node, &Default::default())?;
        Ok(Self(node))
    }
    // Convenience for internal literals and fixtures. External content uses new/Deserialize.
    pub(crate) fn plain(text: &str) -> Self {
        Self(serde_json::from_value(serde_json::json!({"type":"doc","content":[{
            "type":"paragraph","content": if text.is_empty() {vec![]} else {vec![serde_json::json!({"type":"text","text":text})]}
        }]})).expect("plain editorial text"))
    }
    pub fn is_empty(&self) -> bool {
        self.0.content.iter().all(|p| {
            p.content
                .iter()
                .all(|t| t.text.as_ref().is_none_or(|s| s.trim().is_empty()))
        })
    }
    pub fn is_inline(&self) -> bool {
        self.0.content.len() <= 1
    }
    pub fn source(&self, inline: bool) -> String {
        if self.is_empty() {
            return String::new();
        }
        if inline {
            self.0
                .content
                .iter()
                .map(|p| {
                    super::schema::inline(&p.content, false).expect("validated editorial content")
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            super::source(&self.0, &Default::default()).expect("validated editorial content")
        }
    }
}
