#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
mod agent;
mod bibliography;
mod catalogue;
mod driver;
mod files;
mod menus;
mod preferences;
mod proofing;
mod recovery;
mod rendering;
mod schema;
mod session;
mod settings;
mod shared_diagnostic;

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
    agent: Arc<Mutex<agent::AgentState>>,
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
    generation: String,
) -> Result<u64, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    if session.generation != generation {
        return Err("Le document a été remplacé ; rechargez la fenêtre.".into());
    }
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
fn connection(window: WebviewWindow, state: State<Bridge>, online: bool) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    if online {
        session.offline.remove(window.label());
    } else {
        session.offline.insert(window.label().into());
    }
    Ok(())
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
fn recover(state: State<Bridge>, snapshot: Snapshot, update: Vec<u8>) -> Result<(), String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .save(&snapshot, update)
        .map(|_| ())
}
#[tauri::command]
async fn preview(state: State<'_, Bridge>, snapshot: Snapshot) -> Result<Preview, String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        session.validate(&snapshot)?;
        state.agent.lock().map_err(|e| e.to_string())?.snapshot = Some(snapshot.clone());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let (source, assets, settings) = {
            let session = session.lock().map_err(|e| e.to_string())?;
            (
                session.validate(&snapshot)?,
                session.assets.clone(),
                snapshot
                    .settings
                    .clone()
                    .unwrap_or_else(|| session.settings.clone()),
            )
        };
        let map = typstographe_core::content::source_mapped(&snapshot.document, &assets)?.1;
        let result = renderer.render(
            source,
            assets,
            snapshot.revision,
            None,
            Some(&settings),
            Some(map),
        )?;
        if session.lock().map_err(|e| e.to_string())?.revision != snapshot.revision {
            return Err("stale".into());
        }
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
/// Binary IPC: no base64, JSON byte arrays, temporary files or asset HTTP server.
#[tauri::command]
async fn preview_frame(
    state: State<'_, Bridge>,
    snapshot: Snapshot,
    encoding: rendering::Encoding,
) -> Result<tauri::ipc::Response, String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    {
        let session = session.lock().map_err(|e| e.to_string())?;
        session.validate(&snapshot)?;
        state.agent.lock().map_err(|e| e.to_string())?.snapshot = Some(snapshot.clone());
    }
    let bytes = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let (source, assets, settings) = {
            let session = session.lock().map_err(|e| e.to_string())?;
            (
                session.validate(&snapshot)?,
                session.assets.clone(),
                snapshot
                    .settings
                    .clone()
                    .unwrap_or_else(|| session.settings.clone()),
            )
        };
        let map = typstographe_core::content::source_mapped(&snapshot.document, &assets)?.1;
        let bytes = renderer.frame(source, assets, snapshot.revision, &settings, map, encoding)?;
        if session.lock().map_err(|e| e.to_string())?.revision != snapshot.revision {
            return Err("stale".into());
        }
        Ok(bytes)
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(tauri::ipc::Response::new(bytes))
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
        let (source, assets, settings) = {
            let session = session.lock().map_err(|e| e.to_string())?;
            (
                session.validate(&snapshot)?,
                session.assets.clone(),
                snapshot
                    .settings
                    .clone()
                    .unwrap_or_else(|| session.settings.clone()),
            )
        };
        renderer.render(
            source,
            assets,
            snapshot.revision,
            None,
            Some(&settings),
            None,
        )?;
        let session = session.lock().map_err(|e| e.to_string())?;
        let path = session.save(&snapshot, update)?;
        Ok(path.display().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn export_document(
    state: State<'_, Bridge>,
    snapshot: Snapshot,
    destination: Option<PathBuf>,
) -> Result<String, String> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let renderer = state.renderer.clone();
    let (source, assets, root, settings) = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        (
            session.validate(&snapshot)?,
            session.assets.clone(),
            session.root.clone(),
            snapshot
                .settings
                .clone()
                .unwrap_or_else(|| session.settings.clone()),
        )
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let path = destination
        .unwrap_or_else(|| root.join("exports"))
        .join(format!("{stamp}-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    tauri::async_runtime::spawn_blocking(move || {
        let assets = typstographe_core::content::retained_assets(
            &snapshot.document,
            &settings.metadata,
            &settings.template,
            &assets,
            &typstographe_core::packages::for_template(&settings.template),
        )?;
        renderer.render(
            source,
            assets,
            snapshot.revision,
            Some(path.clone()),
            Some(&settings),
            None,
        )?;
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
    let arguments: Vec<_> = std::env::args().collect();
    if arguments.iter().any(|arg| arg == "--mcp") {
        let invitation = arguments
            .iter()
            .position(|arg| arg == "--invitation")
            .and_then(|index| arguments.get(index + 1))
            .map(PathBuf::from);
        let result = invitation
            .ok_or("--invitation attend le fichier fourni par l'application.")
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
            .and_then(agent::serve);
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1)
        }
        return;
    }
    let mut args = std::env::args().skip(1);
    let mut root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../output/web-editor/default");
    let mut custom_root = false;
    let mut stdio = false;
    let mut diagnostic = None;
    let mut initial_path = None;
    while let Some(arg) = args.next() {
        if arg == "--diagnostic" {
            diagnostic = Some("full".to_owned());
        }
        if arg == "--diagnostic-performance" {
            diagnostic = Some("performance".to_owned());
        }
        if arg == "--diagnostic-restore" {
            diagnostic = Some("restore".to_owned());
        }
        if arg == "--stdio" {
            stdio = true;
        }
        if arg == "--session-dir" {
            custom_root = true;
            root = PathBuf::from(args.next().expect("--session-dir attend un chemin"));
        } else if !arg.starts_with('-') && initial_path.is_none() {
            initial_path = Some(
                std::path::absolute(&arg)
                    .expect("Chemin de document invalide")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    if stdio || diagnostic.is_some() {
        typstographe_i18n::initialize(&["fr".parse().expect("French locale")]);
    }
    let global_session = !custom_root && !stdio && diagnostic.is_none();
    let session = if global_session {
        Session::fresh(PathBuf::new())
    } else {
        Session::open(root).expect("Impossible de rouvrir la session de recette")
    };
    let bridge = Bridge {
        session: Arc::new(Mutex::new(session)),
        renderer: Arc::new(Renderer::default()),
        diagnostic,
        agent: Arc::new(Mutex::new(agent::AgentState::default())),
    };
    if stdio {
        driver::run(bridge);
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if global_session {
                let root = app.path().app_local_data_dir()?.join("recovery");
                *app.state::<Bridge>()
                    .session
                    .lock()
                    .map_err(|e| e.to_string())? = {
                    let mut session = Session::managed(root)?;
                    session.initial_path = initial_path;
                    session
                };
            }
            menus::install(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let state = window.state::<Bridge>();
                if let Ok(mut session) = state.session.lock() {
                    session.offline.remove(window.label());
                }
                if let Ok(mut agent) = state.agent.lock() {
                    agent.busy.remove(window.label());
                }
            }
        })
        .on_menu_event(|app, event| {
            if let Some(window) = app
                .webview_windows()
                .into_values()
                .find(|window| window.is_focused().unwrap_or(false))
            {
                let _ = window.emit("app:command", event.id().as_ref());
            }
        })
        .manage(bridge)
        .invoke_handler(tauri::generate_handler![
            settings::validate_model,
            catalogue::content_catalogue,
            recover,
            recovery::recovery_list,
            recovery::recovery_restore,
            recovery::recovery_delete,
            recovery::recovery_discard,
            preferences::preferences_load,
            preferences::preferences_save,
            preferences::profile_image,
            files::choose_file,
            files::choose_export_folder,
            files::set_window_title,
            files::quit_app,
            files::save_file,
            files::open_file,
            bibliography::bibliography_search,
            bibliography::bibliography_index,
            bibliography::validate_bibliography,
            bibliography::model_policy,
            files::new_document,
            files::close_document,
            agent::publish_snapshot,
            agent::agent_state,
            agent::agent_invite,
            agent::agent_revoke,
            agent::agent_reject,
            agent::agent_accept,
            agent::agent_input,
            proofing::spelling_check,
            proofing::spelling_suggestions,
            proofing::count_text,
            rendering::sync_forward,
            rendering::sync_backward,
            rendering::sync_selection,
            join,
            connection,
            relay,
            awareness,
            preview,
            preview_frame,
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
#[cfg_attr(coverage_nightly, coverage(off))]
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
            .render(source, session.assets, 0, Some(export.clone()), None, None)
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
