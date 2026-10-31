//! Explicit recipe-only IPC. Never registered in the distributed application.
use crate::Bridge;
use tauri::{Emitter, Manager, State, WebviewWindow};
#[tauri::command(async)]
pub fn diagnostic_confirmed(state: State<Bridge>) -> Result<serde_json::Value, String> {
    if state.diagnostic.is_none() {
        return Err("Recette native inactive.".into());
    }
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let value = session.ensure_owner()?.state().clone();
    Ok(serde_json::json!({"guard":session.guard(),"state":value}))
}
#[tauri::command]
pub fn diagnostic_mode(state: State<Bridge>) -> Option<String> {
    state.diagnostic.clone()
}
#[tauri::command]
pub fn diagnostic_event(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    target: String,
    step: String,
    payload: serde_json::Value,
) -> Result<(), String> {
    if state.diagnostic.is_none() || app.get_webview_window(&target).is_none() {
        return Err("Recette native inactive.".into());
    }
    if step == "progress" {
        eprintln!("Diagnostic {}: {}", window.label(), payload);
    }
    app.emit_to(
        target,
        "web-editor:diagnostic",
        serde_json::json!({"from":window.label(),"step":step,"payload":payload}),
    )
    .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn diagnostic_finish(
    app: tauri::AppHandle,
    state: State<Bridge>,
    report: serde_json::Value,
) -> Result<(), String> {
    if state.diagnostic.is_none() {
        return Err("Recette native inactive.".into());
    }
    let session = state.session.lock().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&session.root).map_err(|e| e.to_string())?;
    std::fs::write(
        session.root.join(format!(
            "native-{}-report.json",
            state.diagnostic.as_deref().unwrap_or("inactive")
        )),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    drop(session);
    app.exit(if report["success"].as_bool() == Some(true) {
        0
    } else {
        1
    });
    Ok(())
}
