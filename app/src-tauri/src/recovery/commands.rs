//! Native recovery boundary. Paths never come from catalogue identifiers.
use super::{drafts, work};
use crate::Bridge;
use serde_json::json;
use std::path::PathBuf;
use tauri::{Emitter, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command(async)]
pub fn work_list(state: State<Bridge>) -> Result<Vec<work::Entry>, String> {
    state.session.lock().map_err(|e| e.to_string())?.work_list()
}
#[tauri::command(async)]
pub fn work_delete(state: State<Bridge>, id: String) -> Result<(), String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .work_delete(&id)
}
#[tauri::command(async)]
pub fn work_restore(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    id: String,
    revision: u64,
) -> Result<(), String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .work_restore(window.label(), &id, revision)?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}
#[tauri::command(async)]
pub fn work_abandon(
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    generation: String,
) -> Result<(), String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .abandon_window_work(window.label(), &generation)
}
#[tauri::command]
pub async fn choose_work_file(
    app: tauri::AppHandle,
    save: bool,
) -> Result<Option<PathBuf>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let picker = app.dialog().file().add_filter("JSON", &["json"]);
    if save {
        picker
            .set_file_name("typstographe-draft.json")
            .save_file(move |p| {
                let _ = tx.send(p);
            });
    } else {
        picker.pick_file(move |p| {
            let _ = tx.send(p);
        });
    }
    rx.await
        .map_err(|e| e.to_string())?
        .map(|p| p.into_path().map_err(|e| e.to_string()))
        .transpose()
}
#[tauri::command(async)]
pub fn work_import(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    path: PathBuf,
    revision: u64,
) -> Result<(), String> {
    let record = work::read_work(&path)?;
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.lifecycle(revision)?;
    session.restore_work_record(window.label(), record)?;
    drop(session);
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}
#[tauri::command(async)]
pub fn work_export(
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    id: String,
    path: PathBuf,
) -> Result<(), String> {
    let mut s = state.session.lock().map_err(|e| e.to_string())?;
    let item = s
        .recovered_items
        .get(window.label())
        .and_then(|v| v.iter().find(|v| v.id == id))
        .cloned()
        .ok_or("recovery_unknown_item")?;
    let resources = s.resources();
    let generation = s.generation.clone();
    let owner = s.ensure_owner()?;
    let checkpoint = crate::session::Checkpoint {
        schema: "typstographe-web-editor-v1".into(),
        source: String::new(),
        document: owner.state().document.clone(),
        settings: owner.state().settings.clone(),
        update: owner.update_bytes(),
        resources: resources.clone(),
    };
    let input = drafts::Input {
        generation,
        epoch: 0,
        sequence: 0,
        update: checkpoint.update.clone(),
        document: json!(checkpoint.document),
        settings: json!(checkpoint.settings),
        fields: vec![],
        forms: vec![],
        resources,
        rejected: false,
        recovered: vec![item],
        retain: true,
    };
    drafts::write(
        &path,
        &drafts::Record {
            format: drafts::FORMAT.into(),
            name: s.recovered_name.clone(),
            window: window.label().into(),
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs(),
            file_path: None,
            confirmed_revision: s.revision,
            confirmed: checkpoint,
            input,
            rejected_edition: None,
        },
    )
}

#[tauri::command(async)]
pub fn diagnostic_private_records(
    state: tauri::State<Bridge>,
) -> Result<Vec<crate::recovery::drafts::Record>, String> {
    if state.diagnostic.is_none() {
        return Err("Diagnostic inactive".into());
    }
    let s = state.session.lock().map_err(|e| e.to_string())?;
    let mut records = vec![];
    if let Ok(generations) = std::fs::read_dir(s.root.join("drafts")) {
        for generation in generations.flatten() {
            if generation.file_type().is_ok_and(|t| t.is_dir()) {
                for entry in std::fs::read_dir(generation.path())
                    .map_err(|e| e.to_string())?
                    .flatten()
                {
                    if entry.path().extension().is_some_and(|e| e == "json") {
                        let record = crate::recovery::drafts::read(&entry.path())?;
                        crate::session::Session::from_checkpoint(
                            s.root.clone(),
                            record.confirmed.clone(),
                        )?;
                        records.push(record);
                    }
                }
            }
        }
    }
    Ok(records)
}
