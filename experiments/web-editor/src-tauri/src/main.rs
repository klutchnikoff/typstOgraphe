mod driver;
mod rendering;
mod schema;
mod session;

use rendering::{Preview, Renderer};
use session::{Bootstrap, Resource, Session, Snapshot};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{Emitter, Manager, State, WebviewWindow};

struct Bridge {
    session: Arc<Mutex<Session>>,
    renderer: Arc<Renderer>,
    diagnostic: Option<String>,
}
#[derive(Clone, serde::Serialize)]
struct Change {
    from: String,
    revision: u64,
    update: Vec<u8>,
}
#[derive(Clone, serde::Serialize)]
struct Awareness {
    from: String,
    update: Vec<u8>,
}

#[tauri::command]
fn join(window: WebviewWindow, state: State<Bridge>) -> Result<Bootstrap, String> {
    Ok(state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .join(window.label()))
}
#[tauri::command]
fn relay(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    update: Vec<u8>,
) -> Result<u64, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let revision = session.relay(window.label(), &update)?;
    app.emit(
        "web-editor:change",
        Change {
            from: window.label().into(),
            revision,
            update,
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(revision)
}
#[tauri::command]
fn awareness(window: WebviewWindow, app: tauri::AppHandle, update: Vec<u8>) -> Result<(), String> {
    if update.len() > 64 * 1024 {
        return Err("Présence trop volumineuse.".into());
    }
    app.emit(
        "web-editor:awareness",
        Awareness {
            from: window.label().into(),
            update,
        },
    )
    .map_err(|e| e.to_string())
}
#[tauri::command]
async fn preview(state: State<'_, Bridge>, snapshot: Snapshot) -> Result<Preview, String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (source, assets) = {
            let session = session.lock().map_err(|e| e.to_string())?;
            (session.validate(&snapshot)?, session.assets.clone())
        };
        let result = renderer.render(source, assets, snapshot.revision, None)?;
        if session.lock().map_err(|e| e.to_string())?.revision != snapshot.revision {
            return Err("stale".into());
        }
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn save(
    state: State<'_, Bridge>,
    snapshot: Snapshot,
    update: Vec<u8>,
) -> Result<String, String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (source, assets) = {
            let session = session.lock().map_err(|e| e.to_string())?;
            (session.validate(&snapshot)?, session.assets.clone())
        };
        renderer.render(source, assets, snapshot.revision, None)?;
        let session = session.lock().map_err(|e| e.to_string())?;
        let path = session.save(&snapshot, update)?;
        Ok(path.display().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn export_document(state: State<'_, Bridge>, snapshot: Snapshot) -> Result<String, String> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let renderer = state.renderer.clone();
    let (source, assets, root) = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        (
            session.validate(&snapshot)?,
            session.assets.clone(),
            session.root.clone(),
        )
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let path = root
        .join("exports")
        .join(format!("{stamp}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    tauri::async_runtime::spawn_blocking(move || {
        renderer.render(source, assets, snapshot.revision, Some(path.clone()))?;
        Ok(path.display().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn import_image(
    app: tauri::AppHandle,
    state: State<Bridge>,
    extension: String,
    data: Vec<u8>,
) -> Result<Resource, String> {
    let resource = state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .import(&extension, &data)?;
    app.emit("web-editor:asset", &resource)
        .map_err(|e| e.to_string())?;
    Ok(resource)
}
#[tauri::command]
fn open_peer(app: tauri::AppHandle) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    tauri::WebviewWindowBuilder::new(
        &app,
        format!("peer-{id}"),
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title(format!("typstOgraphe — participant {}", id + 1))
    .inner_size(1280.0, 900.0)
    .min_inner_size(800.0, 600.0)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn prepare_rust_edit() -> serde_json::Value {
    serde_json::json!({"type":"paragraph","content":[{"type":"text","text":"Cette phrase a été préparée depuis Rust, puis insérée dans l’état partagé."}]})
}
#[tauri::command]
fn diagnostic_mode(state: State<Bridge>) -> Option<String> {
    state.diagnostic.clone()
}
#[tauri::command]
fn diagnostic_event(
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
    app.emit_to(
        target,
        "web-editor:diagnostic",
        serde_json::json!({"from":window.label(),"step":step,"payload":payload}),
    )
    .map_err(|e| e.to_string())
}
#[tauri::command]
fn diagnostic_finish(
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
    app.exit(if report["success"].as_bool() == Some(true) {
        0
    } else {
        1
    });
    Ok(())
}
fn main() {
    let mut args = std::env::args().skip(1);
    let mut root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../output/web-editor/default");
    let mut stdio = false;
    let mut diagnostic = None;
    while let Some(arg) = args.next() {
        if arg == "--diagnostic" {
            diagnostic = Some("full".to_owned());
        }
        if arg == "--diagnostic-restore" {
            diagnostic = Some("restore".to_owned());
        }
        if arg == "--stdio" {
            stdio = true;
        }
        if arg == "--session-dir" {
            root = PathBuf::from(args.next().expect("--session-dir attend un chemin"));
        }
    }
    let session = Session::open(root).expect("Impossible de rouvrir la session expérimentale");
    let bridge = Bridge {
        session: Arc::new(Mutex::new(session)),
        renderer: Arc::new(Renderer::default()),
        diagnostic,
    };
    if stdio {
        driver::run(bridge);
        return;
    }
    tauri::Builder::default()
        .manage(bridge)
        .invoke_handler(tauri::generate_handler![
            join,
            relay,
            awareness,
            preview,
            save,
            export_document,
            import_image,
            open_peer,
            prepare_rust_edit,
            diagnostic_mode,
            diagnostic_event,
            diagnostic_finish
        ])
        .run(tauri::generate_context!())
        .expect("Erreur du diagnostic Tauri");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_tables_notes_and_images_compile_and_export_through_the_existing_core() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::open(dir.path().into()).unwrap();
        let doc = session.join("test").fixture;
        let source = schema::source(&doc, &session.assets).unwrap();
        let export = dir.path().join("export");
        let result = Renderer::default()
            .render(source, session.assets, 0, Some(export.clone()))
            .unwrap();
        assert!(!result.pages.is_empty());
        assert!(result.pages[0].starts_with("data:image/png;base64,"));
        assert!(export.join("courrier.pdf").exists());
        assert!(export.join("main.typ").exists());
        assert!(export.join("images/parc.svg").exists());
        let body = std::fs::read_to_string(export.join("contenu.typ")).unwrap();
        assert!(body.contains("#footnote"));
        assert!(body.contains("#table"));
    }
}
