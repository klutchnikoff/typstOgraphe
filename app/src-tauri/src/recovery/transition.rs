//! A document switch waits for every window's durable private checkpoint.
use crate::{Bridge, session::Session};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::{Emitter, State};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub id: String,
    pub generation: String,
    pub revision: u64,
    pub windows: HashSet<String>,
    pub acknowledged: HashSet<String>,
    pub errors: HashMap<String, String>,
    pub abandon: HashSet<String>,
}
impl Transition {
    pub fn ready(&self) -> bool {
        self.errors.is_empty() && self.windows == self.acknowledged
    }
}
#[derive(Deserialize)]
pub struct Acknowledgement {
    pub id: String,
    pub error: Option<String>,
    #[serde(default)]
    pub abandon: bool,
}
impl Session {
    pub fn begin_transition(
        &mut self,
        window: &str,
        generation: &str,
    ) -> Result<Transition, String> {
        self.check_peer(window, generation)?;
        if self.transition.is_some() {
            return Err("recovery_transition_busy".into());
        }
        let value = Transition {
            id: uuid::Uuid::new_v4().to_string(),
            generation: generation.into(),
            revision: self.revision,
            windows: self.synchronization().windows.into_iter().collect(),
            acknowledged: HashSet::new(),
            errors: HashMap::new(),
            abandon: HashSet::new(),
        };
        self.transition = Some(value.clone());
        Ok(value)
    }
    pub fn acknowledge_transition(
        &mut self,
        window: &str,
        ack: Acknowledgement,
    ) -> Result<(), String> {
        let conserved = self
            .draft_epochs
            .get(window)
            .is_some_and(|(_, sequence)| *sequence > 0);
        if ack.error.is_none() && !conserved {
            return Err("recovery_transition_not_conserved".into());
        }
        let value = self
            .transition
            .as_mut()
            .ok_or("recovery_transition_obsolete")?;
        if value.id != ack.id || !value.windows.contains(window) {
            return Err("recovery_transition_obsolete".into());
        }
        if let Some(error) = ack.error {
            value.errors.insert(window.into(), error);
        } else {
            value.acknowledged.insert(window.into());
            if ack.abandon {
                value.abandon.insert(window.into());
            }
        }
        Ok(())
    }
    pub fn transition_status(&self, id: &str) -> Result<Transition, String> {
        let value = self
            .transition
            .as_ref()
            .ok_or("recovery_transition_obsolete")?;
        if value.id != id {
            return Err("recovery_transition_obsolete".into());
        }
        Ok(value.clone())
    }
}
#[tauri::command]
pub fn transition_begin(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    generation: String,
) -> Result<Transition, String> {
    let value = state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .begin_transition(window.label(), &generation)?;
    if let Err(error) = app.emit("recovery:prepare-transition", &value) {
        state.session.lock().map_err(|e| e.to_string())?.transition = None;
        return Err(error.to_string());
    }
    Ok(value)
}
#[tauri::command]
pub fn transition_ack(
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    ack: Acknowledgement,
) -> Result<(), String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .acknowledge_transition(window.label(), ack)
}
#[tauri::command]
pub fn transition_status(state: State<Bridge>, id: String) -> Result<Transition, String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .transition_status(&id)
}
#[tauri::command]
pub fn transition_end(
    app: tauri::AppHandle,
    state: State<Bridge>,
    id: String,
) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    if session.transition.as_ref().is_some_and(|v| v.id == id) {
        session.transition = None;
    }
    app.emit("recovery:end-transition", id)
        .map_err(|e| e.to_string())
}
