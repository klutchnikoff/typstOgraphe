mod persistence;
#[cfg(feature = "recipe-driver")]
pub fn close_for_recipe(
    session: &mut crate::session::Session,
    revision: u64,
) -> Result<(), String> {
    persistence::close_project(session, revision)
}
use crate::{Bridge, session::Snapshot, settings::Settings};
use std::path::PathBuf;
use tauri::{Emitter, State};
use tauri_plugin_dialog::DialogExt;
#[cfg(test)]
use typstographe_core::content::archive::Archive;
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle, state: State<Bridge>) -> Result<(), String> {
    {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        session.lifecycle(session.revision)?;
    }
    app.exit(0);
    Ok(())
}
#[tauri::command]
pub fn set_window_title(window: tauri::WebviewWindow, title: String) -> Result<(), String> {
    window.set_title(&title).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn choose_file(
    app: tauri::AppHandle,
    save: bool,
    default_name: Option<String>,
) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let dialog = app
        .dialog()
        .file()
        .set_title(typstographe_i18n::text(if save {
            "save-dialog"
        } else {
            "open-dialog"
        }))
        .add_filter(typstographe_i18n::text("letter-filter"), &["typsto"]);
    if save {
        dialog
            .set_file_name(
                default_name
                    .filter(|s| s.len() <= 255 && !s.contains(['/', '\\', '\0']))
                    .unwrap_or_else(|| "Sans titre.typsto".into()),
            )
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
/// A refused draft may be invalid, so it is explicitly JSON, never a fake .typsto.
#[tauri::command]
pub async fn export_editing_draft(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, Bridge>,
    mut draft: serde_json::Value,
) -> Result<(), String> {
    if draft["format"] != "typstographe-rejected-draft-v1" {
        return Err("Format de brouillon invalide.".into());
    }
    {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        if draft["generation"].as_str() == Some(session.generation.as_str()) {
            draft["confirmed"] =
                serde_json::to_value(session.recovery_checkpoint()?).map_err(|e| e.to_string())?;
            draft["rejectedEdition"] =
                serde_json::to_value(session.ensure_owner()?.draft(window.label())?)
                    .map_err(|e| e.to_string())?;
        }
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name("typstographe-draft.json")
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let Some(path) = rx.await.map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut file = atomic_write_file::AtomicWriteFile::open(path).map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut file, &draft).map_err(|e| e.to_string())?;
        file.commit().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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
        persistence::save_project(
            &session,
            &path,
            snapshot,
            update,
            |source, assets, settings| renderer.validate(source, assets, settings),
        )
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
    let opened_path = path.clone();
    let response = tauri::async_runtime::spawn_blocking(move || {
        persistence::open_project(
            &session,
            &opened_path,
            revision,
            |source, assets, settings| renderer.validate(source, assets, settings),
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = crate::preferences::remember(&app, &path);
    app.emit("app:document", path.to_string_lossy().as_ref())
        .map_err(|e| e.to_string())?;
    Ok(response)
}

#[tauri::command]
pub fn new_document(
    app: tauri::AppHandle,
    state: State<Bridge>,
    settings: Settings,
    revision: u64,
    resources: Option<Vec<crate::session::Resource>>,
) -> Result<(), String> {
    create(
        &mut *state.session.lock().map_err(|e| e.to_string())?,
        settings,
        revision,
        resources.unwrap_or_default(),
    )?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}
/// Same document lifecycle for Tauri and its browser recipe transport.
pub fn create(
    session: &mut crate::session::Session,
    settings: Settings,
    revision: u64,
    resources: Vec<crate::session::Resource>,
) -> Result<(), String> {
    settings
        .validate()
        .map_err(|e| typstographe_i18n::content_error(&e))?;
    session.lifecycle(revision)?;
    let mut assets = typstographe_core::assets::Assets::default();
    if let Some(signature) = settings.metadata.signature_path() {
        let resource = resources
            .iter()
            .find(|r| r.path == signature)
            .ok_or("Signature absente des ressources.")?;
        assets.insert(signature.into(), resource.decoded()?.into());
    }
    let document: typstographe_core::content::Node =
        serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}))
            .unwrap();
    session.mark_clean()?;
    session.file_lease = None;
    session.replace(document, settings, assets, vec![]);
    session.clean_revision = Some(session.revision);
    Ok(())
}
#[tauri::command]
pub fn close_document(
    app: tauri::AppHandle,
    state: State<Bridge>,
    revision: u64,
) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    persistence::close_project(&mut session, revision)?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use typstographe_core::content::{archive::Error, contract, shared::SharedDocument};

    #[test]
    fn existing_article_opens_and_compiles_with_its_original_model_and_history() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/article-working.typsto");
        let before = std::fs::read(&path).unwrap();
        let archive = Archive::read(&path).unwrap();
        SharedDocument::from_update(&archive.yjs)
            .unwrap()
            .verify(&archive.state())
            .unwrap();
        crate::rendering::Renderer::default()
            .validate(
                &archive.validate().unwrap(),
                &archive.assets,
                &archive.state().settings,
            )
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    #[test]
    fn old_formats_are_refused_and_structured_validation_keeps_its_actual_cause() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("legacy.typsto");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&legacy).unwrap());
        writer
            .start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer
            .write_all(br#"{"format":"typstOgraphe","version":9}"#)
            .unwrap();
        writer.finish().unwrap();
        assert!(matches!(
            Archive::read(&legacy),
            Err(Error::Core(
                typstographe_core::errors::CoreError::StorageError(
                    typstographe_core::errors::StorageError::Unsupported
                )
            ))
        ));

        let original =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/article-working.typsto");
        let corrupt = dir.path().join("corrupt.typsto");
        let mut source = zip::ZipArchive::new(std::fs::File::open(original).unwrap()).unwrap();
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&corrupt).unwrap());
        for index in 0..source.len() {
            let mut entry = source.by_index(index).unwrap();
            let mut bytes = vec![];
            entry.read_to_end(&mut bytes).unwrap();
            let name = entry.name().unwrap().into_owned();
            if name == "contenu.typ" {
                bytes = b"A conflicting source".to_vec();
            }
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
        let error = Archive::read(&corrupt).err().unwrap();
        assert!(
            matches!(&error, Error::Content(contract::Error::InconsistentState { field }) if field == "source")
        );
        let message = typstographe_i18n::archive_error(&error);
        assert!(!message.contains("StorageError"));
        assert!(!message.contains("UnknownEntry"));
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "files/persistence_tests.rs"]
mod persistence_tests;
