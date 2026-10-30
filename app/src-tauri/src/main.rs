#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
use base64::Engine as _;
mod agent;
mod bibliography;
mod catalogue;
#[cfg(feature = "recipe-driver")]
mod driver;
mod files;
mod menus;
mod owner;
mod preferences;
mod proofing;
mod recovery;
mod rendering;
mod schema;
mod session;
mod settings;
#[cfg(feature = "recipe-driver")]
mod shared_diagnostic;
mod structured;

use rendering::{Preview, Renderer};
use session::{Bootstrap, Session, Snapshot};
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
    generation: String,
    revision: u64,
    update: Vec<u8>,
}
#[derive(Clone, serde::Serialize)]
struct Awareness {
    from: String,
    generation: String,
    version: u64,
    update: Vec<u8>,
}

// Tauri dispatches these CPU/lock-bearing calls on its worker pool, never the UI thread.
#[tauri::command(async)]
fn join(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
) -> Result<Bootstrap, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let bootstrap = session.join(window.label());
    app.emit("web-editor:synchronization", session.synchronization())
        .map_err(|e| e.to_string())?;
    Ok(bootstrap)
}
#[tauri::command(async)]
// Three framework-injected handles plus the relay wire fields.
#[allow(clippy::too_many_arguments)]
fn relay(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    update: Vec<u8>,
    generation: String,
    group: Option<u64>,
    capture: Option<bool>,
    history_view: Option<owner::HistoryViewRequest>,
) -> Result<owner::Receipt, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.check_peer(window.label(), &generation)?;
    let receipt = session.relay_view(
        window.label(),
        &update,
        group,
        capture.unwrap_or(true),
        history_view,
    )?;
    if !receipt.update.is_empty() && session.should_broadcast(window.label()) {
        app.emit(
            "web-editor:change",
            Change {
                from: window.label().into(),
                generation,
                revision: receipt.revision,
                update: receipt.update.clone(),
            },
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(receipt)
}
#[tauri::command(async)]
fn editing_history(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    generation: String,
    direction: typstographe_core::content::editing::Direction,
) -> Result<owner::Receipt, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.check_peer(window.label(), &generation)?;
    let receipt = session.history(window.label(), direction)?;
    if !receipt.update.is_empty() && session.should_broadcast(window.label()) {
        app.emit(
            "web-editor:change",
            Change {
                from: window.label().into(),
                generation,
                revision: receipt.revision,
                update: receipt.update.clone(),
            },
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(receipt)
}
#[tauri::command(async)]
fn editing_context(
    window: WebviewWindow,
    state: State<Bridge>,
    generation: String,
) -> Result<typstographe_core::content::editing::ContextSnapshot, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.check_peer(window.label(), &generation)?;
    session.ensure_owner()?.snapshot(window.label())
}
#[tauri::command(async)]
fn set_editing_context(
    window: WebviewWindow,
    state: State<Bridge>,
    request: owner::ContextRequest,
) -> Result<typstographe_core::content::editing::ContextSnapshot, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.check_peer(window.label(), &request.generation)?;
    session.ensure_owner()?.set_context(window.label(), request)
}
#[tauri::command(async)]
fn reset_editing_context(
    window: WebviewWindow,
    state: State<Bridge>,
    generation: String,
) -> Result<typstographe_core::content::editing::ContextSnapshot, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.check_peer(window.label(), &generation)?;
    session.ensure_owner()?.reset(window.label())
}

#[tauri::command(async)]
fn prepare_structured_commands(
    state: State<Bridge>,
    request: structured::Request,
) -> Result<typstographe_core::content::shared::Publication, String> {
    structured::prepare(&*state.session.lock().map_err(|e| e.to_string())?, request)
}
#[tauri::command(async)]
fn commit_structured_commands(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    request: structured::Request,
) -> Result<typstographe_core::content::shared::Publication, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let publication = structured::commit(&mut session, window.label(), request)?;
    if session.should_broadcast(window.label()) {
        for resource in &publication.resources {
            app.emit(
            "web-editor:asset",
            serde_json::json!({"generation": publication.guard.document, "path": resource.path, "data": base64::engine::general_purpose::STANDARD.encode(&resource.data)}),
        )
        .map_err(|e| e.to_string())?;
        }
        if publication.changed {
            app.emit(
                "web-editor:change",
                Change {
                    from: window.label().into(),
                    generation: publication.guard.document.clone(),
                    revision: publication.guard.revision,
                    update: publication.update.clone(),
                },
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(publication)
}
#[tauri::command(async)]
fn connection(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    online: bool,
    generation: String,
) -> Result<session::Synchronization, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.connection(window.label(), &generation, online)?;
    let status = session.synchronization();
    app.emit("web-editor:synchronization", &status)
        .map_err(|e| e.to_string())?;
    Ok(status)
}
#[tauri::command(async)]
fn set_synchronization(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    generation: String,
    enabled: bool,
) -> Result<session::Synchronization, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let status = session.set_synchronization(window.label(), &generation, enabled)?;
    app.emit("web-editor:synchronization", &status)
        .map_err(|e| e.to_string())?;
    Ok(status)
}
#[tauri::command(async)]
fn awareness(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    generation: String,
    version: u64,
    update: Vec<u8>,
) -> Result<(), String> {
    if update.len() > 64 * 1024 {
        return Err("Présence trop volumineuse.".into());
    }
    let session = state.session.lock().map_err(|e| e.to_string())?;
    if !session.accepts_awareness(window.label(), &generation, version)? {
        return Ok(());
    }
    app.emit(
        "web-editor:awareness",
        Awareness {
            from: window.label().into(),
            generation,
            version,
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
#[tauri::command(async)]
fn prepare_image(
    extension: String,
    data: String,
) -> Result<typstographe_core::content::commands::ImageResource, String> {
    use typstographe_core::{assets::MAX_IMAGE_BYTES, content::contract::Error};
    if data.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 {
        return Err(crate::owner::error(Error::StateLimitExceeded));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| {
            crate::owner::error(Error::InvalidParameters {
                parameter: "data".into(),
            })
        })?;
    typstographe_core::content::commands::ImageResource::prepare(&extension, bytes)
        .map_err(crate::owner::error)
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
        if arg == "--diagnostic-synchronization-active" {
            diagnostic = Some("synchronization-active".to_owned());
        }
        if arg == "--diagnostic-synchronization" {
            diagnostic = Some("synchronization".to_owned());
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
    #[cfg(not(feature = "recipe-driver"))]
    if stdio {
        eprintln!("--stdio exige un binaire de recette construit avec --features recipe-driver.");
        std::process::exit(2);
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
    #[cfg(feature = "recipe-driver")]
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
                    session.leave(window.label());
                    let _ = window
                        .app_handle()
                        .emit("web-editor:synchronization", session.synchronization());
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
            menus::set_menu_availability,
            settings::validate_model,
            catalogue::content_catalogue,
            recover,
            recovery::recovery_list,
            recovery::recovery_restore,
            recovery::recovery_delete,
            recovery::recovery_discard,
            preferences::preferences_load,
            preferences::preferences_save,
            files::choose_file,
            files::choose_export_folder,
            files::export_editing_draft,
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
            set_synchronization,
            relay,
            editing_history,
            editing_context,
            set_editing_context,
            reset_editing_context,
            prepare_structured_commands,
            commit_structured_commands,
            awareness,
            preview,
            preview_frame,
            save,
            export_document,
            prepare_image,
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
        assert!(body.contains("table(columns:"));
    }
}
