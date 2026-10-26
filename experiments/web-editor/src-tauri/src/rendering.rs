use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};
use typstographe_core::{
    assets::Assets,
    document::SourceMap,
    rendering::{RenderRequest, TypstWorld},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub revision: u64,
    pub source: String,
    pub pages: Vec<String>,
    pub elapsed_ms: f64,
    pub page_sizes: Vec<[f32; 2]>,
}

#[derive(Default)]
pub struct Renderer(
    pub Mutex<TypstWorld>,
    pub Mutex<Option<typstographe_core::navigation::RenderSync>>,
);
impl Renderer {
    pub fn validate(
        &self,
        source: &str,
        assets: &Assets,
        settings: &crate::settings::Settings,
    ) -> Result<(), String> {
        let mut world = self.0.lock().map_err(|e| e.to_string())?;
        let dependencies = typstographe_core::packages::for_template(&settings.template);
        world.set_assets(assets);
        world.set_dependencies(&dependencies);
        world
            .compile_with_template(source, &settings.metadata, &settings.template)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn render(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        export: Option<PathBuf>,
        settings: Option<&crate::settings::Settings>,
        map: Option<SourceMap>,
    ) -> Result<Preview, String> {
        let fallback = crate::settings::Settings::default();
        let settings = settings.unwrap_or(&fallback);
        let template = settings.template.clone();
        let request = RenderRequest {
            revision,
            source: source.clone(),
            assets,
            dependencies: typstographe_core::packages::for_template(&template),
            source_map: SourceMap::default(),
            metadata: settings.metadata.clone(),
            template,
            export_dir: None,
        };
        let start = Instant::now();
        let mut world = self.0.lock().map_err(|e| e.to_string())?;
        typstographe_core::packages::validate(&request.template, &request.dependencies)
            .map_err(|e| e.to_string())?;
        world.set_assets(&request.assets);
        world.set_dependencies(&request.dependencies);
        let document = world
            .compile_with_template(&request.source, &request.metadata, &request.template)
            .map_err(|e| e.to_string())?;
        if let Some(path) = export {
            typstographe_core::exporting::write_document(
                &document,
                &request,
                &path,
                typstographe_core::engine::ExportFormat::Pdf,
                false,
            )
            .map_err(|e| e.to_string())?;
            // L'appelant a élagué les ressources depuis l'arbre structuré.
            // Compléter le PDF par les sources, dépendances et polices autonomes.
            write(&path, "main.typ", request.metadata.main_source().as_bytes())?;
            write(&path, "contenu.typ", source.as_bytes())?;
            write(&path, "modele.typ", request.template.as_bytes())?;
            write(
                &path,
                "metadonnees.json",
                &serde_json::to_vec_pretty(&request.metadata).map_err(|e| e.to_string())?,
            )?;
            for (name, data) in &request.assets {
                write(&path, name, data)?;
            }
            for (name, data) in request.dependencies.iter() {
                write(&path, name, data)?;
            }
            typstographe_core::fonts::export_used(&document, &path).map_err(|e| e.to_string())?;
        }
        let mut pages = Vec::new();
        for page in document.pages() {
            let data = typst_render::render(page, &typst_render::RenderOptions::default())
                .encode_png()
                .map_err(|e| e.to_string())?;
            pages.push(format!("data:image/png;base64,{}", STANDARD.encode(data)));
        }
        let page_sizes = document
            .pages()
            .iter()
            .map(|page| {
                [
                    page.frame.size().x.to_pt() as f32,
                    page.frame.size().y.to_pt() as f32,
                ]
            })
            .collect();
        if let Some(map) = map {
            *self.1.lock().map_err(|e| e.to_string())? =
                Some(typstographe_core::navigation::RenderSync::new(
                    revision,
                    world.clone(),
                    document,
                    map,
                ));
        }
        Ok(Preview {
            revision,
            source,
            pages,
            elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
            page_sizes,
        })
    }
}
fn write(root: &Path, name: &str, data: &[u8]) -> Result<(), String> {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::File::create_new(path)
        .and_then(|mut file| file.write_all(data))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_forward(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    position: usize,
) -> Result<Option<typstographe_core::navigation::PagePoint>, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    if session.revision != revision {
        return Err("stale".into());
    }
    let cache = state.renderer.1.lock().map_err(|e| e.to_string())?;
    Ok(cache
        .as_ref()
        .filter(|sync| sync.revision == revision)
        .and_then(|sync| sync.forward(position)))
}
#[tauri::command]
pub fn sync_backward(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    location: typstographe_core::navigation::PagePoint,
) -> Result<Option<usize>, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    if session.revision != revision {
        return Err("stale".into());
    }
    let cache = state.renderer.1.lock().map_err(|e| e.to_string())?;
    Ok(cache
        .as_ref()
        .filter(|sync| sync.revision == revision)
        .and_then(|sync| sync.backward(location)))
}
#[tauri::command]
pub fn sync_selection(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    from: usize,
    to: usize,
) -> Result<Vec<typstographe_core::navigation::SelectionHighlight>, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    if session.revision != revision {
        return Err("stale".into());
    }
    let cache = state.renderer.1.lock().map_err(|e| e.to_string())?;
    Ok(cache
        .as_ref()
        .filter(|sync| sync.revision == revision)
        .map(|sync| sync.selection_highlights_exact(from..to))
        .unwrap_or_default())
}
