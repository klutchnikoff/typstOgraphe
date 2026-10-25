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
    attached: bool,
    files: Option<super::file_grant::FileGrant>,
    file_permissions: Value,
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
            attached: false,
            files: None,
            file_permissions: Value::Null,
        })
    }
    pub(crate) fn attached_read_only() -> Result<Self, CommandError> {
        let mut policy = Self::new(vec![], vec![], true)?;
        policy.attached = true;
        Ok(policy)
    }
    pub(crate) fn attached_proposals() -> Result<Self, CommandError> {
        let mut policy = Self::new(vec![], vec![], false)?;
        policy.attached = true;
        Ok(policy)
    }
    pub(crate) fn is_attached(&self) -> bool {
        self.attached
    }
    pub(crate) fn with_files(
        mut self,
        engine: &Engine,
        files: super::file_grant::FileGrant,
    ) -> Self {
        self.file_permissions = files.describe(engine);
        self.files = Some(files);
        self
    }
    pub(crate) fn allows_attached_command(&self, name: &str) -> bool {
        !self.read_only
            && self.files.is_some()
            && match name {
                "document.save" => !self.file_permissions["current_file"].is_null(),
                "document.export" => self.file_permissions["export_roots"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty()),
                _ => false,
            }
    }
    pub(crate) fn allows_proposals(&self) -> bool {
        self.attached && !self.read_only
    }
    pub fn describe(&self) -> Value {
        let mut value = self.roots.describe();
        value["session_mode"] = json!(if self.attached {
            "attached"
        } else {
            "autonomous"
        });
        value["read_only"] = json!(self.read_only);
        value["write_enabled"] = json!(!self.read_only && self.roots.can_write());
        value["proposal_enabled"] = json!(self.allows_proposals());
        value["human_approval_required"] = json!(self.allows_proposals());
        value["direct_edit_enabled"] = json!(!self.attached && !self.read_only);
        if self.attached {
            value["attached_files"] = self.file_permissions.clone();
            value["write_enabled"] = json!(
                self.allows_attached_command("document.save")
                    || self.allows_attached_command("document.export")
            );
        }
        value
    }
    pub fn support(&self, spec: &CommandSpec) -> Value {
        let authorized = exposed(spec)
            && (!self.attached
                || spec.family == Family::Query
                || self.allows_attached_command(spec.id.as_str()))
            && match spec.id.as_str() {
                "document.open" => self.roots.can_read(),
                "document.save" | "document.save_as" | "document.export" => {
                    !self.read_only
                        && (self.roots.can_write()
                            || self.allows_attached_command(spec.id.as_str()))
                }
                "document.close" => true,
                _ => !self.read_only || spec.family == Family::Query,
            };
        json!({"exposed":exposed(spec),"authorized":authorized,"proposal_supported":self.allows_proposals() && spec.batch_step,"read_only":self.read_only,
            "filesystem":self.roots.can_read() || (!self.read_only && self.roots.can_write()) || self.allows_attached_command(spec.id.as_str())})
    }
    pub fn prepare(&self, engine: &Engine, command: &mut Command) -> Result<(), CommandError> {
        if self.attached && matches!(command, Command::SaveDocument(_) | Command::Export(_)) {
            return self
                .files
                .as_ref()
                .ok_or_else(|| CommandError::new(ErrorCode::Unavailable, "mcp_file_access"))?
                .prepare(engine, command);
        }
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
