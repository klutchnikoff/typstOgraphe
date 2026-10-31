#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
use base64::Engine as _;
mod agent;
mod bibliography;
mod catalogue;
mod comparisons;
#[cfg(feature = "recipe-driver")]
mod diagnostics;
#[cfg(feature = "recipe-driver")]
mod driver;
mod files;
mod launch;
mod menus;
mod native_locales;
mod owner;
mod preferences;
mod project;
mod proofing;
mod recovery;
mod rendering;
mod schema;
mod session;
mod settings;
#[cfg(feature = "recipe-driver")]
mod shared_diagnostic;
mod structured;
mod versions;

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
    #[cfg(feature = "recipe-driver")]
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
async fn preview(state: State<'_, Bridge>, snapshot: Snapshot) -> Result<Preview, String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (source, assets, settings, dependencies) = session
            .lock()
            .map_err(|e| e.to_string())?
            .render_snapshot(&snapshot)?;
        let map = typstographe_core::content::source_mapped(&snapshot.document, &assets)?.1;
        let result = renderer.render(
            source,
            assets,
            snapshot.revision,
            None,
            Some(rendering::RenderContext {
                settings: &settings,
                dependencies,
            }),
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
        let (source, assets, settings, dependencies) = session
            .lock()
            .map_err(|e| e.to_string())?
            .render_snapshot(&snapshot)?;
        let map = typstographe_core::content::source_mapped(&snapshot.document, &assets)?.1;
        let bytes = renderer.frame(
            source,
            assets,
            snapshot.revision,
            rendering::RenderContext {
                settings: &settings,
                dependencies,
            },
            map,
            encoding,
        )?;
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
        let (source, assets, settings, dependencies) = session
            .lock()
            .map_err(|e| e.to_string())?
            .render_snapshot(&snapshot)?;
        renderer.render(
            source,
            assets,
            snapshot.revision,
            None,
            Some(rendering::RenderContext {
                settings: &settings,
                dependencies,
            }),
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
    let (source, assets, root, settings, dependencies) = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        (
            session.validate(&snapshot)?,
            session.assets.clone(),
            session.root.clone(),
            snapshot
                .settings
                .clone()
                .unwrap_or_else(|| session.settings.clone()),
            session.dependencies_for(snapshot.settings.as_ref().unwrap_or(&session.settings)),
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
            &dependencies,
        )?;
        renderer.render(
            source,
            assets,
            snapshot.revision,
            Some(path.clone()),
            Some(rendering::RenderContext {
                settings: &settings,
                dependencies,
            }),
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
    let options = launch::Options::parse(std::env::args().skip(1), cfg!(feature = "recipe-driver"))
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        });
    let stdio = options.stdio;
    let diagnostic = options.diagnostic;
    let initial_path = options.initial_path;
    let global_session = options.root.is_none();
    let root = options.root.unwrap_or_default();
    if stdio || diagnostic.is_some() {
        typstographe_i18n::initialize(&["fr".parse().expect("French locale")]);
    }
    let mut session = if diagnostic.as_deref() == Some("draft-recover") {
        Session::fresh(root.clone())
    } else if global_session {
        Session::fresh(PathBuf::new())
    } else {
        #[cfg(feature = "recipe-driver")]
        {
            Session::open(root).expect("Impossible de rouvrir la session de recette")
        }
        #[cfg(not(feature = "recipe-driver"))]
        {
            unreachable!("recipe options were rejected before creating a session")
        }
    };
    if diagnostic.as_deref().is_some_and(|mode| {
        mode.starts_with("closure") || mode.starts_with("publication") || mode == "draft-recover"
    }) {
        // Match the normal welcome document, not the rich historic demo fixture.
        session.fixture = Some(
            serde_json::from_value(
                serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}),
            )
            .expect("empty document"),
        );
    }
    let bridge = Bridge {
        session: Arc::new(Mutex::new(session)),
        renderer: Arc::new(Renderer::default()),
        #[cfg(feature = "recipe-driver")]
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
            comparisons::comparison_open,
            comparisons::comparison_read,
            comparisons::comparison_preview,
            comparisons::commit::comparison_apply,
            comparisons::staging::comparison_prepare,
            comparisons::comparison_close,
            versions::versions_list,
            versions::versions_read,
            versions::versions_change,
            versions::versions_preview,
            menus::set_menu_availability,
            settings::validate_model,
            catalogue::content_catalogue,
            recovery::drafts::open_private_recovery,
            recovery::drafts::conserve_private,
            recovery::commands::work_list,
            recovery::commands::work_delete,
            recovery::commands::work_restore,
            recovery::commands::work_abandon,
            recovery::commands::choose_work_file,
            recovery::commands::work_import,
            recovery::commands::work_export,
            recovery::transition::transition_begin,
            recovery::transition::transition_ack,
            recovery::transition::transition_status,
            recovery::transition::transition_end,
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
            #[cfg(feature = "recipe-driver")]
            diagnostics::diagnostic_mode,
            #[cfg(feature = "recipe-driver")]
            diagnostics::diagnostic_confirmed,
            #[cfg(feature = "recipe-driver")]
            recovery::commands::diagnostic_private_records,
            #[cfg(feature = "recipe-driver")]
            recovery::commands::diagnostic_external_ready,
            #[cfg(feature = "recipe-driver")]
            diagnostics::diagnostic_event,
            #[cfg(feature = "recipe-driver")]
            diagnostics::diagnostic_finish
        ])
        .run(tauri::generate_context!())
        .expect("Erreur de l’application typstOgraphe");
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
