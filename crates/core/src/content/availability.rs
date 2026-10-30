//! Selection-level interaction policy. Opening a form does not validate its future
//! parameters: concrete edits still go through commands.available and preparation.
use super::{
    Node, catalogue,
    commands::{TextZone, TreeRoot},
    contract::State,
    editing::Selection,
};
use serde::Serialize;
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    Style,
    Link,
    Heading,
    Block,
    List,
    Inline,
    Table,
}

pub fn policy(id: &str) -> Policy {
    match id {
        "text.bold" | "text.italic" => Policy::Style,
        "text.link" => Policy::Link,
        "block.paragraph" | "block.heading_1" | "block.heading_2" | "block.heading_3" => {
            Policy::Heading
        }
        "block.bullet" | "block.numbered" | "block.description" => Policy::List,
        "insert.footnote" | "insert.citation" => Policy::Inline,
        "insert.table" => Policy::Table,
        _ => Policy::Block,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    NoSelection,
    UnsupportedDocumentType,
    TextRequired,
    SelectionRequired,
    BodyRequired,
    TopLevelRequired,
    CaretRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Availability {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
}
fn result(reason: Option<Reason>) -> Availability {
    Availability {
        enabled: reason.is_none(),
        reason,
    }
}
fn root<'a>(state: &'a State, value: &TreeRoot) -> Option<Cow<'a, Node>> {
    match value {
        TreeRoot::Body => Some(Cow::Borrowed(&state.document)),
        TreeRoot::Record {
            collection,
            id,
            field,
        } => {
            let metadata = serde_json::to_value(&state.settings.metadata).ok()?;
            let record = metadata["fields"][collection]
                .as_array()?
                .iter()
                .find(|r| r["id"] == *id)?;
            serde_json::from_value(record[field].clone())
                .ok()
                .map(Cow::Owned)
        }
    }
}
fn zone(zone: &TextZone) -> Option<(TreeRoot, &[usize])> {
    match zone {
        TextZone::Body { path } => Some((TreeRoot::Body, path)),
        TextZone::Record {
            collection,
            id,
            field,
            path,
        } if !path.is_empty() => Some((
            TreeRoot::Record {
                collection: collection.clone(),
                id: id.clone(),
                field: field.clone(),
            },
            path,
        )),
        _ => None,
    }
}
fn at<'a>(root: &'a Node, path: &[usize]) -> Option<&'a Node> {
    path.iter().try_fold(root, |n, i| n.content.get(*i))
}
fn inline(node: &Node) -> bool {
    matches!(
        node.kind.as_str(),
        "paragraph"
            | "heading"
            | "figure"
            | "tableCaption"
            | "descriptionTerm"
            | "descriptionDefinition"
            | "quoteAttribution"
            | "postscript"
            | "attachment"
    )
}
fn link_at(node: &Node, offset: u32) -> bool {
    let mut start = 0;
    node.content.iter().any(|n| {
        let end = start + n.inline_len() as u32;
        let matches = start <= offset && offset <= end && n.marks.iter().any(|m| m.kind == "link");
        start = end;
        matches
    })
}
struct Target<'a> {
    root: Cow<'a, Node>,
    body: bool,
    from: Vec<usize>,
    to: Vec<usize>,
    empty: bool,
    linked: bool,
    text: bool,
    inline_selection: bool,
}
impl<'a> Target<'a> {
    fn read(state: &'a State, selection: &Selection) -> Option<Self> {
        match selection {
            Selection::Text { anchor, head } => {
                let (a, path) = zone(&anchor.zone)?;
                let (b, end) = zone(&head.zone)?;
                if a != b {
                    return None;
                }
                let root = root(state, &a)?;
                let first = at(&root, path)?;
                let last = at(&root, end)?;
                if !inline(first) || !inline(last) {
                    return None;
                }
                let linked = path == end && link_at(first, anchor.offset);
                Some(Self {
                    body: a == TreeRoot::Body,
                    from: path.to_vec(),
                    to: end.to_vec(),
                    empty: anchor == head,
                    linked,
                    text: true,
                    inline_selection: path == end,
                    root,
                })
            }
            Selection::Node { target } => {
                let root = root(state, &target.root)?;
                let node = at(&root, &target.path)?;
                let inline_node = matches!(node.kind.as_str(), "footnote" | "citation");
                let mut path = target.path.clone();
                if inline_node {
                    path.pop();
                }
                Some(Self {
                    body: target.root == TreeRoot::Body,
                    from: path.clone(),
                    to: path,
                    empty: false,
                    linked: false,
                    text: inline_node,
                    inline_selection: inline_node,
                    root,
                })
            }
            Selection::All { root: value } => {
                let root = root(state, value)?;
                let last = root.content.len().checked_sub(1)?;
                Some(Self {
                    body: *value == TreeRoot::Body,
                    from: vec![0],
                    to: vec![last],
                    empty: false,
                    linked: false,
                    text: true,
                    inline_selection: false,
                    root,
                })
            }
            Selection::Cells { anchor, head } => {
                if anchor.root != head.root {
                    return None;
                }
                let root = root(state, &anchor.root)?;
                if !matches!(
                    at(&root, &anchor.path)?.kind.as_str(),
                    "tableCell" | "tableHeader"
                ) || !matches!(
                    at(&root, &head.path)?.kind.as_str(),
                    "tableCell" | "tableHeader"
                ) {
                    return None;
                }
                Some(Self {
                    body: anchor.root == TreeRoot::Body,
                    from: anchor.path.clone(),
                    to: head.path.clone(),
                    empty: false,
                    linked: false,
                    text: true,
                    inline_selection: false,
                    root,
                })
            }
        }
    }
    fn top_level(&self) -> bool {
        self.from.len() == 1 && self.to.len() == 1
    }
    fn paragraphs(&self) -> bool {
        self.top_level()
            && self.root.content[self.from[0].min(self.to[0])..=self.from[0].max(self.to[0])]
                .iter()
                .all(|node| matches!(node.kind.as_str(), "paragraph" | "heading"))
    }
    fn list(&self) -> bool {
        self.from.first() == self.to.first()
            && self.from.first().is_some_and(|i| {
                matches!(
                    self.root.content[*i].kind.as_str(),
                    "bulletList" | "orderedList" | "descriptionList"
                )
            })
    }
    fn reason(&self, policy: Policy) -> Option<Reason> {
        match policy {
            Policy::Style if !self.text => Some(Reason::TextRequired),
            Policy::Link if !self.text => Some(Reason::TextRequired),
            Policy::Link if self.empty && !self.linked => Some(Reason::SelectionRequired),
            Policy::Style | Policy::Link => None,
            _ if !self.body => Some(Reason::BodyRequired),
            Policy::Inline if !self.inline_selection => Some(Reason::TextRequired),
            Policy::Inline => None,
            Policy::List if self.list() => None,
            Policy::Heading if !self.text || !self.paragraphs() => Some(Reason::TopLevelRequired),
            Policy::Table if !self.paragraphs() => Some(Reason::TopLevelRequired),
            Policy::Table if !self.empty => Some(Reason::CaretRequired),
            Policy::Block | Policy::List if !self.top_level() => Some(Reason::TopLevelRequired),
            _ => None,
        }
    }
}
/// Read only projected nodes near the target: no Yrs cloning, Typst compilation,
/// publication, context mutation or extra IPC on caret movement.
pub fn for_selection(
    state: &State,
    selection: Option<&Selection>,
) -> BTreeMap<String, Availability> {
    let target = selection.and_then(|s| Target::read(state, s));
    catalogue::ALL
        .iter()
        .map(|c| {
            let reason = if !catalogue::for_type(state.settings.metadata.document_type())
                .any(|v| v.id == c.id)
            {
                Some(Reason::UnsupportedDocumentType)
            } else {
                target
                    .as_ref()
                    .map_or(Some(Reason::NoSelection), |t| t.reason(policy(c.id)))
            };
            (c.id.into(), result(reason))
        })
        .collect()
}
