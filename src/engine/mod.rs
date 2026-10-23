//! Moteur documentaire sans egui, dialogues ni Fluent. Les familles migrées
//! passent par run ; les préparations historiques restent temporairement dans App.
mod annotations;
mod blocks;
mod error;
mod image_types;
mod images;
mod queries;
pub mod registry;
mod run;
mod search;
mod search_types;
mod types;
mod typing;
use crate::{
    document::{Projection, edits::EditError},
    session::{EditGroup, Selection, SelectionTarget, Session, Snapshot, TargetGuard},
};
pub use error::{CommandError, ErrorCode};
pub use image_types::*;
pub use registry::{AvailabilityRule, Call, CommandId, CommandSpec, Family, catalogue};
pub use search_types::*;
use serde::Serialize;
use serde_json::Value;
pub use types::*;

pub struct Engine {
    session: Session,
    selection: Selection,
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
            selection: Selection::default(),
        }
    }
}
impl Engine {
    pub fn selection(&self) -> Selection {
        self.selection
    }
    /// Adaptateur transitoire du curseur graphique, jamais un indice JSON public.
    pub(crate) fn set_adapter_selection(
        &mut self,
        selection: Selection,
    ) -> Result<(), CommandError> {
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
    pub fn commit_projection(
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
        Ok(selection)
    }
    pub fn remember(
        &mut self,
        before: Snapshot,
        selection: Selection,
        typing: bool,
    ) -> Result<Selection, EditError> {
        let selection = self.session.remember(before, selection, typing)?;
        self.selection = selection;
        Ok(selection)
    }
}

// Compatibilité limitée à la migration famille par famille. Les anciens gestes
// préparent encore leurs champs dans App, puis Session::record les valide.
// Ce passage ne constitue pas l'API publique du moteur ni la fin de la phase 2.
impl std::ops::Deref for Engine {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.session
    }
}
impl std::ops::DerefMut for Engine {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
