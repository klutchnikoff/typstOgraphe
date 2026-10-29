//! Relative positions are private. Public commands still use guarded zone paths.
use crate::content::{
    commands::{TextTarget, TextZone},
    contract::{Error, State, TargetReason},
    shared::{
        SharedDocument,
        targets::{self, Resolved},
    },
};
use yrs::{
    Assoc, IndexedSequence, Map, Out, ReadTxn, StickyIndex, Text, Transact, XmlElementRef,
    XmlFragment, XmlFragmentRef, XmlOut,
};
#[derive(Clone)]
enum Position {
    Text(StickyIndex),
    Gap(StickyIndex),
    Scalar(u32),
}
#[derive(Clone)]
pub(super) struct Range {
    zone: TextZone,
    identity: Option<StickyIndex>,
    from: Position,
    to: Position,
    scalar: Option<String>,
    field_identity: Option<StickyIndex>,
}
fn branch_id<T: AsRef<yrs::branch::Branch>>(value: &T) -> yrs::BranchID {
    value.as_ref().id()
}
fn lost() -> Error {
    targets::invalid(TargetReason::MissingNode)
}
fn size<T: ReadTxn>(node: &XmlOut, tx: &T) -> u32 {
    match node {
        XmlOut::Text(t) => t.len(tx),
        _ => 1,
    }
}
fn sticky<T: ReadTxn, B: IndexedSequence + AsRef<yrs::branch::Branch>>(
    value: &B,
    tx: &T,
    offset: u32,
    len: u32,
    assoc: Assoc,
) -> Option<StickyIndex> {
    // Yrs returns None for an After association at the sequence end.
    if offset == len && assoc == Assoc::After {
        Some(StickyIndex::from_type(tx, value, assoc))
    } else {
        value.sticky_index(tx, offset, assoc)
    }
}
fn point<T: ReadTxn>(
    zone: &Resolved,
    tx: &T,
    offset: u32,
    assoc: Assoc,
) -> Result<Position, Error> {
    match zone {
        Resolved::Rich(parent) => {
            let mut at = 0;
            for (index, child) in parent.children(tx).enumerate() {
                if let XmlOut::Text(text) = &child
                    && offset >= at
                    && offset <= at + text.len(tx)
                {
                    return sticky(text, tx, offset - at, text.len(tx), assoc)
                        .map(Position::Text)
                        .ok_or_else(lost);
                }
                if offset == at {
                    return sticky(parent, tx, index as u32, parent.len(tx), assoc)
                        .map(Position::Gap)
                        .ok_or_else(lost);
                }
                at += size(&child, tx);
            }
            sticky(parent, tx, parent.len(tx), parent.len(tx), assoc)
                .map(Position::Gap)
                .ok_or_else(lost)
        }
        Resolved::Text(text) => sticky(text, tx, offset, text.len(tx), assoc)
            .map(Position::Text)
            .ok_or_else(lost),
        _ => Ok(Position::Scalar(offset)),
    }
}
pub(super) fn path<T: ReadTxn>(
    root: XmlFragmentRef,
    tx: &T,
    id: &yrs::BranchID,
) -> Option<Vec<usize>> {
    let mut index = 0;
    for child in root.children(tx) {
        let width =
            crate::content::shared::projection::xml(std::iter::once(child.clone()), tx, 1, &mut 0)
                .ok()?
                .len();
        if let XmlOut::Element(element) = child {
            if branch_id(&element) == *id {
                return Some(vec![index]);
            }
            if let Some(mut nested) = path(
                <XmlElementRef as AsRef<XmlFragmentRef>>::as_ref(&element).clone(),
                tx,
                id,
            ) {
                nested.insert(0, index);
                return Some(nested);
            }
        }
        index += width;
    }
    None
}
fn resolve_point<T: ReadTxn>(point: &Position, zone: &Resolved, tx: &T) -> Result<u32, Error> {
    match (point, zone) {
        (Position::Scalar(i), _) => Ok(*i),
        (Position::Text(mark), Resolved::Text(text)) => {
            let pos = mark.get_offset(tx).ok_or_else(lost)?;
            if pos.branch.id() != branch_id(text) {
                return Err(lost());
            }
            Ok(pos.index)
        }
        (Position::Text(mark), Resolved::Rich(parent)) => {
            let pos = mark.get_offset(tx).ok_or_else(lost)?;
            let mut at = 0;
            for child in parent.children(tx) {
                if let XmlOut::Text(t) = &child
                    && branch_id(t) == pos.branch.id()
                {
                    return Ok(at + pos.index);
                }
                at += size(&child, tx);
            }
            Err(lost())
        }
        (Position::Gap(mark), Resolved::Rich(parent)) => {
            let pos = mark.get_offset(tx).ok_or_else(lost)?;
            if pos.branch.id() != branch_id(parent) {
                return Err(lost());
            }
            Ok(parent
                .children(tx)
                .take(pos.index as usize)
                .map(|n| size(&n, tx))
                .sum())
        }
        _ => Err(lost()),
    }
}
impl Range {
    pub(super) fn capture(
        doc: &SharedDocument,
        state: &State,
        target: &TextTarget,
    ) -> Result<Self, Error> {
        if target.from > target.to {
            return Err(targets::invalid(TargetReason::ReversedRange));
        }
        let resolved = targets::resolve(&doc.doc, state, &target.zone)?;
        let tx = doc.doc.transact();
        let info = resolved.info(&tx)?;
        targets::byte_at(&info.text, target.from)?;
        targets::byte_at(&info.text, target.to)?;
        let identity = match &resolved {
            Resolved::Rich(e) | Resolved::Annotation(e) => {
                Some(StickyIndex::from_type(&tx, e, Assoc::After))
            }
            _ => None,
        };
        let field_identity = match &resolved {
            Resolved::Scalar { map, .. } => Some(StickyIndex::from_type(&tx, map, Assoc::After)),
            _ => None,
        };
        Ok(Self {
            field_identity,
            zone: target.zone.clone(),
            identity,
            from: point(&resolved, &tx, target.from, Assoc::After)?,
            to: point(
                &resolved,
                &tx,
                target.to,
                if target.from == target.to {
                    Assoc::After
                } else {
                    Assoc::Before
                },
            )?,
            scalar: if matches!(resolved, Resolved::Scalar { .. } | Resolved::Annotation(_)) {
                Some(info.text)
            } else {
                None
            },
        })
    }
    pub(super) fn resolve_zone(
        &self,
        doc: &SharedDocument,
        state: &State,
    ) -> Result<TextZone, Error> {
        let mut zone = self.zone.clone();
        if let Some(identity) = &self.identity {
            let tx = doc.doc.transact();
            let id = identity.get_offset(&tx).ok_or_else(lost)?.branch.id();
            let root = match &zone {
                TextZone::Body { .. } | TextZone::Annotation { .. } => {
                    tx.get_xml_fragment("default").ok_or_else(lost)?
                }
                TextZone::Record {
                    collection,
                    id,
                    field,
                    ..
                } => {
                    let (map, _) = targets::record(&doc.doc, &tx, state, collection, id, field)?;
                    match map.get(&tx, field) {
                        Some(Out::YXmlFragment(root)) => root,
                        _ => return Err(lost()),
                    }
                }
                _ => return Err(lost()),
            };
            let current = path(root, &tx, &id).ok_or_else(lost)?;
            match &mut zone {
                TextZone::Body { path }
                | TextZone::Annotation { path }
                | TextZone::Record { path, .. } => *path = current,
                _ => return Err(lost()),
            }
        }
        Ok(zone)
    }
    pub(super) fn resolve(&self, doc: &SharedDocument, state: &State) -> Result<TextTarget, Error> {
        let zone = self.resolve_zone(doc, state)?;
        let resolved = targets::resolve(&doc.doc, state, &zone)?;
        let tx = doc.doc.transact();
        let info = resolved.info(&tx)?;
        if let Some(identity) = &self.field_identity {
            let Resolved::Scalar { map, .. } = &resolved else {
                return Err(lost());
            };
            if identity.get_offset(&tx).ok_or_else(lost)?.branch.id() != branch_id(map) {
                return Err(lost());
            }
        }
        if self
            .scalar
            .as_ref()
            .is_some_and(|before| before != &info.text)
        {
            return Err(lost());
        }
        let from = resolve_point(&self.from, &resolved, &tx)?;
        let to = resolve_point(&self.to, &resolved, &tx)?;
        if from > to {
            return Err(lost());
        }
        targets::byte_at(&info.text, from)?;
        targets::byte_at(&info.text, to)?;
        Ok(TextTarget { zone, from, to })
    }
}
