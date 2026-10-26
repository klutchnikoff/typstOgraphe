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
    document_types::{DocumentType, Metadata},
    rendering::{RenderRequest, TypstWorld},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub revision: u64,
    pub source: String,
    pub pages: Vec<String>,
    pub elapsed_ms: f64,
}

#[derive(Default)]
pub struct Renderer(pub Mutex<TypstWorld>);
impl Renderer {
    pub fn render(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        export: Option<PathBuf>,
    ) -> Result<Preview, String> {
        let template = DocumentType::Courrier.default_model();
        let request = RenderRequest {
            revision,
            source: source.clone(),
            assets,
            dependencies: typstographe_core::packages::for_template(&template),
            source_map: SourceMap::default(),
            metadata: Metadata::demo(DocumentType::Courrier),
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
            // Le projecteur de production ignore encore tableaux et notes : cet
            // export expérimental conserve toutes les ressources de la session.
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
        Ok(Preview {
            revision,
            source,
            pages,
            elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
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
