//! Immutable file grants for one invitation; never an arbitrary document write root.
use super::roots::{Roots, denied};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use typstographe_core::engine::{Command, CommandError, Engine, TargetGuard};

#[derive(Clone)]
pub(crate) struct FileGrant {
    guard: TargetGuard,
    current: Option<PathBuf>,
    save_root: Roots,
    exports: Roots,
}
impl FileGrant {
    pub(crate) fn new(
        engine: &Engine,
        save_current: bool,
        export_root: Option<PathBuf>,
    ) -> Result<Self, CommandError> {
        let current = if save_current {
            if engine.is_read_only() {
                return denied();
            }
            Some(
                engine
                    .file_path()
                    .ok_or_else(|| {
                        super::proposals::error("Save the document in the application first")
                    })?
                    .canonicalize()
                    .map_err(CommandError::invalid)?,
            )
        } else {
            None
        };
        Ok(Self {
            guard: engine.target_guard(),
            save_root: Roots::new(
                vec![],
                current
                    .iter()
                    .filter_map(|p| p.parent().map(Path::to_path_buf))
                    .collect(),
            )?,
            current,
            exports: Roots::new(vec![], export_root.into_iter().collect())?,
        })
    }
    pub(crate) fn current_path(&self, engine: &Engine) -> Option<PathBuf> {
        let current = self.current.as_ref()?;
        if self.guard.document_id != engine.target_guard().document_id || engine.is_read_only() {
            return None;
        }
        // Recheck both the path binding and its lock companion, including replaced symlinks.
        let bound = self.save_root.document_write(engine.file_path()?).ok()?;
        let granted = self.save_root.document_write(current).ok()?;
        (bound == *current && granted == *current).then(|| current.clone())
    }
    pub(crate) fn export_path(&self, directory: &Path) -> Result<PathBuf, CommandError> {
        self.exports.write(directory)
    }
    pub(crate) fn describe(&self, engine: &Engine) -> Value {
        json!({"current_file":self.current_path(engine),"export_roots":self.exports.describe()["write_roots"],
            "save_as":false,"published_only":true,"export_requires_current_revision_before_write":true})
    }
    pub(crate) fn prepare(
        &self,
        engine: &Engine,
        command: &mut Command,
    ) -> Result<(), CommandError> {
        if self.guard.document_id != engine.target_guard().document_id {
            return denied();
        }
        match command {
            Command::SaveDocument(params) => {
                let Some(current) = self.current_path(engine) else {
                    return denied();
                };
                if let Some(path) = &params.path
                    && self.save_root.document_write(path)? != current
                {
                    return denied();
                }
                // Keep the application's spelling of the existing binding (e.g. relative path).
                params.path = engine.file_path().map(Path::to_path_buf);
                Ok(())
            }
            Command::Export(params) => {
                params.directory = self.export_path(&params.directory)?;
                Ok(())
            }
            _ => denied(),
        }
    }
}
