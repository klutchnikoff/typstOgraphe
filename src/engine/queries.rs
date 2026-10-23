use super::*;
use crate::document::structure::{BlockContent, TextZone};
use serde_json::json;

impl Engine {
    pub fn query(&self, query: Query) -> Result<Value, CommandError> {
        Ok(match query {
            Query::SpellingIssues(_) => {
                serde_json::to_value(self.spelling_issues()?).expect("serializable spelling issues")
            }
            Query::SpellingSuggestions(p) => self.spelling_suggestions(p)?,
            Query::Inspect(_) => json!({
                "guard": self.session.target_guard(),
                "selection": self.session.selection_target(self.selection).ok(),
                "dirty": self.session.is_dirty(),
                "can_undo": !self.session.undo.is_empty(),
                "can_redo": !self.session.redo.is_empty(),
            }),
            Query::Text(params) => {
                let range = self.resolve_query_target(params.target)?.range();
                let text: String = self
                    .session
                    .document
                    .view
                    .text
                    .chars()
                    .skip(range.start)
                    .take(range.len())
                    .filter(|c| *c != crate::document::IMAGE_OBJECT)
                    .collect();
                json!({"guard": self.session.target_guard(), "text": text})
            }
            Query::Structure(_) => self.structure_json(),
            Query::Metadata(_) => {
                json!({"guard": self.session.target_guard(), "metadata": self.session.metadata})
            }
            Query::Commands(_) => json!({"entries": catalogue()}),
            Query::Find(params) => serde_json::to_value(self.search_results(params)?)
                .expect("serializable search results"),
        })
    }
    fn structure_json(&self) -> Value {
        fn zone(zone: &TextZone, role: &str) -> Value {
            json!({"id": zone.id, "role": role, "text": zone.view.text})
        }
        let blocks: Vec<_> = self
            .session
            .document
            .structure
            .blocks
            .iter()
            .map(|block| {
                let (kind, zones) = match &block.content {
                    BlockContent::Text { kind, body } => {
                        let name = match kind {
                            crate::document::BlockKind::Heading(1) => "heading_1",
                            crate::document::BlockKind::Heading(2) => "heading_2",
                            crate::document::BlockKind::Heading(_) => "heading_3",
                            crate::document::BlockKind::Bullet => "bullet",
                            crate::document::BlockKind::Numbered => "numbered",
                            crate::document::BlockKind::Postscript => "postscript",
                            crate::document::BlockKind::Attachment => "attachment",
                            _ => "paragraph",
                        };
                        (name, vec![zone(body, "body")])
                    }
                    BlockContent::Description { term, definition } => (
                        "description",
                        vec![zone(term, "term"), zone(definition, "definition")],
                    ),
                    BlockContent::Quote { body, attribution } => (
                        "quote",
                        vec![zone(body, "body"), zone(attribution, "attribution")],
                    ),
                    BlockContent::Image { caption, .. } => {
                        ("image", vec![zone(caption, "caption")])
                    }
                };
                let mut value = json!({"id": block.id, "kind": kind, "zones": zones});
                if let BlockContent::Image { image, .. } = &block.content {
                    value["image"] = json!({"path": image.path, "alt": image.alt});
                }
                value
            })
            .collect();
        json!({"guard": self.session.target_guard(), "blocks": blocks})
    }
}
