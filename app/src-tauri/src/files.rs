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
            DocumentLease::acquire(&canonical).map_err(|e| typstographe_i18n::command_error(&e))?
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
        archive
            .save(&mut lease)
            .map_err(|e| typstographe_i18n::archive_error(&e))?;
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
            .map_err(|e| typstographe_i18n::command_error(&e))?;
        let archive = read_project(&path).map_err(|e| {
            eprintln!("Project open failed: {e}");
            typstographe_i18n::archive_error(&e)
        })?;
        let settings = Settings {
            metadata: archive.metadata.clone(),
            template: archive.template.clone(),
            model_id: archive.model_id.clone(),
        };
        settings
            .validate()
            .map_err(|e| typstographe_i18n::content_error(&e))?;
        renderer.validate(
            &archive
                .validate()
                .map_err(|e| typstographe_i18n::archive_error(&e))?,
            &archive.assets,
            &settings,
        )?;
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
    agent.snapshot = None;
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
    session.lifecycle(revision)?;
    session.mark_clean()?;
    session.file_lease = None;
    session.opened = false;
    session.owner = None;
    session.generation = uuid::Uuid::new_v4().to_string();
    session.revision += 1;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    agent.snapshot = None;
    app.emit("app:document", ()).map_err(|e| e.to_string())
}

/// Select a reader by the recognized format, never by failure of validation.
pub(crate) fn read_project(
    path: &std::path::Path,
) -> Result<Archive, typstographe_core::content::archive::Error> {
    use typstographe_core::content::{archive::Error, contract};
    match Archive::read(path) {
        Err(Error::LegacyFormat) => {
            let legacy = typstographe_core::storage::read(path)?;
            Ok(Archive {
                document: typstographe_core::content::legacy::import(&legacy.source)
                    .map_err(|reason| contract::Error::InvalidDocument { reason })?,
                metadata: legacy.metadata,
                model_id: "embedded".into(),
                template: legacy.template,
                assets: legacy.assets,
                dependencies: legacy.dependencies,
                yjs: vec![],
            })
        }
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use typstographe_core::content::{archive::Error, contract, shared::SharedDocument};

    #[test]
    fn existing_article_opens_and_compiles_with_its_original_model_and_history() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/article-working.typsto");
        let before = std::fs::read(&path).unwrap();
        let archive = read_project(&path).unwrap();
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
    fn reader_selection_is_by_format_and_keeps_the_actual_structured_cause() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("legacy.typsto");
        typstographe_core::storage::save(
            &legacy,
            &typstographe_core::storage::StoredDocument::blank(
                typstographe_core::document_types::DocumentType::Courrier,
            ),
        )
        .unwrap();
        assert!(matches!(Archive::read(&legacy), Err(Error::LegacyFormat)));
        read_project(&legacy).unwrap().validate().unwrap();

        let original =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/article-working.typsto");
        let corrupt = dir.path().join("corrupt.typsto");
        let mut source = zip::ZipArchive::new(std::fs::File::open(original).unwrap()).unwrap();
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&corrupt).unwrap());
        for index in 0..source.len() {
            let mut entry = source.by_index(index).unwrap();
            let mut bytes = vec![];
            entry.read_to_end(&mut bytes).unwrap();
            if entry.name() == "contenu.typ" {
                bytes = b"A conflicting source".to_vec();
            }
            writer
                .start_file(entry.name(), zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
        let error = read_project(&corrupt).err().unwrap();
        assert!(
            matches!(&error, Error::Content(contract::Error::InconsistentState { field }) if field == "source")
        );
        let message = typstographe_i18n::archive_error(&error);
        assert!(!message.contains("StorageError"));
        assert!(!message.contains("UnknownEntry"));
    }
}
