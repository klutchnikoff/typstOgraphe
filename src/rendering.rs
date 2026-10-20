//! Moteur Typst embarqué : fichiers virtuels et polices embarquées, sans réseau.
use crate::{
    document::SourceMap,
    document_types::{Metadata, ModelPresentation},
    navigation::RenderSync,
};
use comemo::Track;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration as StdDuration, Instant},
};
use typst::{
    Library, LibraryExt, World, WorldExt,
    diag::{FileError, FileResult},
    engine::{Route, Sink, Traced},
    foundations::{Bytes, Datetime, Duration, Value},
    syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot},
    text::{Font, FontBook},
    utils::LazyHash,
};
use typst_layout::PagedDocument;

#[derive(Clone)]
pub struct TypstWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    files: HashMap<FileId, Source>,
    assets: HashMap<FileId, Bytes>,
    main: FileId,
}

impl Default for TypstWorld {
    fn default() -> Self {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        let book = FontBook::from_fonts(fonts.iter());
        let main = file_id("main.typ");
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            files: HashMap::new(),
            assets: HashMap::new(),
            main,
        }
    }
}
pub(crate) fn file_id(path: &str) -> FileId {
    FileId::new(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(path).expect("chemin embarqué valide"),
    ))
}

impl TypstWorld {
    pub fn set_assets(&mut self, assets: &crate::assets::Assets) {
        self.assets = assets
            .iter()
            .map(|(path, data)| (file_id(path), Bytes::new(data.clone())))
            .collect();
    }
    fn set_source(&mut self, name: &str, text: &str) {
        let id = file_id(name);
        self.files
            .entry(id)
            .and_modify(|source| {
                if source.text() != text {
                    source.replace(text);
                }
            })
            .or_insert_with(|| Source::new(id, text.to_owned()));
    }
    pub fn model_presentation(&mut self, template: &str) -> Result<ModelPresentation, String> {
        self.set_source("modele.typ", template);
        let source = self
            .source(file_id("modele.typ"))
            .map_err(|e| e.to_string())?;
        let module = typst_eval::eval(
            (self as &dyn World).track(),
            &self.library,
            Traced::default().track(),
            Sink::new().track_mut(),
            Route::default().track(),
            &source,
        )
        .map_err(|errors| {
            errors
                .iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        let mut policy = ModelPresentation::default();
        let Some(binding) = module.scope().get("editor-options") else {
            return Ok(policy);
        };
        let invalid = || crate::tr!("model-invalid-editor-options");
        let Value::Dict(options) = binding.read() else {
            return Err(invalid());
        };
        for (key, value) in options.iter() {
            match (key.as_str(), value) {
                ("bullet-markers", Value::Array(markers)) => {
                    for marker in markers.iter() {
                        let Value::Str(marker) = marker else {
                            return Err(invalid());
                        };
                        if !crate::document::valid_bullet_marker(marker.as_str()) {
                            return Err(invalid());
                        }
                        if !policy.bullet_markers.iter().any(|s| s == marker.as_str()) {
                            policy.bullet_markers.push(marker.to_string());
                        }
                    }
                }
                ("custom-bullet-marker", Value::Bool(allowed)) => {
                    policy.custom_bullet_marker = *allowed
                }
                _ => return Err(invalid()),
            }
        }
        Ok(policy)
    }
    #[cfg(test)]
    pub fn compile_courrier(
        &mut self,
        source: &str,
        metadata: &Metadata,
        model: crate::document_types::courrier::Model,
    ) -> Result<PagedDocument, String> {
        self.compile_with_template(source, metadata, &model.source())
    }
    pub fn compile_with_template(
        &mut self,
        source: &str,
        metadata: &Metadata,
        template: &str,
    ) -> Result<PagedDocument, String> {
        let presentation = self.model_presentation(template)?;
        if let Some(marker) = &metadata.options().bullet_marker
            && !presentation.allows_bullet_marker(marker)
        {
            return Err(crate::tr!("model-bullet-marker-not-allowed"));
        }
        self.set_source("main.typ", &metadata.main_source());
        self.set_source("modele.typ", template);
        self.set_source("contenu.typ", source);
        let result = typst::compile::<PagedDocument>(self);
        result.output.map_err(|errors| {
            errors
                .iter()
                .map(|error| {
                    let location = error
                        .span
                        .id()
                        .and_then(|id| self.files.get(&id).map(|s| (id, s)))
                        .and_then(|(id, s)| {
                            self.range(error.span).map(|range| {
                                format!(
                                    "{}:{}",
                                    id.vpath().get_with_slash(),
                                    s.lines().byte_to_line(range.start).unwrap_or(0) + 1
                                )
                            })
                        })
                        .unwrap_or_else(|| "Typst".into());
                    let hints = error
                        .hints
                        .iter()
                        .map(|hint| format!("\n  {}", hint.v))
                        .collect::<String>();
                    format!("{location} : {}{hints}", error.message)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
}

impl World for TypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().get_with_slash().into()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.assets.get(&id).cloned().map(Ok).unwrap_or_else(|| {
            self.source(id)
                .map(|s| Bytes::new(s.text().as_bytes().to_vec()))
        })
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

impl typst_ide::IdeWorld for TypstWorld {
    fn upcast(&self) -> &dyn World {
        self
    }
}

#[derive(Clone)]
pub struct RenderRequest {
    pub revision: u64,
    pub source: String,
    pub assets: crate::assets::Assets,
    pub source_map: SourceMap,
    pub metadata: Metadata,
    pub template: String,
    pub export_dir: Option<PathBuf>,
}
pub struct PageImage {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}
pub struct Rendered {
    pub pages: Vec<PageImage>,
    pub sync: RenderSync,
    pub elapsed: StdDuration,
    pub export_dir: Option<PathBuf>,
    pub presentation: ModelPresentation,
}
pub struct RenderResponse {
    pub revision: u64,
    pub result: Result<Rendered, String>,
    pub is_export: bool,
}

/// Ressources à écrire, sans modifier le document ni ses instantanés d’annulation.
/// Les chemins déclarés dans le modèle restent disponibles, même dans une branche
/// inactive. Pour les chemins calculés, Typst vérifie que retirer un lot ne change
/// ni la compilation ni son résultat (pages et métadonnées).
pub fn retained_assets(
    source: &str,
    metadata: &Metadata,
    template: &str,
    assets: &crate::assets::Assets,
) -> Result<crate::assets::Assets, String> {
    if assets.is_empty() {
        return Ok(assets.clone());
    }
    let document = crate::document::Document::from_source(source)?;
    let mut required: std::collections::BTreeSet<_> = document
        .view
        .images
        .values()
        .map(|image| image.path.clone())
        .collect();
    fn declared_paths(
        node: &typst_syntax::SyntaxNode,
        required: &mut std::collections::BTreeSet<String>,
    ) {
        use typst_syntax::ast;
        if let Some(value) = node.cast::<ast::Str>() {
            let value = value.get();
            let path = value.trim_start_matches('/').trim_start_matches("./");
            if crate::assets::valid_path(path) {
                required.insert(path.into());
            }
        }
        for child in node.children() {
            declared_paths(child, required);
        }
    }
    declared_paths(
        typst_syntax::Source::detached(template).root(),
        &mut required,
    );
    let candidates: Vec<_> = assets
        .keys()
        .filter(|path| !required.contains(*path))
        .cloned()
        .collect();
    if candidates.is_empty() {
        return Ok(assets.clone());
    }
    let mut world = TypstWorld::default();
    world.set_assets(assets);
    let Ok(baseline) = world.compile_with_template(source, metadata, template) else {
        // L’enregistrement doit rester possible pendant une erreur de modèle.
        // Sans résultat de référence, conserver les ressources est le seul choix sûr.
        return Ok(assets.clone());
    };
    let expected = typst::utils::hash128(&baseline);
    let mut retained = assets.clone();
    // Essayer d’abord tout le lot : une seule vérification suffit pour les anciennes
    // images d’un courrier courant. Subdiviser seulement si le modèle en utilise.
    fn prune(
        paths: &[String],
        assets: &mut crate::assets::Assets,
        unchanged: &mut impl FnMut(&crate::assets::Assets) -> bool,
    ) {
        if paths.is_empty() {
            return;
        }
        let removed: crate::assets::Assets = paths
            .iter()
            .filter_map(|path| assets.remove(path).map(|data| (path.clone(), data)))
            .collect();
        if unchanged(assets) {
            return;
        }
        assets.extend(removed);
        if paths.len() > 1 {
            let middle = paths.len() / 2;
            prune(&paths[..middle], assets, unchanged);
            prune(&paths[middle..], assets, unchanged);
        }
    }
    prune(&candidates, &mut retained, &mut |trial| {
        world.set_assets(trial);
        world
            .compile_with_template(source, metadata, template)
            .is_ok_and(|output| typst::utils::hash128(&output) == expected)
    });
    Ok(retained)
}

pub fn render(world: &mut TypstWorld, request: &RenderRequest) -> Result<Rendered, String> {
    let start = Instant::now();
    world.set_assets(&request.assets);
    let document =
        world.compile_with_template(&request.source, &request.metadata, &request.template)?;
    let presentation = world.model_presentation(&request.template)?;
    if let Some(path) = &request.export_dir {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        if std::fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
        {
            return Err(crate::tr!("export-nonempty"));
        }
        let pdf =
            typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
                errors
                    .iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
        write(path, "courrier.pdf", &pdf)?;
        write(path, "main.typ", request.metadata.main_source().as_bytes())?;
        write(path, "contenu.typ", request.source.as_bytes())?;
        let assets = retained_assets(
            &request.source,
            &request.metadata,
            &request.template,
            &request.assets,
        )?;
        if !assets.is_empty() {
            std::fs::create_dir(path.join("images")).map_err(|e| e.to_string())?;
        }
        for (name, data) in &assets {
            if !crate::assets::valid_path(name) {
                return Err(crate::tr!("storage-unknown-entry"));
            }
            write(path, name, data)?;
        }
        write(path, "modele.typ", request.template.as_bytes())?;
        write(
            path,
            "metadonnees.json",
            serde_json::to_string_pretty(&request.metadata)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )?;
    }
    let mut pages = vec![];
    for (index, page) in document.pages().iter().enumerate() {
        let pixmap = typst_render::render(page, &typst_render::RenderOptions::default());
        if let Some(path) = &request.export_dir {
            let png = pixmap.encode_png().map_err(|e| e.to_string())?;
            write(path, &format!("page-{}.png", index + 1), &png)?;
        }
        pages.push(PageImage {
            size: [pixmap.width() as usize, pixmap.height() as usize],
            rgba: pixmap.data().to_vec(),
        });
    }
    comemo::evict(10);
    Ok(Rendered {
        presentation,
        pages,
        sync: RenderSync::new(
            request.revision,
            world.clone(),
            document,
            request.source_map.clone(),
        ),
        elapsed: start.elapsed(),
        export_dir: request.export_dir.clone(),
    })
}
fn write(path: &Path, name: &str, data: &[u8]) -> Result<(), String> {
    std::fs::write(path.join(name), data)
        .map_err(|e| crate::tr!("export-write-failed", name = name, error = e.to_string()))
}

/// Un unique worker conserve les caches Typst. Les demandes d'aperçu devenues
/// obsolètes sont coalescées. Une demande d'export n'est jamais abandonnée.
pub struct RenderWorker {
    pub sender: mpsc::Sender<RenderRequest>,
    pub receiver: mpsc::Receiver<RenderResponse>,
}
impl RenderWorker {
    pub fn spawn(context: eframe::egui::Context) -> Self {
        let (sender, requests) = mpsc::channel::<RenderRequest>();
        let (results, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("typst-render".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut world = TypstWorld::default();
                while let Ok(mut request) = requests.recv() {
                    let mut exports = Vec::new();
                    while let Ok(next) = requests.try_recv() {
                        if request.export_dir.is_some() {
                            exports.push(request);
                        }
                        request = next;
                    }
                    exports.push(request);
                    for request in exports {
                        let response = RenderResponse {
                            revision: request.revision,
                            is_export: request.export_dir.is_some(),
                            result: render(&mut world, &request),
                        };
                        if results.send(response).is_err() {
                            return;
                        }
                        context.request_repaint();
                    }
                }
            })
            .expect("création du thread Typst");
        Self { sender, receiver }
    }
}

#[cfg(test)]
mod asset_tests {
    use super::*;
    use crate::{assets::Assets, document_types::DocumentType};

    #[test]
    fn pruning_preserves_model_dependencies_computed_paths_and_shared_images() {
        let metadata = Metadata::demo(DocumentType::Courrier);
        let template = DocumentType::Courrier.default_model().replace(
            "  let contenu = {",
            "  image(\"images/\" + \"shared\" + \".svg\", width: 12pt)\n  let contenu = {",
        ) + "\n// Ancienne image : \"images/obsolete.svg\"\n#let future-logo = \"images/future.svg\"\n";
        // Des octets identiques ne signifient pas que les chemins sont interchangeables.
        let bytes: std::sync::Arc<[u8]> = include_bytes!("../tests/fixtures/parc.svg")
            .as_slice()
            .into();
        let assets: Assets = ["shared", "future", "obsolete", "deleted"]
            .into_iter()
            .map(|name| (format!("images/{name}.svg"), bytes.clone()))
            .collect();
        let source = "#figure(image(\"images/shared.svg\"), caption: [Un parc.])";
        let expected: Vec<String> = vec!["images/future.svg".into(), "images/shared.svg".into()];
        for body in [source, "Texte sans image.", source, ""] {
            let retained = retained_assets(body, &metadata, &template, &assets).unwrap();
            assert_eq!(retained.keys().cloned().collect::<Vec<_>>(), expected);
            let mut world = TypstWorld::default();
            world.set_assets(&retained);
            world
                .compile_with_template(body, &metadata, &template)
                .unwrap();
        }
        assert_eq!(assets.len(), 4);
    }

    #[test]
    fn a_broken_model_does_not_lose_resources_during_save() {
        let mut assets = Assets::new();
        assets.insert(
            "images/logo.svg".into(),
            include_bytes!("../tests/fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        assert_eq!(
            retained_assets(
                "",
                &Metadata::demo(DocumentType::Courrier),
                "#let courrier(..args) = panic(\"modèle à réparer\")",
                &assets
            )
            .unwrap(),
            assets
        );
    }
}
