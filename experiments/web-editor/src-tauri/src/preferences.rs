use crate::session::Resource;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};
use tauri::{Manager, State};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub personal: BTreeMap<String, Vec<String>>,
    pub appearance: String,
    pub font: String,
    pub size: f64,
    pub profile: BTreeMap<String, String>,
    pub signature: Option<Resource>,
    pub recent: Vec<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            personal: Default::default(),
            appearance: "system".into(),
            font: "serif".into(),
            size: 18.,
            profile: Default::default(),
            signature: None,
            recent: vec![],
        }
    }
}
impl Preferences {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join("preferences.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        if metadata.len() > 16 * 1024 * 1024 {
            return Err("Préférences trop volumineuses.".into());
        }
        let result: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), String> {
        if !["system", "light", "dark"].contains(&self.appearance.as_str())
            || !["serif", "sans", "mono"].contains(&self.font.as_str())
            || !self.size.is_finite()
            || !(12.0..=32.0).contains(&self.size)
            || self.recent.len() > 20
            || self.profile.values().any(|s| s.len() > 16_000)
        {
            return Err("Préférences invalides.".into());
        }
        if let Some(signature) = &self.signature {
            signature.decoded()?;
        }
        Ok(())
    }
    pub fn save(&self, root: &Path) -> Result<(), String> {
        self.validate()?;
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let options = atomic_write_file::AtomicWriteFile::options();
        #[cfg(unix)]
        let options = {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = options;
            options.mode(0o600);
            options
        };
        let mut file = options
            .open(root.join("preferences.json"))
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.commit().map_err(|e| e.to_string())
    }
}
pub fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let state = app.state::<crate::Bridge>();
    if state.diagnostic.is_some() {
        return Ok(state
            .session
            .lock()
            .map_err(|e| e.to_string())?
            .root
            .join("preferences"));
    }
    app.path().app_config_dir().map_err(|e| e.to_string())
}
pub fn remember(app: &tauri::AppHandle, path: &Path) -> Result<(), String> {
    let root = root(app)?;
    let mut preferences = Preferences::load(&root)?;
    let path = path.to_string_lossy().into_owned();
    preferences.recent.retain(|previous| previous != &path);
    preferences.recent.insert(0, path);
    preferences.recent.truncate(12);
    preferences.save(&root)
}
#[tauri::command]
pub fn preferences_load(
    app: tauri::AppHandle,
    state: State<crate::Bridge>,
) -> Result<Preferences, String> {
    let preferences = Preferences::load(&root(&app)?)?;
    if let Some(signature) = &preferences.signature {
        let bytes = signature.decoded()?;
        state
            .session
            .lock()
            .map_err(|e| e.to_string())?
            .assets
            .insert(signature.path.clone(), bytes.into());
    }
    Ok(preferences)
}
#[tauri::command]
pub fn preferences_save(app: tauri::AppHandle, preferences: Preferences) -> Result<(), String> {
    preferences.save(&root(&app)?)
}
#[tauri::command]
pub fn profile_image(
    state: State<crate::Bridge>,
    extension: String,
    data: Vec<u8>,
) -> Result<Resource, String> {
    state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .import(&extension, &data)
}
