//! Toutes les voies de fichiers passent ici, y compris les étapes et la fin d’un lot.
use super::roots::Roots;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use typstographe_core::engine::*;

pub fn exposed(spec: &CommandSpec) -> bool {
    !spec.id.as_str().starts_with("recovery.") && spec.id.as_str() != "models.presentation"
}
pub struct Policy {
    roots: Roots,
    read_only: bool,
}
impl Policy {
    pub fn new(
        read: Vec<PathBuf>,
        write: Vec<PathBuf>,
        read_only: bool,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            roots: Roots::new(read, write)?,
            read_only,
        })
    }
    pub fn describe(&self) -> Value {
        let mut value = self.roots.describe();
        value["read_only"] = json!(self.read_only);
        value["write_enabled"] = json!(!self.read_only && self.roots.can_write());
        value
    }
    pub fn support(&self, spec: &CommandSpec) -> Value {
        let authorized = exposed(spec)
            && match spec.id.as_str() {
                "document.open" => self.roots.can_read(),
                "document.save" | "document.save_as" | "document.export" => {
                    !self.read_only && self.roots.can_write()
                }
                "document.close" => true,
                _ => !self.read_only || spec.family == Family::Query,
            };
        json!({"exposed":exposed(spec),"authorized":authorized,"read_only":self.read_only,
            "filesystem":self.roots.can_read() || (!self.read_only && self.roots.can_write())})
    }
    pub fn prepare(&self, engine: &Engine, command: &mut Command) -> Result<(), CommandError> {
        if !exposed(
            catalogue()
                .iter()
                .find(|s| s.id.as_str() == command.id())
                .expect("command"),
        ) {
            return super::roots::denied();
        }
        if self.read_only
            && !matches!(
                command,
                Command::CloseDocument(_)
                    | Command::OpenDocument(OpenDocumentParams {
                        read_only: true,
                        ..
                    })
            )
        {
            return Err(CommandError::new(ErrorCode::Unavailable, "mcp_read_only"));
        }
        match command {
            Command::OpenDocument(p) => {
                p.path = self.roots.read_file(&p.path)?;
                if !p.read_only {
                    p.path = self.roots.document_write(&p.path)?;
                }
            }
            Command::SaveDocument(p) => p.path = Some(self.save_path(engine, p.path.as_deref())?),
            Command::SaveAs(p) => p.path = self.roots.document_write(&p.path)?,
            Command::Export(p) => p.directory = self.roots.write(&p.directory)?,
            Command::NewDocument(p) => self.image(p.signature.as_mut())?,
            Command::InsertImage(p) => self.image(Some(&mut p.image))?,
            Command::ReplaceImage(p) => self.image(Some(&mut p.image))?,
            Command::SetSignature(p) => self.image(p.image.as_mut())?,
            Command::SetModel(p) => {
                if let ModelInput::Path { path } = &mut p.model {
                    *path = self.roots.read_file(path)?;
                }
            }
            Command::Batch(p) => match p.save.as_mut() {
                Some(BatchSave::Current {}) => {
                    self.save_path(engine, None)?;
                }
                Some(BatchSave::As { path, .. }) => *path = self.roots.document_write(path)?,
                None => {}
            },
            _ => {}
        }
        Ok(())
    }
    fn save_path(&self, engine: &Engine, path: Option<&Path>) -> Result<PathBuf, CommandError> {
        if !self.roots.can_write() {
            return super::roots::denied();
        }
        let path = path
            .or_else(|| engine.file_path())
            .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "save_path_required"))?;
        self.roots.document_write(path)
    }
    fn image(&self, image: Option<&mut ImageInput>) -> Result<(), CommandError> {
        if let Some(ImageInput::Path { path }) = image {
            *path = self.roots.read_file(path)?;
        }
        Ok(())
    }
}
