pub mod delta;
use crate::error::CommandError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
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
    pub references: Vec<typstographe_core::content::references::ResolvedReference>,
    pub source: String,
    pub pages: Vec<String>,
    pub elapsed_ms: f64,
    pub timings: PreviewTimings,
    pub page_sizes: Vec<[f32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<delta::Metadata>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Start,
    Compiled,
    Page,
    Synchronization,
    Packet,
}
struct Control<'a> {
    ticket: &'a delta::Ticket,
    current: &'a dyn Fn(Stage) -> bool,
}
fn check(control: Option<&Control<'_>>, stage: Stage) -> Result<(), CommandError> {
    if control.is_some_and(|c| !c.ticket.current() || !(c.current)(stage)) {
        return Err("stale".into());
    }
    Ok(())
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
    indices: Vec<usize>,
    keys: Vec<String>,
    sync: Option<Arc<typstographe_core::navigation::RenderSync>>,
}
impl Output {
    fn packet(self, encoding: Encoding) -> Result<Vec<u8>, CommandError> {
        let mut offset = 0;
        let pages: Vec<_> = self.buffers.iter().zip(&self.dimensions).enumerate().map(|(i, (bytes, size))| {
            let page = serde_json::json!({"encoding": encoding, "width": size[0], "height": size[1], "offset": offset, "length": bytes.len()});
            let mut page = page;
            if self.preview.delta.is_some() { page["index"] = serde_json::json!(self.indices[i]); }
            offset += bytes.len();
            page
        }).collect();
        let header =
            serde_json::to_vec(&serde_json::json!({"preview": self.preview, "pages": pages}))?;
        let length = u32::try_from(header.len())?;
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
    pub Mutex<Option<Arc<typstographe_core::navigation::RenderSync>>>,
    pub Mutex<delta::Views>,
);
pub struct RenderContext<'a> {
    pub settings: &'a crate::settings::Settings,
    pub dependencies: typstographe_core::packages::Dependencies,
}
impl<'a> From<&'a crate::settings::Settings> for RenderContext<'a> {
    fn from(settings: &'a crate::settings::Settings) -> Self {
        Self {
            settings,
            dependencies: typstographe_core::packages::for_template(&settings.template),
        }
    }
}
impl Renderer {
    pub fn validate(
        &self,
        source: &str,
        assets: &Assets,
        context: RenderContext<'_>,
    ) -> Result<(), CommandError> {
        let mut world = self.0.lock()?;
        let settings = context.settings;
        world.set_assets(assets);
        world.set_dependencies(&typstographe_core::packages::including_source(
            &context.dependencies,
            source,
        ));
        world
            .validate_with_model(
                source,
                &settings.metadata,
                &settings.template,
                &settings.model_manifest,
            )
            .map(|_| ())
            .map_err(CommandError::localized_core)
    }
    pub fn render(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        export: Option<PathBuf>,
        settings: Option<RenderContext<'_>>,
        map: Option<SourceMap>,
    ) -> Result<Preview, CommandError> {
        self.render_output(
            source,
            assets,
            revision,
            export,
            settings,
            map,
            Encoding::Dataurl,
            None,
        )
        .and_then(|output| {
            if let Some(sync) = &output.sync {
                *self.1.lock()? = Some(sync.clone());
            }
            Ok(output.preview)
        })
    }
    pub fn frame(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        settings: RenderContext<'_>,
        map: SourceMap,
        encoding: Encoding,
    ) -> Result<Vec<u8>, CommandError> {
        let output = self.render_output(
            source,
            assets,
            revision,
            None,
            Some(settings),
            Some(map),
            encoding,
            None,
        )?;
        if let Some(sync) = &output.sync {
            *self.1.lock()? = Some(sync.clone());
        }
        output.packet(encoding)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn delta_frame(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        settings: RenderContext<'_>,
        map: SourceMap,
        ticket: &delta::Ticket,
        current: &dyn Fn(Stage) -> bool,
    ) -> Result<(Vec<u8>, Arc<delta::Record>), CommandError> {
        let control = Control { ticket, current };
        let mut output = self.render_output(
            source,
            assets,
            revision,
            None,
            Some(settings),
            Some(map),
            Encoding::Rgba,
            Some(&control),
        )?;
        let metadata = delta::Metadata {
            generation: ticket.scope.generation.clone(),
            epoch: ticket.scope.epoch.clone(),
            version: ticket.scope.version,
            id: uuid::Uuid::new_v4().to_string(),
            base: ticket.base.as_ref().map(|base| base.metadata.id.clone()),
            keys: std::mem::take(&mut output.keys),
        };
        output.preview.delta = Some(metadata.clone());
        let record = Arc::new(delta::Record {
            metadata,
            sync: output.sync.take().expect("Mapped delta frame"),
        });
        check(Some(&control), Stage::Packet)?;
        let bytes = output.packet(Encoding::Rgba)?;
        check(Some(&control), Stage::Packet)?;
        Ok((bytes, record))
    }
    // One compilation/validation/synchronisation path for all preview transports.
    #[allow(clippy::too_many_arguments)]
    fn render_output(
        &self,
        source: String,
        assets: Assets,
        revision: u64,
        export: Option<PathBuf>,
        context: Option<RenderContext<'_>>,
        map: Option<SourceMap>,
        encoding: Encoding,
        control: Option<&Control<'_>>,
    ) -> Result<Output, CommandError> {
        check(control, Stage::Start)?;
        let fallback = crate::settings::Settings::default();
        let context = context.unwrap_or_else(|| (&fallback).into());
        let settings = context.settings;
        if export.is_some() && !settings.metadata.ready_for_export() {
            return Err(typstographe_i18n::text("export-required-fields").into());
        }
        let template = settings.template.clone();
        let request = RenderRequest {
            source: source.clone(),
            assets,
            dependencies: typstographe_core::packages::including_source(
                &context.dependencies,
                &source,
            ),

            metadata: settings.metadata.clone(),
            template,
            model_manifest: settings.model_manifest.clone(),
        };
        let start = Instant::now();
        let mut timings = PreviewTimings::default();
        let mut world = self.0.lock()?;
        check(control, Stage::Start)?;
        typstographe_core::packages::validate_embedded(&request.template, &request.dependencies)
            .map_err(CommandError::message)?;
        world.set_assets(&request.assets);
        world.set_dependencies(&request.dependencies);
        timings.prepare_ms = milliseconds(start);
        let compile_start = Instant::now();
        let document = world
            .compile_with_model(
                &request.source,
                &request.metadata,
                &request.template,
                &request.model_manifest,
            )
            .map_err(CommandError::message)?;
        timings.compile_ms = milliseconds(compile_start);
        check(control, Stage::Compiled)?;
        let export_start = Instant::now();
        if let Some(path) = export {
            typstographe_core::exporting::write_document(
                &document,
                &request,
                &path,
                typstographe_core::protocol::ExportFormat::Both,
                false,
            )
            .map_err(CommandError::message)?;
        }
        timings.export_ms = milliseconds(export_start);
        let mut pages = Vec::new();
        let mut buffers = Vec::new();
        let mut dimensions = Vec::new();
        let mut indices = Vec::new();
        let mut keys = Vec::new();
        let options = typst_render::RenderOptions::default();
        for (index, page) in document.pages().iter().enumerate() {
            check(control, Stage::Page)?;
            if let Some(control) = control {
                let key = typstographe_core::rendering::page_key(page, &options);
                let reuse = control
                    .ticket
                    .base
                    .as_ref()
                    .is_some_and(|base| base.metadata.keys.get(index) == Some(&key));
                keys.push(key);
                if reuse {
                    continue;
                }
            }
            indices.push(index);
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
        let references = typstographe_core::content::references::resolved(&document);
        let sync_start = Instant::now();
        check(control, Stage::Synchronization)?;
        let sync = map.map(|map| {
            Arc::new(typstographe_core::navigation::RenderSync::new(
                revision,
                world.clone(),
                document,
                map,
            ))
        });
        timings.sync_ms = milliseconds(sync_start);
        Ok(Output {
            preview: Preview {
                revision,
                references,
                source,
                pages,
                elapsed_ms: milliseconds(start),
                timings,
                page_sizes,
                delta: None,
            },
            buffers,
            dimensions,
            indices,
            keys,
            sync,
        })
    }
}

fn preview_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, CommandError> {
    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // Interactive local preview favours latency over minimum file size.
        // Compression is lossless; export keeps its own encoding policy.
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
    }
    Ok(data)
}
/// A stale request fails; a preview rendered for another revision answers nothing.
fn current_sync<T>(
    session: &Mutex<crate::session::Session>,
    renderer: &Renderer,
    revision: u64,
    query: impl FnOnce(&typstographe_core::navigation::RenderSync) -> Option<T>,
) -> Result<Option<T>, CommandError> {
    let session = session.lock()?;
    if session.revision != revision {
        return Err("stale".into());
    }
    let cache = renderer.1.lock()?;
    Ok(cache
        .as_ref()
        .filter(|sync| sync.revision == revision)
        .and_then(|sync| query(sync)))
}
#[tauri::command]
pub fn sync_forward(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    position: usize,
) -> Result<Option<typstographe_core::navigation::PagePoint>, CommandError> {
    current_sync(&state.session, &state.renderer, revision, |sync| {
        sync.forward(position)
    })
}
#[tauri::command]
pub fn sync_backward(
    state: tauri::State<crate::Bridge>,
    revision: u64,
    location: typstographe_core::navigation::PagePoint,
) -> Result<Option<usize>, CommandError> {
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
) -> Result<Vec<typstographe_core::navigation::SelectionHighlight>, CommandError> {
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
        let prepared=typstographe_core::bibliography::editions::prepare(serde_json::from_value(serde_json::json!({"id":"books","name":"books.bib","format":"biblatex","contents":"@book{key,author={Bourdieu, Pierre},title={Le sens pratique},year={1980}}\r\n"})).unwrap()).unwrap();
        let reference = prepared.source;
        let record_id = prepared.records[0].id().clone();
        meta.references = prepared.records;
        meta.reference_sources.push(reference.clone());
        let settings = crate::settings::Settings {
            metadata: typstographe_core::document_types::Metadata::Article(meta),
            model_manifest: typstographe_core::document_types::DocumentType::Article
                .default_model_manifest()
                .into(),
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
        let node = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"citation","attrs":{"items":[{"record_id":record_id}],"form":"normal"}}]},{"type":"figure","attrs":{"src":image,"alt":"Un parc"}}]})).unwrap();
        let source =
            typstographe_core::content::source_with_metadata(&node, &assets, &settings.metadata)
                .unwrap();
        let root = tempfile::tempdir().unwrap();
        Renderer::default()
            .render(
                source,
                assets,
                1,
                Some(root.path().into()),
                Some((&settings).into()),
                None,
            )
            .unwrap();
        assert_eq!(
            std::fs::read(root.path().join("bibliography/sources/books.bib")).unwrap(),
            reference.contents().as_bytes()
        );
        assert!(root.path().join(image).is_file());
        assert!(root.path().join("capabilities/references.typ").is_file());
        assert!(root.path().join("article.pdf").is_file());
        assert!(
            std::fs::read_to_string(root.path().join("main.typ"))
                .unwrap()
                .contains("path(\"bibliography/references.yaml\")")
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
                .compile_with_model(
                    source,
                    &settings.metadata,
                    &settings.template,
                    &settings.model_manifest,
                )
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
                    Some((&settings).into()),
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
                    (&settings).into(),
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
                    (&small_pages()).into(),
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
                (&small_pages()).into(),
                SourceMap::default(),
                Encoding::Rgba,
            )
            .unwrap();
        let root = tempfile::tempdir().unwrap();
        let article = typstographe_core::document_types::DocumentType::Article;
        let settings = crate::settings::Settings {
            metadata: typstographe_core::document_types::Metadata::blank(article),
            model_manifest: article.default_model_manifest().into(),
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
                    Some((&settings).into()),
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
                    (&small_pages()).into(),
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
                (&small_pages()).into(),
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
                        Some((&settings).into()),
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod delta_tests {
    use super::*;
    fn scope(epoch: &str, version: u64) -> delta::Scope {
        delta::Scope {
            generation: "document".into(),
            epoch: epoch.into(),
            version,
        }
    }
    fn settings() -> crate::settings::Settings {
        crate::settings::Settings {
            template: "#let courrier(body, ..options) = { set page(width: 80pt, height: 80pt, margin: 0pt); body }".into(),
            ..Default::default()
        }
    }
    fn produce(
        renderer: &Renderer,
        ticket: &delta::Ticket,
        source: &str,
        revision: u64,
    ) -> (Vec<u8>, Arc<delta::Record>) {
        renderer
            .delta_frame(
                source.into(),
                Assets::default(),
                revision,
                (&settings()).into(),
                SourceMap::default(),
                ticket,
                &|_| true,
            )
            .unwrap()
    }
    fn payload(packet: &[u8]) -> (serde_json::Value, &[u8]) {
        let size = u32::from_le_bytes(packet[4..8].try_into().unwrap()) as usize;
        (
            serde_json::from_slice(&packet[8..8 + size]).unwrap(),
            &packet[8 + size..],
        )
    }
    #[test]
    fn differential_matches_full_pixels_and_refreshes_sync_without_pixels() {
        let renderer = Renderer::default();
        let mut views = delta::Views::default();
        let epoch = views.bind("first", "document");
        let ticket = views.begin("first", scope(&epoch, 1), None).unwrap();
        let (full, first) = produce(&renderer, &ticket, "Un\n#pagebreak()\nDeux", 1);
        assert_eq!(payload(&full).0["pages"].as_array().unwrap().len(), 2);
        views.commit("first", &ticket, first.clone()).unwrap();
        let ticket = views
            .begin("first", scope(&epoch, 2), Some(&first.metadata.id))
            .unwrap();
        // A comment moves all source offsets without painting a pixel.
        let (same, second) = produce(
            &renderer,
            &ticket,
            "// source offsets\nUn\n#pagebreak()\nDeux",
            2,
        );
        assert!(payload(&same).1.is_empty());
        assert_eq!(payload(&same).0["pages"], serde_json::json!([]));
        assert_eq!(second.sync.revision, 2);
        assert!(!Arc::ptr_eq(&first.sync, &second.sync));
        views.commit("first", &ticket, second.clone()).unwrap();
        let ticket = views
            .begin("first", scope(&epoch, 3), Some(&second.metadata.id))
            .unwrap();
        let (changed, third) = produce(&renderer, &ticket, "Un\n#pagebreak()\nTrois", 3);
        let (header, pixels) = payload(&changed);
        assert_eq!(header["pages"].as_array().unwrap().len(), 1);
        assert_eq!(header["pages"][0]["index"], 1);
        let full = renderer
            .frame(
                "Un\n#pagebreak()\nTrois".into(),
                Assets::default(),
                3,
                (&settings()).into(),
                SourceMap::default(),
                Encoding::Rgba,
            )
            .unwrap();
        let (header, full_pixels) = payload(&full);
        let second_page = header["pages"][1]["offset"].as_u64().unwrap() as usize;
        assert_eq!(pixels, &full_pixels[second_page..]);
        assert_eq!(third.metadata.keys[0], first.metadata.keys[0]);
    }
    #[test]
    fn pixel_reuse_retains_fresh_utf16_source_maps_after_invisible_link_changes() {
        let renderer = Renderer::default();
        let mut views = delta::Views::default();
        let epoch = views.bind("first", "document");
        let mut base = None;
        for (version, href) in [
            (1, "https://example.org/a"),
            (2, "https://example.org/a-much-longer-address"),
        ] {
            let node = serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Été 🦉","marks":[{"type":"link","attrs":{"href":href}}]},{"type":"text","text":" et la suite."}]}]})).unwrap();
            let (source, map) =
                typstographe_core::content::source_mapped(&node, &Assets::default()).unwrap();
            let ticket = views
                .begin("first", scope(&epoch, version), base.as_deref())
                .unwrap();
            let (packet, record) = renderer
                .delta_frame(
                    source.clone(),
                    Assets::default(),
                    version,
                    (&settings()).into(),
                    map.clone(),
                    &ticket,
                    &|_| true,
                )
                .unwrap();
            if version == 2 {
                assert!(payload(&packet).1.is_empty());
            }
            renderer
                .frame(
                    source,
                    Assets::default(),
                    version,
                    (&settings()).into(),
                    map,
                    Encoding::Rgba,
                )
                .unwrap();
            let full = renderer.1.lock().unwrap().clone().unwrap();
            for position in 1..20 {
                let point = record.sync.forward(position);
                assert_eq!(point, full.forward(position));
                if let Some(point) = point {
                    assert_eq!(record.sync.backward(point), full.backward(point));
                }
                assert_eq!(
                    record
                        .sync
                        .selection_highlights_exact(position..position + 1),
                    full.selection_highlights_exact(position..position + 1)
                );
            }
            base = Some(record.metadata.id.clone());
            views.commit("first", &ticket, record).unwrap();
        }
    }

    #[test]
    fn abandoned_candidates_do_not_evict_displayed_base_and_epochs_isolate_windows() {
        let renderer = Renderer::default();
        let mut views = delta::Views::default();
        let epoch = views.bind("first", "document");
        let ticket = views.begin("first", scope(&epoch, 1), None).unwrap();
        let (_, first) = produce(&renderer, &ticket, "Un", 1);
        views.commit("first", &ticket, first.clone()).unwrap();
        let ticket = views
            .begin("first", scope(&epoch, 2), Some(&first.metadata.id))
            .unwrap();
        let (_, discarded) = produce(&renderer, &ticket, "Deux", 2);
        views.commit("first", &ticket, discarded).unwrap();
        let ticket = views
            .begin("first", scope(&epoch, 3), Some(&first.metadata.id))
            .unwrap();
        assert_eq!(ticket.base.as_ref().unwrap().metadata.id, first.metadata.id);
        let (same, _) = produce(&renderer, &ticket, "Un", 3);
        assert!(payload(&same).1.is_empty());
        let other = views.bind("second", "document");
        assert_ne!(other, epoch);
        assert!(
            views
                .begin("second", scope(&epoch, 4), Some(&first.metadata.id))
                .is_err()
        );
        let unknown = views
            .begin("second", scope(&other, 4), Some(&first.metadata.id))
            .unwrap();
        assert!(unknown.base.is_none());
        views.invalidate("first", &scope(&epoch, 5)).unwrap();
        views.invalidate("first", &scope(&epoch, 4)).unwrap(); // Late cancellation cannot regress.
        assert!(!ticket.current());
        assert!(views.begin("first", scope(&epoch, 4), None).is_err());
        let latest = views.begin("first", scope(&epoch, 5), None).unwrap();
        assert!(latest.current());
        views.detach("first");
        assert!(!latest.current());
        assert_ne!(views.bind("first", "document"), epoch);
        assert!(views.begin("first", scope(&epoch, 6), None).is_err());
    }
    #[test]
    fn obsolete_jobs_abort_at_each_expensive_boundary_and_never_publish_sync() {
        let renderer = Renderer::default();
        let mut views = delta::Views::default();
        let epoch = views.bind("first", "document");
        for stop in [
            Stage::Start,
            Stage::Compiled,
            Stage::Page,
            Stage::Synchronization,
            Stage::Packet,
        ] {
            let ticket = views.begin("first", scope(&epoch, 1), None).unwrap();
            let reached = std::cell::Cell::new(false);
            let result = renderer.delta_frame(
                "Un\n#pagebreak()\nDeux".into(),
                Assets::default(),
                1,
                (&settings()).into(),
                SourceMap::default(),
                &ticket,
                &|stage| {
                    if stage == stop {
                        reached.set(true);
                        false
                    } else {
                        true
                    }
                },
            );
            assert!(reached.get(), "{stop:?}");
            assert!(matches!(result, Err(error) if error == CommandError::from("stale")));
            assert!(renderer.1.lock().unwrap().is_none());
        }
        let ticket = views.begin("first", scope(&epoch, 1), None).unwrap();
        let pages = std::cell::Cell::new(0);
        let result = renderer.delta_frame(
            "Un\n#pagebreak()\nDeux".into(),
            Assets::default(),
            1,
            (&settings()).into(),
            SourceMap::default(),
            &ticket,
            &|stage| {
                if stage == Stage::Page {
                    pages.set(pages.get() + 1);
                }
                pages.get() < 2
            },
        );
        assert_eq!(pages.get(), 2);
        assert!(result.is_err()); // Stop between page one and page two, not after packet construction.
        let ticket = views.begin("first", scope(&epoch, 1), None).unwrap();
        let (_, record) = produce(&renderer, &ticket, "Un", 1);
        let newer = views.begin("first", scope(&epoch, 1), None).unwrap();
        assert!(!ticket.current());
        assert!(newer.current());
        assert!(views.commit("first", &ticket, record).is_err());
    }
}
