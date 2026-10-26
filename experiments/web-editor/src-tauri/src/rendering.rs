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
    pub timings: PreviewTimings,
    pub page_sizes: Vec<[f32; 2]>,
}

/// Rust-side stages only: debounce, source projection, IPC and browser decoding
/// are outside this measurement. elapsed_ms remains the existing total counter.
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTimings {
    pub prepare_ms: f64,
    pub compile_ms: f64,
    pub export_ms: f64,
    pub raster_ms: f64,
    pub png_ms: f64,
    pub base64_ms: f64,
    pub sync_ms: f64,
}

fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
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
        let mut timings = PreviewTimings::default();
        let mut world = self.0.lock().map_err(|e| e.to_string())?;
        typstographe_core::packages::validate(&request.template, &request.dependencies)
            .map_err(|e| e.to_string())?;
        world.set_assets(&request.assets);
        world.set_dependencies(&request.dependencies);
        timings.prepare_ms = milliseconds(start);
        let compile_start = Instant::now();
        let document = world
            .compile_with_template(&request.source, &request.metadata, &request.template)
            .map_err(|e| e.to_string())?;
        timings.compile_ms = milliseconds(compile_start);
        let export_start = Instant::now();
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
        timings.export_ms = milliseconds(export_start);
        let mut pages = Vec::new();
        for page in document.pages() {
            let raster_start = Instant::now();
            let pixmap = typst_render::render(page, &typst_render::RenderOptions::default());
            timings.raster_ms += milliseconds(raster_start);
            let png_start = Instant::now();
            let (width, height) = (pixmap.width(), pixmap.height());
            // Own the pixmap: consume its buffer instead of cloning it merely
            // to undo premultiplied alpha for PNG. Preserve transparent models.
            let rgba = pixmap.take_demultiplied();
            let data = preview_png(width, height, &rgba)?;
            timings.png_ms += milliseconds(png_start);
            let base64_start = Instant::now();
            pages.push(format!("data:image/png;base64,{}", STANDARD.encode(data)));
            timings.base64_ms += milliseconds(base64_start);
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
        let sync_start = Instant::now();
        if let Some(map) = map {
            *self.1.lock().map_err(|e| e.to_string())? =
                Some(typstographe_core::navigation::RenderSync::new(
                    revision,
                    world.clone(),
                    document,
                    map,
                ));
        }
        timings.sync_ms = milliseconds(sync_start);
        Ok(Preview {
            revision,
            source,
            pages,
            elapsed_ms: milliseconds(start),
            timings,
            page_sizes,
        })
    }
}

fn preview_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // Interactive local preview favours latency over minimum file size.
        // Compression is lossless; export keeps its own encoding policy.
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(data)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded_png(data: Vec<u8>) -> Vec<u8> {
        let mut reader = png::Decoder::new(std::io::Cursor::new(data))
            .read_info()
            .unwrap();
        assert_eq!(reader.info().color_type, png::ColorType::Rgba);
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let frame = reader.next_frame(&mut pixels).unwrap();
        pixels.truncate(frame.buffer_size());
        pixels
    }

    #[test]
    fn preview_encoding_preserves_opaque_and_transparent_pixels() {
        for fill in ["white", "none"] {
            let settings = crate::settings::Settings {
                template: format!(
                    "#let courrier(body, ..options) = {{\nset page(width: 40pt, height: 40pt, margin: 0pt, fill: {fill})\nbody\n}}"
                ),
                ..Default::default()
            };
            let source = "#rect(width: 20pt, height: 20pt, fill: rgb(\"#80402080\"), stroke: none)";
            let mut world = TypstWorld::default();
            let document = world
                .compile_with_template(source, &settings.metadata, &settings.template)
                .unwrap();
            let raster = typst_render::render(
                &document.pages()[0],
                &typst_render::RenderOptions::default(),
            );
            let expected = decoded_png(raster.encode_png().unwrap());
            assert_eq!(
                expected
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|pixel| pixel[3] == 255),
                fill == "white"
            );
            let preview = Renderer::default()
                .render(
                    source.into(),
                    Assets::default(),
                    1,
                    None,
                    Some(&settings),
                    None,
                )
                .unwrap();
            let bytes = STANDARD
                .decode(
                    preview.pages[0]
                        .strip_prefix("data:image/png;base64,")
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(decoded_png(bytes), expected);
        }
    }

    /// No timing threshold in CI. Both adapters use the same source, assets,
    /// model, raster options and debug profile; compilation caches stay alive.
    #[test]
    #[ignore = "Local performance diagnostic; run explicitly with --ignored --nocapture"]
    fn profile_preview_pipeline() {
        let settings = crate::settings::Settings::default();
        let cases: [(&str, crate::schema::Node); 2] = [
            ("simple", serde_json::from_value(serde_json::json!({"type":"doc","content":[
                {"type":"paragraph","content":[{"type":"text","text":"Madame, Monsieur,"}]},
                {"type":"paragraph","content":[{"type":"text","text":"Je vous remercie pour votre retour et vous prie de me confirmer la disponibilité de la salle pour notre prochaine rencontre."}]}
            ]})).unwrap()),
            ("fixture", serde_json::from_str(include_str!("../../fixture.json")).unwrap()),
        ];
        for (name, document) in cases {
            let mut assets = Assets::default();
            assets.insert(
                "images/parc.svg".into(),
                include_bytes!("../../../../fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
            let renderer = Renderer::default();
            let mut world = TypstWorld::default();
            for iteration in 0..9 {
                let mut document = document.clone();
                document.content.push(serde_json::from_value(serde_json::json!({"type":"paragraph","content":[{"type":"text","text":format!("Mesure numéro {iteration}.")}]})).unwrap());
                let projection_start = Instant::now();
                let (source, map) =
                    typstographe_core::content::source_mapped(&document, &assets).unwrap();
                let projection_ms = milliseconds(projection_start);
                let request = RenderRequest {
                    revision: iteration,
                    source: source.clone(),
                    assets: assets.clone(),
                    dependencies: typstographe_core::packages::for_template(&settings.template),
                    source_map: map.clone(),
                    metadata: settings.metadata.clone(),
                    template: settings.template.clone(),
                    export_dir: None,
                };
                let core = |world: &mut TypstWorld| {
                    typstographe_core::rendering::render(world, &request).unwrap()
                };
                let web = || {
                    renderer
                        .render(
                            source.clone(),
                            assets.clone(),
                            iteration,
                            None,
                            Some(&settings),
                            Some(map.clone()),
                        )
                        .unwrap()
                };
                let (legacy, preview) = if iteration % 2 == 0 {
                    let legacy = core(&mut world);
                    (legacy, web())
                } else {
                    let preview = web();
                    (core(&mut world), preview)
                };
                println!(
                    "PROFILE {}",
                    serde_json::json!({
                        "case":name,"iteration":iteration,"projectionMs":projection_ms,
                        "legacyMs":legacy.elapsed.as_secs_f64()*1000.0,
                        "webMs":preview.elapsed_ms,"stages":preview.timings,
                        "pages":preview.pages.len(),"dimensions":legacy.pages.iter().map(|p|p.size).collect::<Vec<_>>(),
                        "rgbaBytes":legacy.pages.iter().map(|p|p.rgba.len()).sum::<usize>(),
                        "dataUrlBytes":preview.pages.iter().map(|p|p.len()).sum::<usize>(),
                    })
                );
            }
        }
    }
}
