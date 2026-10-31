use super::super::{archive_error, content_error, decode, invalid};
use super::{Session, error};
use crate::{content::project::VersionChange, protocol::CommandError};
use serde_json::Value;

impl Session {
    pub(super) fn version_query(&self, id: &str, params: &Value) -> Result<Value, CommandError> {
        let project = &self.project()?.storage;
        match id {
            "versions.list" => Ok(project.version_catalogue()),
            "versions.read" => project
                .version_content(decode(params["id"].clone())?)
                .map_err(archive_error),
            _ => Err(content_error(invalid("version_query"))),
        }
    }
    pub(super) fn change_versions(&mut self, change: VersionChange) -> Result<Value, CommandError> {
        self.writable()?;
        let candidate = self
            .stored_project()?
            .prepare_version_change(&change)
            .map_err(archive_error)?;
        let project = self.project.as_mut().unwrap();
        candidate
            .save(
                project
                    .lease
                    .as_mut()
                    .ok_or_else(|| error("save_path_required"))?,
            )
            .map_err(archive_error)?;
        project.storage = candidate;
        project.saved_origin = true;
        project.saved = project.engine.update();
        project.saved_assets = project.engine.assets().keys().cloned().collect();
        Ok(project.storage.version_catalogue())
    }
}
