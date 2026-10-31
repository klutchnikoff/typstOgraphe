//! Immutable documentary comparisons and guarded human decisions, shared by all adapters.
mod diff;
use super::{
    Node,
    archive::Archive,
    contract::{Error, Guard},
    editing::{ContextId, EditingSession},
    shared::{PreparedUpdate, Publication},
};
pub(in crate::content) use diff::sequence_diff;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Range};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceSide {
    Before,
    After,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Pending,
    Applied,
    Refused,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Body,
    Document,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextChangeKind {
    Equal,
    Delete,
    Insert,
}
#[derive(Clone, Debug, Serialize)]
pub struct TextChange {
    pub kind: TextChangeKind,
    pub text: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Group {
    pub id: usize,
    pub section: Section,
    pub before: Range<usize>,
    pub after: Range<usize>,
    pub linked: bool,
    pub changes: Vec<TextChange>,
    pub decision: Decision,
}
#[derive(Clone, Debug, Serialize)]
pub struct FileState {
    pub bytes: usize,
    pub hash: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct FileChange {
    pub kind: String,
    pub path: String,
    pub before: Option<FileState>,
    pub after: Option<FileState>,
}
#[derive(Clone, Serialize)]
pub struct Report {
    pub guard: Guard,
    pub source: SourceSide,
    pub groups: Vec<Group>,
    pub stale: bool,
    pub files: Vec<FileChange>,
}
/// Holds private immutable states. Reports are descriptive, never executable patches.
#[derive(Clone)]
pub struct Comparison {
    identity: uuid::Uuid,
    before: Archive,
    after: Archive,
    target: Archive,
    guard: Guard,
    source: SourceSide,
    groups: Vec<Group>,
    files: Vec<FileChange>,
    epoch: u64,
}
pub struct PreparedDecision {
    update: PreparedUpdate,
    archive: Archive,
    comparison: uuid::Uuid,
    ids: Vec<usize>,
    epoch: u64,
    guard: Guard,
}
impl PreparedDecision {
    pub fn archive(&self) -> &Archive {
        &self.archive
    }
}
fn invalid(reason: &str) -> Error {
    Error::InvalidParameters {
        parameter: reason.into(),
    }
}
impl Comparison {
    pub fn new(
        before: Archive,
        after: Archive,
        source: SourceSide,
        guard: Guard,
    ) -> Result<Self, Error> {
        for state in [&before, &after] {
            state.validate().map_err(|e| Error::InvalidDocument {
                reason: e.to_string(),
            })?;
        }
        if before.metadata.document_type() != after.metadata.document_type() {
            return Err(invalid("comparison_document_type"));
        }
        let groups = diff::groups(&before, &after);
        let files = diff::files(&before, &after);
        let target = match source {
            SourceSide::Before => after.clone(),
            SourceSide::After => before.clone(),
        };
        Ok(Self {
            identity: uuid::Uuid::new_v4(),
            before,
            after,
            target,
            guard,
            source,
            groups,
            files,
            epoch: 0,
        })
    }
    pub fn guard(&self) -> &Guard {
        &self.guard
    }
    pub fn before(&self) -> &Archive {
        &self.before
    }
    pub fn after(&self) -> &Archive {
        &self.after
    }
    pub fn report(&self, current: &Guard) -> Report {
        Report {
            guard: self.guard.clone(),
            source: self.source,
            groups: self.groups.clone(),
            stale: self.guard != *current,
            files: self.files.clone(),
        }
    }
    fn ids(&self, ids: &[usize]) -> Result<Vec<usize>, Error> {
        if ids.is_empty() {
            return Err(invalid("comparison_groups"));
        }
        let unique: BTreeSet<_> = ids.iter().copied().collect();
        if unique.len() != ids.len()
            || ids.iter().any(|id| {
                self.groups
                    .get(*id)
                    .is_none_or(|g| g.decision != Decision::Pending)
            })
        {
            return Err(invalid("comparison_decision"));
        }
        Ok(unique.into_iter().collect())
    }
    pub fn refuse(&mut self, ids: &[usize]) -> Result<(), Error> {
        let ids = self.ids(ids)?;
        for id in ids {
            self.groups[id].decision = Decision::Refused;
        }
        self.epoch += 1;
        Ok(())
    }
    /// Prepare against the current causal revision. Never publishes or marks a decision.
    pub fn prepare(
        &self,
        engine: &EditingSession,
        ids: &[usize],
    ) -> Result<PreparedDecision, Error> {
        self.guard.check(engine.guard())?;
        let ids = self.ids(ids)?;
        let selected: BTreeSet<_> = ids.iter().copied().collect();
        let source = match self.source {
            SourceSide::Before => &self.before,
            SourceSide::After => &self.after,
        };
        let mut desired = self.target.clone();
        desired.yjs.clear();
        for group in self
            .groups
            .iter()
            .rev()
            .filter(|g| g.decision == Decision::Applied || selected.contains(&g.id))
        {
            if matches!(group.section, Section::Document) {
                desired = source.clone();
                desired.yjs.clear();
            } else {
                let (target, origin) = match self.source {
                    SourceSide::Before => (group.after.clone(), group.before.clone()),
                    SourceSide::After => (group.before.clone(), group.after.clone()),
                };
                desired
                    .document
                    .content
                    .splice(target, source.document.content[origin].iter().cloned());
            }
        }
        let update = engine.prepare_comparison(&self.guard, &desired)?;
        desired.yjs = update.checkpoint();
        Ok(PreparedDecision {
            comparison: self.identity,
            update,
            archive: desired,
            ids,
            epoch: self.epoch,
            guard: self.guard.clone(),
        })
    }
    /// The owner's single publication/history path also protects IME, anchors and undo.
    pub fn publish(
        &mut self,
        engine: &mut EditingSession,
        context: ContextId,
        prepared: PreparedDecision,
    ) -> Result<Publication, Error> {
        if prepared.comparison != self.identity {
            return Err(invalid("comparison_identity"));
        }
        if prepared.epoch != self.epoch {
            return Err(invalid("comparison_decision_changed"));
        }
        self.guard.check(&prepared.guard)?;
        let result = engine.publish_prepared(&prepared.guard, context, prepared.update)?;
        for id in prepared.ids {
            self.groups[id].decision = Decision::Applied;
        }
        self.guard = result.guard.clone();
        self.epoch += 1;
        Ok(result)
    }
}
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
