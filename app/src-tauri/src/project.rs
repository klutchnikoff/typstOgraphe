//! Archive ownership outside the live Yjs engine; no catalogue reconstruction on save.
use std::sync::Mutex;
use typstographe_core::{
    assets::Assets,
    content::{
        archive::Archive,
        contract::State,
        project::{Backup, Project},
    },
    packages::Dependencies,
};

pub const MAX_RECOVERY_BYTES: u64 = 1024 * 1024 * 1024;
pub fn read_recovery_file(path: &std::path::Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_RECOVERY_BYTES {
        return Err("recovery_too_large".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_RECOVERY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_RECOVERY_BYTES {
        return Err("recovery_too_large".into());
    }
    Ok(bytes)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryContext {
    pub archive: Backup,
    pub saved_origin: bool,
}
type CachedBackup = Option<(String, String, Backup)>;
pub struct Context {
    pub owner: Project,
    pub saved_origin: bool,
    backup: Mutex<CachedBackup>,
}
impl Context {
    pub fn new(owner: Project, saved_origin: bool) -> Self {
        Self {
            owner,
            saved_origin,
            backup: Mutex::new(None),
        }
    }
    pub fn recovery(&self, current: Archive) -> Result<RecoveryContext, String> {
        let mut cache = self.backup.lock().map_err(|e| e.to_string())?;
        if let Some((template, model, backup)) = &*cache
            && *template == current.template
            && *model == current.model_id
        {
            return Ok(RecoveryContext {
                archive: backup.clone(),
                saved_origin: self.saved_origin,
            });
        }
        let prepared = self.owner.with_current(current.clone()).map_err(error)?;
        let backup = Backup::capture(&prepared).map_err(error)?;
        *cache = Some((current.template, current.model_id, backup.clone()));
        Ok(RecoveryContext {
            archive: backup,
            saved_origin: self.saved_origin,
        })
    }
}
pub fn error(e: typstographe_core::content::archive::Error) -> String {
    typstographe_i18n::archive_error(&e)
}
pub fn archive(
    state: State,
    update: Vec<u8>,
    assets: Assets,
    dependencies: Dependencies,
) -> Archive {
    Archive {
        document: state.document,
        metadata: state.settings.metadata,
        template: state.settings.template,
        model_id: state.settings.model_id,
        yjs: update,
        assets,
        dependencies,
    }
}
pub fn recovery_for_state(state: State, update: Vec<u8>) -> Result<RecoveryContext, String> {
    let dependencies = typstographe_core::packages::for_template(&state.settings.template);
    let project =
        Project::new(archive(state, update, Assets::default(), dependencies)).map_err(error)?;
    Ok(RecoveryContext {
        archive: Backup::capture(&project).map_err(error)?,
        saved_origin: false,
    })
}

impl crate::session::Session {
    pub fn stored_project(&mut self) -> Result<Project, String> {
        let owner = self.ensure_owner()?;
        let state = owner.state().clone();
        let update = owner.update_bytes();
        let dependencies = self.dependencies_for(&state.settings);
        let current = archive(state, update, self.assets.clone(), dependencies);
        if self.project.get().is_none() {
            let _ = self.project.set(Context::new(
                Project::new(current.clone()).map_err(error)?,
                false,
            ));
        }
        self.project
            .get()
            .expect("initialized owner")
            .owner
            .with_current(current)
            .map_err(error)
    }

    pub fn render_snapshot(
        &self,
        snapshot: &crate::session::Snapshot,
    ) -> Result<(String, Assets, crate::settings::Settings, Dependencies), String> {
        let source = self.validate(snapshot)?;
        let settings = snapshot
            .settings
            .clone()
            .unwrap_or_else(|| self.settings.clone());
        let dependencies = self.dependencies_for(&settings);
        Ok((source, self.assets.clone(), settings, dependencies))
    }
    pub fn dependencies_for(&self, settings: &crate::settings::Settings) -> Dependencies {
        self.project.get().map_or_else(
            || typstographe_core::packages::for_template(&settings.template),
            |context| context.owner.dependencies_for(&settings.template),
        )
    }
    pub fn render_context<'a>(
        &self,
        settings: &'a crate::settings::Settings,
    ) -> crate::rendering::RenderContext<'a> {
        crate::rendering::RenderContext {
            settings,
            dependencies: self.dependencies_for(settings),
        }
    }
    pub fn project_recovery(
        &self,
        state: State,
        update: Vec<u8>,
    ) -> Result<RecoveryContext, String> {
        let dependencies = self.dependencies_for(&state.settings);
        let current = archive(state, update, self.assets.clone(), dependencies);
        if self.project.get().is_none() {
            let owner = Project::new(current.clone()).map_err(error)?;
            let _ = self.project.set(Context::new(owner, false));
        }
        self.project
            .get()
            .expect("initialized project owner")
            .recovery(current)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "project_tests.rs"]
mod tests;
