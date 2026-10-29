use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex, time::Instant};
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

/// Rust-side stages only: scheduling, source projection, IPC and browser decoding
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
    pub svg_ms: f64,
}

fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// Real native IPC candidates, also used by the repeatable WebView benchmark.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Png,
    Rgba,
    Svg,
    #[serde(skip)]
    Dataurl,
}
struct Output {
    preview: Preview,
    buffers: Vec<Vec<u8>>,
    dimensions: Vec<[u32; 2]>,
}
impl Output {
    fn packet(self, encoding: Encoding) -> Result<Vec<u8>, String> {
        let mut offset = 0;
        let pages: Vec<_> = self.buffers.iter().zip(&self.dimensions).map(|(bytes, size)| {
            let page = serde_json::json!({"encoding": encoding, "width": size[0], "height": size[1], "offset": offset, "length": bytes.len()});
            offset += bytes.len();
            page
        }).collect();
        let header =
            serde_json::to_vec(&serde_json::json!({"preview": self.preview, "pages": pages}))
                .map_err(|e| e.to_string())?;
        let length = u32::try_from(header.len()).map_err(|e| e.to_string())?;
        let mut packet = Vec::with_capacity(8 + header.len() + offset);
        packet.extend_from_slice(b"TOG1");
        packet.extend_from_slice(&length.to_le_bytes());
        packet.extend(header);
        for bytes in self.buffers {
            packet.extend(bytes);
        }
        Ok(packet)
    }
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
            .validate_model(source, &settings.metadata, &settings.template)
            .map(|_| ())
            .map_err(|e| typstographe_i18n::error(&e))
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
        self.render_output(
            source,
            assets,
            revision,
            export,
            settings,
            map,
            Encoding::Dataurl,
        )
        .map(|output| output.preview)
    }
    pub fn frame(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        settings: &crate::settings::Settings,
        map: SourceMap,
        encoding: Encoding,
    ) -> Result<Vec<u8>, String> {
        self.render_output(
            source,
            assets,
            revision,
            None,
            Some(settings),
            Some(map),
            encoding,
        )?
        .packet(encoding)
    }
    // One compilation/validation/synchronisation path for all preview transports.
    #[allow(clippy::too_many_arguments)]
    fn render_output(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        export: Option<PathBuf>,
        settings: Option<&crate::settings::Settings>,
        map: Option<SourceMap>,
        encoding: Encoding,
    ) -> Result<Output, String> {
        let fallback = crate::settings::Settings::default();
        let settings = settings.unwrap_or(&fallback);
        if export.is_some() && !settings.metadata.ready_for_export() {
            return Err(typstographe_i18n::text("export-required-fields"));
        }
        let template = settings.template.clone();
        let request = RenderRequest {
            source: source.clone(),
            assets,
            dependencies: typstographe_core::packages::for_template(&template),

            metadata: settings.metadata.clone(),
            template,
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
                typstographe_core::protocol::ExportFormat::Both,
                false,
            )
            .map_err(|e| e.to_string())?;
        }
        timings.export_ms = milliseconds(export_start);
        let mut pages = Vec::new();
        let mut buffers = Vec::new();
        let mut dimensions = Vec::new();
        for page in document.pages() {
            if matches!(encoding, Encoding::Svg) {
                let svg_start = Instant::now();
                buffers.push(typst_svg::svg(page, &typst_svg::SvgOptions::default()).into_bytes());
                dimensions.push([0, 0]);
                timings.svg_ms += milliseconds(svg_start);
                continue;
            }
            let raster_start = Instant::now();
            let pixmap = typst_render::render(page, &typst_render::RenderOptions::default());
            timings.raster_ms += milliseconds(raster_start);
            let png_start = Instant::now();
            let (width, height) = (pixmap.width(), pixmap.height());
            let rgba = pixmap.take_demultiplied();
            dimensions.push([width, height]);
            if matches!(encoding, Encoding::Rgba) {
                buffers.push(rgba);
                continue;
            }
            let data = preview_png(width, height, &rgba)?;
            timings.png_ms += milliseconds(png_start);
            if matches!(encoding, Encoding::Dataurl) {
                let base64_start = Instant::now();
                pages.push(format!("data:image/png;base64,{}", STANDARD.encode(data)));
                timings.base64_ms += milliseconds(base64_start);
            } else {
                buffers.push(data);
            }
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
        Ok(Output {
            preview: Preview {
                revision,
                source,
                pages,
                elapsed_ms: milliseconds(start),
                timings,
                page_sizes,
            },
            buffers,
            dimensions,
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
/// A stale request fails; a preview rendered for another revision answers nothing.
fn current_sync<T>(
    session: &Mutex<crate::session::Session>,
    renderer: &Renderer,
    revision: u64,
    query: impl FnOnce(&typstographe_core::navigation::RenderSync) -> Option<T>,
) -> Result<Option<T>, String> {
    let session = session.lock().map_err(|e| e.to_string())?;
    if session.revision != revision {
        return Err("stale".into());
    }
    let cache = renderer.1.lock().map_err(|e| e.to_string())?;
    Ok(cache
        .as_ref()
        .filter(|sync| sync.revision == revision)
        .and_then(query))
}
#[tauri::command]
pub fn sync_forward(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    position: usize,
) -> Result<Option<typstographe_core::navigation::PagePoint>, String> {
    current_sync(&state.session, &state.renderer, revision, |sync| {
        sync.forward(position)
    })
}
#[tauri::command]
pub fn sync_backward(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    location: typstographe_core::navigation::PagePoint,
) -> Result<Option<usize>, String> {
    current_sync(&state.session, &state.renderer, revision, |sync| {
        sync.backward(location)
    })
}
#[tauri::command]
pub fn sync_selection(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    from: usize,
    to: usize,
) -> Result<Vec<typstographe_core::navigation::SelectionHighlight>, String> {
    current_sync(&state.session, &state.renderer, revision, |sync| {
        Some(sync.selection_highlights_exact(from..to))
    })
    .map(Option::unwrap_or_default)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn structured_export_includes_original_bibliography_files_and_retained_images() {
        let mut meta = typstographe_core::document_types::article::Metadata::demo();
        let reference = typstographe_core::bibliography::ReferenceFile {
            id: "books".into(),
            name: "books.bib".into(),
            contents:
                "@book{key,author={Bourdieu, Pierre},title={Le sens pratique},year={1980}}\r\n"
                    .into(),
        };
        meta.references.push(reference.clone());
        let settings = crate::settings::Settings {
            metadata: typstographe_core::document_types::Metadata::Article(meta),
            template: typstographe_core::document_types::DocumentType::Article.default_model(),
            model_id: "article.model.working".into(),
        };
        let image = "images/retained.svg";
        let assets: Assets = [(
            image.into(),
            include_bytes!("../../../fixtures/parc.svg")
                .as_slice()
                .into(),
        )]
        .into_iter()
        .collect();
        let node = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"citation","attrs":{"items":[{"file_id":"books","key":"key"}],"form":"normal"}}]},{"type":"figure","attrs":{"src":image,"alt":"Un parc"}}]})).unwrap();
        let source = typstographe_core::content::source(&node, &assets).unwrap();
        let root = tempfile::tempdir().unwrap();
        Renderer::default()
            .render(
                source,
                assets,
                1,
                Some(root.path().into()),
                Some(&settings),
                None,
            )
            .unwrap();
        assert_eq!(
            std::fs::read(root.path().join(reference.path())).unwrap(),
            reference.contents.as_bytes()
        );
        assert!(root.path().join(image).is_file());
        assert!(root.path().join("capabilities/references.typ").is_file());
        assert!(root.path().join("article.pdf").is_file());
        assert!(
            std::fs::read_to_string(root.path().join("main.typ"))
                .unwrap()
                .contains("path(\"bibliography/books.bib\")")
        );
    }

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
            let packet = Renderer::default()
                .frame(
                    source.into(),
                    Assets::default(),
                    1,
                    &settings,
                    SourceMap::default(),
                    Encoding::Rgba,
                )
                .unwrap();
            assert_eq!(&packet[..4], b"TOG1");
            let length = u32::from_le_bytes(packet[4..8].try_into().unwrap()) as usize;
            let header: serde_json::Value = serde_json::from_slice(&packet[8..8 + length]).unwrap();
            assert_eq!(header["pages"][0]["encoding"], "rgba");
            assert_eq!(header["pages"][0]["width"], 80);
            assert_eq!(header["pages"][0]["height"], 80);
            assert_eq!(header["pages"][0]["offset"], 0);
            assert_eq!(header["pages"][0]["length"], expected.len());
            assert_eq!(&packet[8 + length..], expected);
        }
    }

    fn small_pages() -> crate::settings::Settings {
        crate::settings::Settings {
            template: "#let courrier(body, ..options) = {\nset page(width: 40pt, height: 30pt, margin: 0pt)\nbody\n}".into(),
            ..Default::default()
        }
    }

    #[test]
    fn frame_packets_index_every_page_for_each_native_encoding() {
        let source = "Un\n#pagebreak()\nDeux";
        for encoding in [Encoding::Png, Encoding::Svg, Encoding::Rgba] {
            let packet = Renderer::default()
                .frame(
                    source.into(),
                    Assets::default(),
                    7,
                    &small_pages(),
                    SourceMap::default(),
                    encoding,
                )
                .unwrap();
            assert_eq!(&packet[..4], b"TOG1");
            let length = u32::from_le_bytes(packet[4..8].try_into().unwrap()) as usize;
            let header: serde_json::Value = serde_json::from_slice(&packet[8..8 + length]).unwrap();
            let body = &packet[8 + length..];
            assert_eq!(header["preview"]["revision"], 7);
            assert_eq!(
                header["preview"]["pageSizes"],
                serde_json::json!([[40.0, 30.0], [40.0, 30.0]])
            );
            let pages = header["pages"].as_array().unwrap();
            assert_eq!(pages.len(), 2);
            let mut offset = 0;
            for page in pages {
                assert_eq!(page["offset"], offset);
                let size = page["length"].as_u64().unwrap() as usize;
                let bytes = &body[offset..offset + size];
                let (width, height) = (
                    page["width"].as_u64().unwrap(),
                    page["height"].as_u64().unwrap(),
                );
                match encoding {
                    Encoding::Png => {
                        assert_eq!(decoded_png(bytes.to_vec()).len() as u64, width * height * 4)
                    }
                    Encoding::Rgba => assert_eq!(size as u64, width * height * 4),
                    Encoding::Svg => {
                        assert_eq!((width, height), (0, 0));
                        assert!(std::str::from_utf8(bytes).unwrap().starts_with("<svg"));
                    }
                    Encoding::Dataurl => unreachable!(),
                }
                offset += size;
            }
            assert_eq!(offset, body.len());
        }
    }

    #[test]
    fn incomplete_metadata_or_invalid_source_writes_nothing_and_keeps_the_previous_sync() {
        let renderer = Renderer::default();
        renderer
            .frame(
                "Valide".into(),
                Assets::default(),
                1,
                &small_pages(),
                SourceMap::default(),
                Encoding::Rgba,
            )
            .unwrap();
        let root = tempfile::tempdir().unwrap();
        let article = typstographe_core::document_types::DocumentType::Article;
        let settings = crate::settings::Settings {
            metadata: typstographe_core::document_types::Metadata::blank(article),
            template: article.default_model(),
            model_id: "article.model.working".into(),
        };
        assert!(!settings.metadata.ready_for_export());
        assert!(
            renderer
                .render(
                    "Texte".into(),
                    Assets::default(),
                    2,
                    Some(root.path().into()),
                    Some(&settings),
                    Some(SourceMap::default())
                )
                .is_err()
        );
        assert!(
            renderer
                .frame(
                    "#let".into(),
                    Assets::default(),
                    3,
                    &small_pages(),
                    SourceMap::default(),
                    Encoding::Rgba
                )
                .is_err()
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        assert_eq!(renderer.1.lock().unwrap().as_ref().unwrap().revision, 1);
    }

    #[test]
    fn synchronisation_refuses_stale_requests_and_ignores_a_preview_of_another_revision() {
        let session = Mutex::new(crate::session::Session::fresh(PathBuf::new()));
        let renderer = Renderer::default();
        let current = session.lock().unwrap().revision;
        let query = |_: &typstographe_core::navigation::RenderSync| Some(());
        assert_eq!(current_sync(&session, &renderer, current, query), Ok(None));
        renderer
            .frame(
                "Texte".into(),
                Assets::default(),
                current,
                &small_pages(),
                SourceMap::default(),
                Encoding::Rgba,
            )
            .unwrap();
        assert_eq!(
            current_sync(&session, &renderer, current, query),
            Ok(Some(()))
        );
        assert_eq!(
            current_sync(&session, &renderer, current + 1, query),
            Err("stale".into())
        );
        session.lock().unwrap().revision += 1;
        assert_eq!(
            current_sync(&session, &renderer, current + 1, query),
            Ok(None)
        );
    }

    /// No timing threshold in CI. Measure the current pipeline with warm caches.
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
                include_bytes!("../../../fixtures/parc.svg")
                    .as_slice()
                    .into(),
            );
            let renderer = Renderer::default();
            for iteration in 0..9 {
                let mut document = document.clone();
                document.content.push(serde_json::from_value(serde_json::json!({"type":"paragraph","content":[{"type":"text","text":format!("Mesure numéro {iteration}.")}]})).unwrap());
                let projection_start = Instant::now();
                let (source, map) =
                    typstographe_core::content::source_mapped(&document, &assets).unwrap();
                let projection_ms = milliseconds(projection_start);
                let preview = renderer
                    .render(
                        source,
                        assets.clone(),
                        iteration,
                        None,
                        Some(&settings),
                        Some(map),
                    )
                    .unwrap();
                println!(
                    "PROFILE {}",
                    serde_json::json!({
                        "case":name,"iteration":iteration,"projectionMs":projection_ms,
                        "webMs":preview.elapsed_ms,"stages":preview.timings,
                        "pages":preview.pages.len(),"dimensions":preview.page_sizes,
                        "dataUrlBytes":preview.pages.iter().map(|p|p.len()).sum::<usize>(),
                    })
                );
            }
        }
    }
}
