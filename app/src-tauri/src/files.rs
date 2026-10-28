use crate::{
    Bridge,
    session::{Resource, Snapshot},
    settings::Settings,
};
use std::path::PathBuf;
use tauri::{Emitter, State};
use tauri_plugin_dialog::DialogExt;
use typstographe_core::{DocumentLease, content::archive::Archive};
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle, state: State<Bridge>) -> Result<(), String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .mark_clean()?;
    app.exit(0);
    Ok(())
}
#[tauri::command]
pub fn set_window_title(window: tauri::WebviewWindow, title: String) -> Result<(), String> {
    window.set_title(&title).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn choose_file(app: tauri::AppHandle, save: bool) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let dialog = app.dialog().file().add_filter("typstOgraphe", &["typsto"]);
    if save {
        dialog
            .set_file_name("Sans titre.typsto")
            .save_file(move |path| {
                let _ = tx.send(path);
            });
    } else {
        dialog.pick_file(move |path| {
            let _ = tx.send(path);
        });
    }
    rx.await
        .map_err(|e| e.to_string())?
        .map(|p| {
            p.into_path()
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| e.to_string())
        })
        .transpose()
}
#[tauri::command]
pub async fn choose_export_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });
    rx.await
        .map_err(|e| e.to_string())?
        .map(|path| {
            path.into_path()
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| e.to_string())
        })
        .transpose()
}
#[tauri::command]
pub async fn save_file(
    state: State<'_, Bridge>,
    path: PathBuf,
    snapshot: Snapshot,
    update: Vec<u8>,
) -> Result<(), String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if path.extension().and_then(|e| e.to_str()) != Some("typsto") {
            return Err("Extension .typsto attendue.".into());
        }
        let (source, assets, settings) = {
            let s = session.lock().map_err(|e| e.to_string())?;
            (
                s.validate(&snapshot)?,
                s.assets.clone(),
                snapshot
                    .settings
                    .clone()
                    .unwrap_or_else(|| s.settings.clone()),
            )
        };
        renderer.validate(&source, &assets, &settings)?;
        let mut s = session.lock().map_err(|e| e.to_string())?;
        s.validate(&snapshot)?;
        let canonical = if path.exists() {
            path.canonicalize()
        } else {
            path.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .canonicalize()
                .map(|p| p.join(path.file_name().unwrap_or_default()))
        }
        .map_err(|e| e.to_string())?;
        let mut lease = if s.file_lease.as_ref().is_some_and(|l| l.path() == canonical) {
            s.file_lease.as_ref().unwrap().clone()
        } else {
            DocumentLease::acquire(&canonical).map_err(|e| e.to_string())?
        };
        let archive = Archive {
            document: snapshot.document,
            metadata: settings.metadata,
            template: settings.template.clone(),
            model_id: settings.model_id,
            assets,
            dependencies: typstographe_core::packages::for_template(&settings.template),
            yjs: update,
        };
        archive.save(&mut lease)?;
        s.file_lease = Some(lease);
        s.mark_clean()?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn open_file(
    app: tauri::AppHandle,
    state: State<'_, Bridge>,
    path: PathBuf,
    revision: u64,
) -> Result<serde_json::Value, String> {
    let session = state.session.clone();
    session
        .lock()
        .map_err(|e| e.to_string())?
        .lifecycle(revision)?;
    let renderer = state.renderer.clone();
    state.agent.lock().map_err(|e| e.to_string())?.token = None;
    tauri::async_runtime::spawn_blocking(move || {
        let canonical = path.canonicalize().map_err(|e| e.to_string())?;
        let lease = session
            .lock()
            .map_err(|e| e.to_string())?
            .file_lease
            .as_ref()
            .filter(|lease| lease.path() == canonical)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| DocumentLease::acquire(&path))
            .map_err(|e| e.to_string())?;
        let archive = match Archive::read(&path) {
            Ok(archive) => archive,
            Err(structured_error) => {
                let legacy = typstographe_core::storage::read(&path)
                    .map_err(|e| format!("{structured_error}\n{e}"))?;
                Archive {
                    document: typstographe_core::content::legacy::import(&legacy.source)?,
                    metadata: legacy.metadata,
                    model_id: "courrier.model.personal".into(),
                    template: legacy.template,
                    assets: legacy.assets,
                    dependencies: legacy.dependencies,
                    yjs: vec![],
                }
            }
        };
        let settings = Settings {
            metadata: archive.metadata.clone(),
            template: archive.template.clone(),
            model_id: archive.model_id.clone(),
        };
        settings.validate().map_err(|e| e.to_string())?;
        renderer.validate(&archive.validate()?, &archive.assets, &settings)?;
        let resources: Vec<_> = archive
            .assets
            .iter()
            .map(|(path, data)| {
                use base64::Engine;
                Resource {
                    path: path.clone(),
                    data: base64::engine::general_purpose::STANDARD.encode(data),
                }
            })
            .collect();
        {
            let mut current = session.lock().map_err(|e| e.to_string())?;
            current.lifecycle(revision)?;
            current.mark_clean()?;
            current.file_lease = Some(lease);
            current.replace(
                archive.document.clone(),
                settings.clone(),
                archive.assets,
                archive.yjs.clone(),
            );
            current.clean_revision = Some(current.revision);
        }
        let _ = crate::preferences::remember(&app, &path);
        app.emit("app:document", path.to_string_lossy().as_ref())
            .map_err(|e| e.to_string())?;
        let response = serde_json::json!({
            "document":archive.document,
            "settings":settings,
            "resources":resources,
            "yjs":archive.yjs,
        });
        Ok(response)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn new_document(
    app: tauri::AppHandle,
    state: State<Bridge>,
    settings: Settings,
    revision: u64,
) -> Result<(), String> {
    settings.validate().map_err(|e| e.to_string())?;
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.lifecycle(revision)?;
    let mut assets = typstographe_core::assets::Assets::default();
    if let Some(signature) = settings.metadata.signature_path() {
        let data = session
            .assets
            .get(signature)
            .ok_or("Signature absente des ressources.")?;
        assets.insert(signature.into(), data.clone());
    }
    let document: typstographe_core::content::Node =
        serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}))
            .unwrap();
    session.mark_clean()?;
    session.file_lease = None;
    session.replace(document, settings, assets, vec![]);
    session.clean_revision = Some(session.revision);
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    agent.snapshot = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn close_document(
    app: tauri::AppHandle,
    state: State<Bridge>,
    revision: u64,
) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.lifecycle(revision)?;
    session.mark_clean()?;
    session.file_lease = None;
    session.opened = false;
    session.generation = uuid::Uuid::new_v4().to_string();
    session.revision += 1;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    agent.snapshot = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}
