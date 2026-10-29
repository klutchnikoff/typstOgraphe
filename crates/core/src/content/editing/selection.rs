//! Public view selections preserve direction and structural selections. CRDT
//! identities/relative positions stay private and never leak into adapters.
use super::anchors::Range;
use crate::content::{
    commands::{TextTarget, TextZone, TreeRoot, TreeTarget},
    contract::{Error, State, TargetReason},
    shared::{SharedDocument, structure, targets},
};
use serde::{Deserialize, Serialize};
use yrs::{Assoc, StickyIndex, Transact};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextPoint {
    pub zone: TextZone,
    pub offset: u32,
}
impl TextPoint {
    fn target(&self) -> TextTarget {
        TextTarget {
            zone: self.zone.clone(),
            from: self.offset,
            to: self.offset,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    Text {
        anchor: TextPoint,
        head: TextPoint,
    },
    Node {
        target: TreeTarget,
    },
    Cells {
        anchor: TreeTarget,
        head: TreeTarget,
    },
    All {
        root: TreeRoot,
    },
}
impl From<TextTarget> for Selection {
    fn from(t: TextTarget) -> Self {
        Self::Text {
            anchor: TextPoint {
                zone: t.zone.clone(),
                offset: t.from,
            },
            head: TextPoint {
                zone: t.zone,
                offset: t.to,
            },
        }
    }
}
fn unsupported() -> Error {
    Error::EditingContext {
        reason: "text_selection_required".into(),
    }
}
impl Selection {
    pub(super) fn head_target(&self) -> Result<TextTarget, Error> {
        match self {
            Self::Text { head, .. } => Ok(head.target()),
            _ => Err(unsupported()),
        }
    }
    pub fn text_target(&self) -> Result<TextTarget, Error> {
        match self {
            Self::Text { anchor, head } if anchor.zone == head.zone => Ok(TextTarget {
                zone: anchor.zone.clone(),
                from: anchor.offset.min(head.offset),
                to: anchor.offset.max(head.offset),
            }),
            _ => Err(unsupported()),
        }
    }
}
fn text_root(zone: &TextZone) -> Option<TreeRoot> {
    match zone {
        TextZone::Body { .. } => Some(TreeRoot::Body),
        TextZone::Record {
            collection,
            id,
            field,
            path,
        } if !path.is_empty() => Some(TreeRoot::Record {
            collection: collection.clone(),
            id: id.clone(),
            field: field.clone(),
        }),
        _ => None,
    }
}
pub(super) struct NodeAnchor {
    root: TreeRoot,
    identity: StickyIndex,
}
impl NodeAnchor {
    fn capture(
        doc: &SharedDocument,
        state: &State,
        target: &TreeTarget,
        cell: bool,
    ) -> Result<Self, Error> {
        let root = structure::root(&doc.doc, state, &target.root)?;
        let tx = doc.doc.transact();
        let node = targets::element(root, &tx, &target.path)?;
        if cell && !matches!(node.tag().as_ref(), "tableCell" | "tableHeader") {
            return Err(targets::invalid(TargetReason::NotTextZone));
        }
        Ok(Self {
            root: target.root.clone(),
            identity: StickyIndex::from_type(&tx, &node, Assoc::After),
        })
    }
    fn resolve(&self, doc: &SharedDocument, state: &State) -> Result<TreeTarget, Error> {
        let root = structure::root(&doc.doc, state, &self.root)?;
        let tx = doc.doc.transact();
        let id = self
            .identity
            .get_offset(&tx)
            .ok_or_else(|| targets::invalid(TargetReason::MissingNode))?
            .branch
            .id();
        let path = super::anchors::path(root, &tx, &id)
            .ok_or_else(|| targets::invalid(TargetReason::MissingNode))?;
        Ok(TreeTarget {
            root: self.root.clone(),
            path,
        })
    }
}
fn same_table(a: &TreeTarget, b: &TreeTarget) -> bool {
    a.root == b.root
        && a.path.len() >= 3
        && b.path.len() >= 3
        && a.path[..a.path.len() - 2] == b.path[..b.path.len() - 2]
}
pub(super) enum AnchoredSelection {
    Range {
        range: Range,
        backward: bool,
    },
    Text {
        anchor: Range,
        head: Box<Range>,
    },
    Node(NodeAnchor),
    Cells {
        anchor: NodeAnchor,
        head: NodeAnchor,
    },
    All {
        root: TreeRoot,
        identity: StickyIndex,
    },
}
impl AnchoredSelection {
    pub(super) fn capture(
        doc: &SharedDocument,
        state: &State,
        selection: &Selection,
    ) -> Result<Self, Error> {
        match selection {
            Selection::Text { anchor, head } if anchor.zone == head.zone => Ok(Self::Range {
                range: Range::capture(doc, state, &selection.text_target()?)?,
                backward: anchor.offset > head.offset,
            }),
            Selection::Text { anchor, head } => {
                if text_root(&anchor.zone).is_none()
                    || text_root(&anchor.zone) != text_root(&head.zone)
                {
                    return Err(unsupported());
                }
                Ok(Self::Text {
                    anchor: Range::capture(doc, state, &anchor.target())?,
                    head: Box::new(Range::capture(doc, state, &head.target())?),
                })
            }
            Selection::Node { target } => {
                Ok(Self::Node(NodeAnchor::capture(doc, state, target, false)?))
            }
            Selection::Cells { anchor, head } => {
                if !same_table(anchor, head) {
                    return Err(unsupported());
                }
                Ok(Self::Cells {
                    anchor: NodeAnchor::capture(doc, state, anchor, true)?,
                    head: NodeAnchor::capture(doc, state, head, true)?,
                })
            }
            Selection::All { root } => {
                let value = structure::root(&doc.doc, state, root)?;
                let tx = doc.doc.transact();
                Ok(Self::All {
                    root: root.clone(),
                    identity: StickyIndex::from_type(&tx, &value, Assoc::After),
                })
            }
        }
    }
    pub(super) fn text_zone(&self, doc: &SharedDocument, state: &State) -> Result<TextZone, Error> {
        match self {
            Self::Range { range, .. } => range.resolve_zone(doc, state),
            _ => Err(unsupported()),
        }
    }
    pub(super) fn resolve(&self, doc: &SharedDocument, state: &State) -> Result<Selection, Error> {
        match self {
            Self::Range { range, backward } => {
                let t = range.resolve(doc, state)?;
                Ok(Selection::Text {
                    anchor: TextPoint {
                        zone: t.zone.clone(),
                        offset: if *backward { t.to } else { t.from },
                    },
                    head: TextPoint {
                        zone: t.zone,
                        offset: if *backward { t.from } else { t.to },
                    },
                })
            }
            Self::Text { anchor, head } => {
                let a = anchor.resolve(doc, state)?;
                let h = head.resolve(doc, state)?;
                Ok(Selection::Text {
                    anchor: TextPoint {
                        zone: a.zone,
                        offset: a.from,
                    },
                    head: TextPoint {
                        zone: h.zone,
                        offset: h.from,
                    },
                })
            }
            Self::Node(target) => Ok(Selection::Node {
                target: target.resolve(doc, state)?,
            }),
            Self::Cells { anchor, head } => {
                let a = anchor.resolve(doc, state)?;
                let h = head.resolve(doc, state)?;
                if !same_table(&a, &h) {
                    return Err(unsupported());
                }
                Ok(Selection::Cells { anchor: a, head: h })
            }
            Self::All { root, identity } => {
                let value = structure::root(&doc.doc, state, root)?;
                let tx = doc.doc.transact();
                let id = identity
                    .get_offset(&tx)
                    .ok_or_else(|| targets::invalid(TargetReason::MissingNode))?
                    .branch
                    .id();
                if id != <yrs::XmlFragmentRef as AsRef<yrs::branch::Branch>>::as_ref(&value).id() {
                    return Err(targets::invalid(TargetReason::MissingNode));
                }
                Ok(Selection::All { root: root.clone() })
            }
        }
    }
}
