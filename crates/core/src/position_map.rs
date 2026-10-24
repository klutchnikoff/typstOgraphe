//! Exact retained-position mapping from recorded Unicode splices, never text matching.
use crate::{
    document::{
        Projection,
        structure::{TargetError, targets::PositionIndex},
    },
    session::{Caret, Selection, SelectionTarget, TargetGuard},
};
use std::{
    ops::Range,
    sync::{Arc, atomic::AtomicU64},
};
static NEXT_TRACE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub(crate) struct Splice {
    pub at: usize,
    removed: Arc<str>,
    inserted: Arc<str>,
}
impl Splice {
    pub fn new(at: usize, removed: &str, inserted: &str) -> Self {
        Self {
            at,
            removed: removed.into(),
            inserted: inserted.into(),
        }
    }
    fn inverse(&self) -> Self {
        Self {
            at: self.at,
            removed: self.inserted.clone(),
            inserted: self.removed.clone(),
        }
    }
    fn map(&self, index: usize) -> usize {
        let removed = self.removed.chars().count();
        let inserted = self.inserted.chars().count();
        if index < self.at {
            index
        } else if index <= self.at + removed {
            self.at + inserted
        } else {
            index - removed + inserted
        }
    }
}
pub(crate) fn inverse(edits: &[Splice]) -> Vec<Splice> {
    edits.iter().rev().map(Splice::inverse).collect()
}

#[derive(Clone, Debug)]
pub(crate) struct Trace {
    root: u64,
    pub edits: Vec<Splice>,
}
impl Default for Trace {
    fn default() -> Self {
        Self {
            root: crate::identity::next(&NEXT_TRACE),
            edits: vec![],
        }
    }
}
impl Trace {
    pub fn between(&self, other: &Self) -> Option<Vec<Splice>> {
        if self.root != other.root {
            return None;
        }
        // Branches share the root, including literal shortcut snapshots used by undo.
        let mut edits = inverse(&self.edits);
        edits.extend(other.edits.clone());
        Some(edits)
    }
}

pub(crate) fn verifies(before: &str, after: &str, edits: &[Splice]) -> bool {
    let mut text: Vec<char> = before.chars().collect();
    for edit in edits {
        let removed: Vec<char> = edit.removed.chars().collect();
        let Some(end) = edit.at.checked_add(removed.len()) else {
            return false;
        };
        if text.get(edit.at..end) != Some(removed.as_slice()) {
            return false;
        }
        text.splice(edit.at..end, edit.inserted.chars());
    }
    text.iter().copied().eq(after.chars())
}

/// A single published transaction. Only positions retained by the host may be
/// mapped; external command references still require their exact current guard.
#[derive(Clone, Debug)]
pub struct PositionMap {
    pub before: TargetGuard,
    pub after: TargetGuard,
    old: PositionIndex,
    new: PositionIndex,
    pub(crate) edits: Vec<Splice>,
}
impl PositionMap {
    pub(crate) fn new(
        before: TargetGuard,
        after: TargetGuard,
        old: PositionIndex,
        new: PositionIndex,
        edits: Vec<Splice>,
    ) -> Self {
        Self {
            before,
            after,
            old,
            new,
            edits,
        }
    }
    pub fn map_retained(&self, target: SelectionTarget) -> Result<SelectionTarget, TargetError> {
        if target.guard.document_id != self.before.document_id {
            return Err(TargetError::ForeignDocument);
        }
        if target.guard != self.before {
            return Err(TargetError::StaleRevision);
        }
        let (anchor, head) = self.old.resolve_selection(target.selection)?;
        let selection = self.map_selection(Selection {
            primary: Caret {
                index: head,
                prefer_next_row: false,
            },
            secondary: Caret {
                index: anchor,
                prefer_next_row: false,
            },
        });
        Ok(SelectionTarget {
            guard: self.after,
            selection: self
                .new
                .selection_at(selection.secondary.index, selection.primary.index)?,
        })
    }
    pub(crate) fn map_selection(&self, mut selection: Selection) -> Selection {
        for caret in [&mut selection.primary, &mut selection.secondary] {
            for edit in &self.edits {
                caret.index = edit.map(caret.index);
            }
        }
        selection
    }
}

impl Projection {
    pub(crate) fn record_splice(&mut self, range: Range<usize>, inserted: &str) {
        if range.is_empty() && inserted.is_empty() {
            return;
        }
        let removed: String = self
            .text
            .chars()
            .skip(range.start)
            .take(range.len())
            .collect();
        self.trace
            .edits
            .push(Splice::new(range.start, &removed, inserted));
    }
    pub(crate) fn reset_trace(&mut self) {
        self.trace = Trace::default();
    }
}
