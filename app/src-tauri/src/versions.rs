//! Version storage is independent of live editing, private forms and preview synchronization.
use crate::{Bridge, project, session::Session};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tauri::{Emitter, State};
use typstographe_core::{
    DocumentLease,
    content::{contract::Guard, project::VersionChange},
};
use uuid::Uuid;

fn check(s: &Session, peer: &str, guard: &Guard) -> Result<(), String> {
    s.check_peer(peer, &guard.document)?;
    guard.check(&s.guard()).map_err(crate::owner::error)
}
pub fn list(s: &mut Session, peer: &str, guard: &Guard) -> Result<Value, String> {
    check(s, peer, guard)?;
    if s.project.get().is_none() {
        s.stored_project()?;
    }
    Ok(s.project.get().unwrap().owner.version_catalogue())
}
pub fn read(s: &mut Session, peer: &str, guard: &Guard, id: Uuid) -> Result<Value, String> {
    check(s, peer, guard)?;
    if s.project.get().is_none() {
        s.stored_project()?;
    }
    s.project
        .get()
        .unwrap()
        .owner
        .version_content(id)
        .map_err(project::error)
}
pub fn change(
    s: &mut Session,
    peer: &str,
    guard: &Guard,
    change: VersionChange,
    destination: Option<&Path>,
) -> Result<Value, String> {
    check(s, peer, guard)?;
    // Build and validate the complete candidate before touching the file or live owner.
    let candidate = s
        .stored_project()?
        .prepare_version_change(&change)
        .map_err(project::error)?;
    let existing = s.file_lease.as_ref().map(|lease| lease.path());
    if existing.is_some() && destination.is_some() && destination != existing {
        return Err(typstographe_i18n::text("web-versions-existing-path"));
    }
    let path = existing
        .or(destination)
        .ok_or_else(|| typstographe_i18n::text("web-versions-path-required"))?;
    if path.extension().and_then(|e| e.to_str()) != Some("typsto") {
        return Err(typstographe_i18n::text("web-versions-extension"));
    }
    let mut lease = s
        .file_lease
        .clone()
        .map(Ok)
        .unwrap_or_else(|| DocumentLease::acquire(path))
        .map_err(|e| typstographe_i18n::command_error(&e))?;
    candidate.save(&mut lease).map_err(project::error)?;
    let path = lease.path().to_path_buf();
    // The durable commit is authoritative, even if removing redundant recovery files fails.
    s.file_lease = Some(lease);
    s.project = std::sync::OnceLock::from(project::Context::new(candidate, true));
    s.recovered_name = None;
    let warning = s.mark_clean().err();
    let mut response = s.project.get().unwrap().owner.version_catalogue();
    response["path"] = json!(path);
    response["revision"] = json!(s.revision);
    response["warning"] = json!(warning);
    Ok(response)
}
pub fn selected(
    s: &mut Session,
    peer: &str,
    guard: &Guard,
    id: Uuid,
) -> Result<typstographe_core::content::archive::Archive, String> {
    check(s, peer, guard)?;
    if s.project.get().is_none() {
        s.stored_project()?;
    }
    s.project
        .get()
        .unwrap()
        .owner
        .version(id)
        .map_err(project::error)
}
pub fn preview(
    archive: typstographe_core::content::archive::Archive,
) -> Result<crate::rendering::Preview, String> {
    crate::rendering::Renderer::default().render(
        archive.validate().map_err(project::error)?,
        archive.assets.clone(),
        0,
        None,
        Some(crate::rendering::RenderContext {
            settings: &archive.state().settings,
            dependencies: typstographe_core::packages::for_template(&archive.template),
        }),
        None,
    )
}
#[tauri::command]
pub fn versions_list(
    window: tauri::WebviewWindow,
    state: State<'_, Bridge>,
    guard: Guard,
) -> Result<Value, String> {
    list(
        &mut *state.session.lock().map_err(|e| e.to_string())?,
        window.label(),
        &guard,
    )
}
#[tauri::command]
pub fn versions_read(
    window: tauri::WebviewWindow,
    state: State<'_, Bridge>,
    guard: Guard,
    id: Uuid,
) -> Result<Value, String> {
    read(
        &mut *state.session.lock().map_err(|e| e.to_string())?,
        window.label(),
        &guard,
        id,
    )
}
#[tauri::command]
pub async fn versions_change(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, Bridge>,
    guard: Guard,
    change: VersionChange,
    destination: Option<PathBuf>,
) -> Result<Value, String> {
    let session = state.session.clone();
    let document = guard.document.clone();
    let peer = window.label().to_string();
    let result = tauri::async_runtime::spawn_blocking(move || {
        self::change(
            &mut *session.lock().map_err(|e| e.to_string())?,
            &peer,
            &guard,
            change,
            destination.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit(
        "web-editor:versions",
        json!({"document":document,"path":result["path"],"revision":result["revision"]}),
    );
    Ok(result)
}
#[tauri::command]
pub async fn versions_preview(
    window: tauri::WebviewWindow,
    state: State<'_, Bridge>,
    guard: Guard,
    id: Uuid,
) -> Result<crate::rendering::Preview, String> {
    let archive = selected(
        &mut *state.session.lock().map_err(|e| e.to_string())?,
        window.label(),
        &guard,
        id,
    )?;
    tauri::async_runtime::spawn_blocking(move || preview(archive))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "versions_tests.rs"]
mod tests;
