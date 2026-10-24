//! Moteur documentaire propriétaire de sa session : commandes et lectures explicites,
//! sans exposition de Session ni publication directe par l’interface.
mod annotations;
mod availability;
mod batch;
mod batch_types;
mod blocks;
mod contexts;
pub use contexts::{ContextSnapshot, EditContext};
mod discovery;
mod documentation;
mod documentation_examples;
mod documentation_guides;
mod error;
mod exports;
mod file_types;
mod files;
mod image_types;
mod images;
mod metadata;
mod metadata_types;
mod model_types;
mod models;
mod native;
mod proposals;
pub use proposals::PreparedProposal;
mod queries;
mod recovery;
mod recovery_types;
pub mod registry;
mod render;
mod render_types;
mod resources;
mod run;
mod search;
mod search_types;
mod spelling;
mod spelling_types;
mod state;
mod statistics;
mod types;
mod typing;
mod validation;
mod validation_types;
pub use crate::position_map::PositionMap;
pub use crate::session::{Caret, Selection, SelectionTarget, TargetGuard};
use crate::{
    document::{Projection, edits::EditError},
    session::{EditGroup, Session, Snapshot},
};
pub use availability::CommandIntent;
pub use batch::{MAX_BATCH_BYTES, MAX_BATCH_STEPS};
pub use batch_types::*;
pub use error::{CommandError, ErrorCode};
pub use file_types::*;
pub use image_types::*;
pub use metadata_types::*;
pub use model_types::*;
pub use native::{NativeEdit, NativeInput};
pub use recovery_types::*;
pub use registry::{AvailabilityRule, Call, CommandId, CommandSpec, Family, catalogue};
pub use render_types::*;
pub use search_types::*;
use serde::Serialize;
use serde_json::Value;
pub use spelling_types::*;
pub use state::HistoryInfo;
pub use statistics::{StatisticsParams, StatisticsResult};
pub use types::*;
pub use validation_types::*;

pub struct Engine {
    session: Session,
    open: bool,
    recovery: recovery::RecoveryState,
    selection: Selection,
    contexts: contexts::Contexts,
    spelling: spelling::SpellingState,
    search_cache: std::cell::RefCell<search::SearchCache>,
    render: std::cell::RefCell<Option<render::RenderState>>,
    statistics: std::cell::RefCell<statistics::StatisticsCache>,
    presentation_cache: metadata::PresentationCache,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Availability {
    Enabled,
    Disabled { reason: CommandError },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Render,
    RevealSelection,
    DocumentReplaced,
    Saved,
}

#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub changed: bool,
    pub guard: TargetGuard,
    pub selection: Option<SelectionTarget>,
    pub effects: Vec<Effect>,
    pub value: Option<Value>,
}

impl From<Session> for Engine {
    fn from(session: Session) -> Self {
        let _ = catalogue(); // Vérifier le catalogue dès la création du moteur.
        Self {
            session,
            open: true,
            recovery: Default::default(),
            selection: Selection::default(),
            contexts: Default::default(),
            spelling: Default::default(),
            search_cache: Default::default(),
            render: Default::default(),
            statistics: Default::default(),
            presentation_cache: Default::default(),
        }
    }
}
impl Engine {
    /// Last committed transaction, including native input and undo/redo.
    pub fn position_map(&self) -> Option<&PositionMap> {
        self.session.last_positions.as_ref()
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    /// Cache validé du curseur graphique, jamais un indice JSON public.
    pub fn set_adapter_selection(&mut self, selection: Selection) -> Result<(), CommandError> {
        if selection.range().end > self.session.document.view.len() {
            return Err(CommandError::new(ErrorCode::Document, "invalid_selection"));
        }
        self.selection = selection;
        Ok(())
    }
    pub fn run_call(&mut self, call: Call) -> Result<Outcome, CommandError> {
        self.run(Command::from_call(call.id.as_str(), call.params)?)
    }
    pub fn query_call(&self, call: Call) -> Result<Value, CommandError> {
        self.query(Query::from_call(call.id.as_str(), call.params)?)
    }
    fn commit_projection(
        &mut self,
        guard: TargetGuard,
        before: Snapshot,
        view: Projection,
        selection: Selection,
        group: EditGroup,
    ) -> Result<Selection, EditError> {
        let selection = self
            .session
            .commit_projection(guard, before, view, selection, group)?;
        self.selection = selection;
        self.refresh_contexts();
        Ok(selection)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
