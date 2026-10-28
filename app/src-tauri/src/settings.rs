pub use typstographe_core::content::contract::Settings;

#[tauri::command]
pub async fn validate_model(
    state: tauri::State<'_, crate::Bridge>,
    snapshot: crate::session::Snapshot,
) -> Result<(), String> {
    let session = state.session.clone();
    let renderer = state.renderer.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let s = session.lock().map_err(|e| e.to_string())?;
        let source = s.validate(&snapshot)?;
        renderer.validate(
            &source,
            &s.assets,
            snapshot.settings.as_ref().unwrap_or(&s.settings),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn embedded_models_need_not_be_installed_but_must_compile_against_the_type() {
        let settings = Settings {
            model_id: "embedded".into(),
            template: format!("{}\n// Modèle personnel", Settings::default().template),
            ..Default::default()
        };
        settings.validate().unwrap();
        let renderer = crate::rendering::Renderer::default();
        let assets = Default::default();
        renderer
            .validate("Texte du courrier", &assets, &settings)
            .unwrap();
        let invalid = Settings {
            template: "#let autre(body) = body".into(),
            ..settings.clone()
        };
        assert!(renderer.validate("Texte", &assets, &invalid).is_err());
        let not_embedded = Settings {
            model_id: Settings::default().model_id,
            ..settings
        };
        not_embedded.validate().unwrap();
        renderer.validate("Texte", &assets, &not_embedded).unwrap();
    }
}
