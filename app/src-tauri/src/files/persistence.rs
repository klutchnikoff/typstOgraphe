//! Native project IO: validate outside the session lock, recheck before publication.
use crate::{
    session::{Resource, Session, Snapshot},
    settings::Settings,
};
use std::{path::Path, sync::Mutex};
use typstographe_core::{DocumentLease, assets::Assets, content::archive::Archive};

pub(super) fn save_project(
    session: &Mutex<Session>,
    path: &Path,
    snapshot: Snapshot,
    update: Vec<u8>,
    validate: impl FnOnce(&str, &Assets, &Settings) -> Result<(), String>,
) -> Result<(), String> {
    if path.extension().and_then(|e| e.to_str()) != Some("typsto") {
        return Err("Extension .typsto attendue.".into());
    }
    let (checkpoint, assets) = {
        let s = session.lock().map_err(|e| e.to_string())?;
        (s.checkpoint(&snapshot, update)?, s.assets.clone())
    };
    let settings = checkpoint.settings;
    let source = checkpoint.source;
    validate(&source, &assets, &settings)?;
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
    let same_target = s.file_lease.as_ref().is_some_and(|l| l.path() == canonical);
    let mut lease = if same_target {
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
        yjs: checkpoint.update,
    };
    archive
        .save(&mut lease)
        .map_err(|e| typstographe_i18n::archive_error(&e))?;
    if let Err(error) = s.mark_clean() {
        // The archive commit succeeded but cleanup failed. Keep the original
        // binding after a refused Save As; otherwise a later ordinary Save
        // could acquire the old path afresh and overwrite an external edit.
        // For an in-place save only, retain our own newly written fingerprint.
        if same_target {
            s.file_lease = Some(lease);
        }
        return Err(error);
    }
    s.file_lease = Some(lease);
    s.recovered_name = None;
    Ok(())
}

pub(super) fn open_project(
    session: &Mutex<Session>,
    path: &Path,
    revision: u64,
    validate: impl FnOnce(&str, &Assets, &Settings) -> Result<(), String>,
) -> Result<serde_json::Value, String> {
    session
        .lock()
        .map_err(|e| e.to_string())?
        .lifecycle(revision)?;
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let lease = session
        .lock()
        .map_err(|e| e.to_string())?
        .file_lease
        .as_ref()
        .filter(|lease| lease.path() == canonical)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| DocumentLease::acquire(path))
        .map_err(|e| typstographe_i18n::command_error(&e))?;
    let archive = Archive::read(path).map_err(|e| {
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
    validate(
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
        // Reading and compiling happen outside the session lock. A different
        // process may have replaced the target while either was in progress.
        lease
            .check()
            .map_err(|e| typstographe_i18n::command_error(&e))?;
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
    let response = serde_json::json!({
        "document":archive.document,
        "settings":settings,
        "resources":resources,
        "yjs":archive.yjs,
    });
    Ok(response)
}

/// Explicitly confirmed document closure. Refusal must precede cleanup.
pub(super) fn close_project(session: &mut Session, revision: u64) -> Result<(), String> {
    session.lifecycle(revision)?;
    session.mark_clean()?;
    session.file_lease = None;
    session.opened = false;
    session.owner = None;
    session.reset_document_generation();
    session.revision += 1;
    Ok(())
}
