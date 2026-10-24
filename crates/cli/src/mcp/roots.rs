//! Autorisations de chemins de l’adaptateur, indépendantes des verrous du cœur.
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};
use typstographe_core::engine::{CommandError, ErrorCode};

pub struct Roots {
    cwd: PathBuf,
    read: Vec<PathBuf>,
    write: Vec<PathBuf>,
}
impl Roots {
    pub fn new(read: Vec<PathBuf>, write: Vec<PathBuf>) -> Result<Self, CommandError> {
        fn directories(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, CommandError> {
            let mut roots = Vec::new();
            for path in paths {
                let root = path
                    .canonicalize()
                    .map_err(|e| CommandError::detail(ErrorCode::InvalidParams, "mcp_root", e))?;
                if !root.is_dir() {
                    return Err(CommandError::new(ErrorCode::InvalidParams, "mcp_root"));
                }
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
            Ok(roots)
        }
        Ok(Self {
            cwd: std::env::current_dir()
                .and_then(|p| p.canonicalize())
                .map_err(|e| CommandError::detail(ErrorCode::Io, "mcp_root", e))?,
            read: directories(read)?,
            write: directories(write)?,
        })
    }
    pub fn describe(&self) -> Value {
        json!({"read_roots":self.read,"write_roots":self.write,"relative_to":self.cwd,
            "parent_components":false,"symlinks":"resolved before containment checks","os_sandbox":false})
    }
    pub fn can_read(&self) -> bool {
        !self.read.is_empty()
    }
    pub fn can_write(&self) -> bool {
        !self.write.is_empty()
    }
    fn resolve(&self, path: &Path, roots: &[PathBuf]) -> Result<PathBuf, CommandError> {
        if roots.is_empty()
            || path.as_os_str().is_empty()
            || path.components().any(|p| matches!(p, Component::ParentDir))
        {
            return denied();
        }
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        };
        // Un export peut avoir plusieurs parents à créer. Résoudre le premier
        // ancêtre existant puis réattacher les noms manquants sans faire d’IO.
        let mut ancestor = absolute.as_path();
        let mut missing = Vec::new();
        let canonical = loop {
            match ancestor.symlink_metadata() {
                Ok(_) => break ancestor.canonicalize().map_err(access_error)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    missing.push(ancestor.file_name().ok_or_else(access_denied)?.to_owned());
                    ancestor = ancestor.parent().ok_or_else(access_denied)?;
                }
                Err(e) => return Err(access_error(e)),
            }
        };
        let mut target = canonical;
        for name in missing.into_iter().rev() {
            target.push(name);
        }
        // Une racine remplacée par un lien après le démarrage ne donne aucun droit.
        if roots.iter().any(|root| {
            root.is_dir()
                && root.canonicalize().is_ok_and(|current| current == *root)
                && target.starts_with(root)
        }) {
            Ok(target)
        } else {
            denied()
        }
    }
    pub fn read_file(&self, path: &Path) -> Result<PathBuf, CommandError> {
        let path = self.resolve(path, &self.read)?;
        if !path.metadata().map_err(access_error)?.is_file() {
            return denied();
        }
        Ok(path)
    }
    pub fn write(&self, path: &Path) -> Result<PathBuf, CommandError> {
        self.resolve(path, &self.write)
    }
    pub fn document_write(&self, path: &Path) -> Result<PathBuf, CommandError> {
        let target = self.write(path)?;
        regular_or_missing(&target)?;
        // Binding::acquire ouvre aussi ce compagnon : contrôler son éventuel lien.
        let mut name = std::ffi::OsString::from(".");
        name.push(target.file_name().ok_or_else(access_denied)?);
        name.push(".lock");
        let lock = self.write(&target.with_file_name(name))?;
        regular_or_missing(&lock)?;
        Ok(target)
    }
}
fn access_error(error: std::io::Error) -> CommandError {
    CommandError::detail(ErrorCode::Unavailable, "mcp_file_access", error)
}
fn access_denied() -> CommandError {
    CommandError::new(ErrorCode::Unavailable, "mcp_file_access")
}
pub fn denied<T>() -> Result<T, CommandError> {
    Err(access_denied())
}

fn regular_or_missing(path: &Path) -> Result<(), CommandError> {
    match path.symlink_metadata() {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => denied(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(access_error(e)),
    }
}
