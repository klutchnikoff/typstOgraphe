//! Moteur Typst embarqué : fichiers virtuels et polices embarquées, sans réseau.
use crate::errors::{CoreError, TemplateError};
use crate::{
    document::SourceMap,
    document_types::{Metadata, ModelPresentation},
    navigation::RenderSync,
};
use comemo::Track;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration as StdDuration, Instant},
};
use typst::{
    Library, LibraryExt, World, WorldExt,
    diag::{FileError, FileResult},
    engine::{Route, Sink, Traced},
    foundations::{Bytes, Datetime, Duration, Module, Value},
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
    dependencies: HashMap<FileId, Bytes>,
    main: FileId,
}

impl Default for TypstWorld {
    fn default() -> Self {
        let fonts = crate::fonts::typst_fonts();
        let book = FontBook::from_fonts(fonts.iter());
        let main = file_id("main.typ");
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            files: HashMap::new(),
            assets: HashMap::new(),
            dependencies: crate::packages::bundled()
                .iter()
                .map(|(path, data)| (file_id(path), Bytes::new(data.clone())))
                .collect(),
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
    /// Remplacement intégral : le rendu d’un fichier ne complète jamais ses ressources
    /// avec celles de l’application ou du cache Typst de la machine.
    pub fn set_dependencies(&mut self, files: &crate::packages::Dependencies) {
        self.dependencies = files
            .iter()
            .map(|(path, data)| (file_id(path), Bytes::new(data.clone())))
            .collect();
    }

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
    fn model_module(&mut self, template: &str) -> Result<Module, CoreError> {
        self.set_source("modele.typ", template);
        let source = self
            .source(file_id("modele.typ"))
            .map_err(CoreError::external)?;
        typst_eval::eval(
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
        })
        .map_err(|error| CoreError::from(TemplateError::Compilation { error }))
    }
    pub fn validate_model(
        &mut self,
        source: &str,
        metadata: &Metadata,
        template: &str,
    ) -> Result<ModelPresentation, CoreError> {
        let module = self.model_module(template)?;
        let function = metadata.document_type().composition_function();
        if !module
            .scope()
            .get(function)
            .is_some_and(|binding| matches!(binding.read(), Value::Func(_)))
        {
            return Err(CoreError::from(TemplateError::ContractMissing {
                function: (function).to_string(),
            }));
        }
        self.compile_with_template(source, metadata, template)?;
        self.model_presentation(template)
    }
    pub fn model_presentation(&mut self, template: &str) -> Result<ModelPresentation, CoreError> {
        let module = self.model_module(template)?;
        let mut policy = ModelPresentation::default();
        let Some(binding) = module.scope().get("editor-options") else {
            return Ok(policy);
        };
        let invalid = || CoreError::from(TemplateError::InvalidEditorOptions);
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
                ("outline", Value::Bool(allowed)) => policy.outline = *allowed,
                ("citation-styles", Value::Array(styles)) => {
                    for style in styles.iter() {
                        let Value::Str(style) = style else {
                            return Err(invalid());
                        };
                        if !["chicago-author-date", "apa", "ieee"].contains(&style.as_str()) {
                            return Err(invalid());
                        }
                        if !policy.citation_styles.iter().any(|s| s == style.as_str()) {
                            policy.citation_styles.push(style.to_string());
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn compile_courrier(
        &mut self,
        source: &str,
        metadata: &Metadata,
        model: crate::document_types::models::Model,
    ) -> Result<PagedDocument, CoreError> {
        self.compile_with_template(source, metadata, &model.source())
    }
    pub fn compile_with_template(
        &mut self,
        source: &str,
        metadata: &Metadata,
        template: &str,
    ) -> Result<PagedDocument, CoreError> {
        let presentation = self.model_presentation(template)?;
        if let Some(marker) = &metadata.options().bullet_marker
            && !presentation.allows_bullet_marker(marker)
        {
            return Err(CoreError::from(TemplateError::BulletMarkerNotAllowed));
        }
        metadata.validate().map_err(CoreError::external)?;
        self.files
            .retain(|id, _| !id.vpath().get_with_slash().starts_with("/bibliography/"));
        for (path, data) in crate::bibliography::resources(metadata) {
            self.set_source(
                &path,
                std::str::from_utf8(&data).map_err(CoreError::external)?,
            );
        }
        self.set_source("main.typ", &metadata.main_source());
        self.set_source("modele.typ", template);
        self.set_source("contenu.typ", source);
        let result = typst::compile::<PagedDocument>(self);
        result
            .output
            .map_err(|errors| {
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
            .map_err(|error| CoreError::from(TemplateError::Compilation { error }))
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
        if let Some(source) = self.files.get(&id) {
            return Ok(source.clone());
        }
        let bytes = self
            .dependencies
            .get(&id)
            .ok_or_else(|| FileError::NotFound(id.vpath().get_with_slash().into()))?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_owned()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.assets
            .get(&id)
            .or_else(|| self.dependencies.get(&id))
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| {
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
    pub dependencies: crate::packages::Dependencies,
    pub source_map: SourceMap,
    pub metadata: Metadata,
    pub template: String,
    pub export_dir: Option<PathBuf>,
}
pub use crate::assets::RgbaImage as PageImage;

/// Même rasterisation pour une page demandée par un adaptateur et les PNG d’export.
pub(crate) fn page_png(
    document: &PagedDocument,
    index: usize,
) -> Result<Option<Vec<u8>>, CoreError> {
    document
        .pages()
        .get(index)
        .map(|page| {
            typst_render::render(page, &typst_render::RenderOptions::default())
                .encode_png()
                .map_err(CoreError::external)
        })
        .transpose()
}

pub struct Rendered {
    pub pages: Vec<PageImage>,
    pub sync: RenderSync,
    pub elapsed: StdDuration,
    pub export_dir: Option<PathBuf>,
    pub presentation: ModelPresentation,
}
pub struct PreviewRequest {
    pub guard: crate::session::TargetGuard,
    pub request: RenderRequest,
}
pub struct RenderResponse {
    pub guard: crate::session::TargetGuard,
    pub result: Result<Rendered, CoreError>,
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
    dependencies: &crate::packages::Dependencies,
) -> Result<crate::assets::Assets, CoreError> {
    if assets.is_empty() {
        return Ok(assets.clone());
    }
    let document = crate::document::Document::from_source(source)?;
    let required: std::collections::BTreeSet<_> = document
        .view
        .images
        .values()
        .map(|image| image.path.clone())
        .collect();
    retained_assets_for_paths(source, metadata, template, assets, dependencies, required)
}

/// Même rétention pour un adaptateur qui possède déjà les chemins de ses objets.
pub fn retained_assets_for_paths(
    source: &str,
    metadata: &Metadata,
    template: &str,
    assets: &crate::assets::Assets,
    dependencies: &crate::packages::Dependencies,
    mut required: std::collections::BTreeSet<String>,
) -> Result<crate::assets::Assets, CoreError> {
    if assets.is_empty() {
        return Ok(assets.clone());
    }
    required.extend(metadata.signature_path().map(str::to_owned));
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
    world.set_dependencies(dependencies);
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

/// Même compilation Typst pour le worker natif et la requête sans UI. La
/// rasterisation et les écritures éventuelles appartiennent à leurs adaptateurs.
pub(crate) fn compile_request(
    world: &mut TypstWorld,
    request: &RenderRequest,
) -> Result<(PagedDocument, ModelPresentation), CoreError> {
    crate::packages::validate(&request.template, &request.dependencies)?;
    world.set_assets(&request.assets);
    world.set_dependencies(&request.dependencies);
    let document =
        world.compile_with_template(&request.source, &request.metadata, &request.template)?;
    let presentation = world.model_presentation(&request.template)?;
    Ok((document, presentation))
}

pub fn render(world: &mut TypstWorld, request: &RenderRequest) -> Result<Rendered, CoreError> {
    let start = Instant::now();
    let (document, presentation) = compile_request(world, request)?;
    if let Some(path) = &request.export_dir {
        crate::exporting::write_document(
            &document,
            request,
            path,
            crate::engine::ExportFormat::Both,
            true,
        )?;
    }
    let mut pages = vec![];
    for page in document.pages() {
        let pixmap = typst_render::render(page, &typst_render::RenderOptions::default());
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
/// Un unique worker conserve les caches Typst. Les demandes d'aperçu devenues
/// obsolètes sont coalescées. Une demande d'export n'est jamais abandonnée.
pub struct RenderWorker {
    pub sender: mpsc::Sender<PreviewRequest>,
    pub receiver: mpsc::Receiver<RenderResponse>,
}
impl RenderWorker {
    pub fn spawn(wake: std::sync::Arc<dyn Fn() + Send + Sync>) -> Self {
        let (sender, requests) = mpsc::channel::<PreviewRequest>();
        let (results, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("typst-render".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut world = TypstWorld::default();
                while let Ok(mut request) = requests.recv() {
                    let mut exports = Vec::new();
                    while let Ok(next) = requests.try_recv() {
                        if request.request.export_dir.is_some() {
                            exports.push(request);
                        }
                        request = next;
                    }
                    exports.push(request);
                    for request in exports {
                        let response = RenderResponse {
                            guard: request.guard,
                            is_export: request.request.export_dir.is_some(),
                            result: render(&mut world, &request.request),
                        };
                        if results.send(response).is_err() {
                            return;
                        }
                        wake();
                    }
                }
            })
            .expect("création du thread Typst");
        Self { sender, receiver }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod asset_tests {
    use super::*;
    use crate::{assets::Assets, document_types::DocumentType};

    #[test]
    fn worker_wakes_a_non_graphical_caller_for_success_and_failure_with_the_request_guard() {
        let (notify, notified) = mpsc::channel();
        let worker = RenderWorker::spawn(std::sync::Arc::new(move || {
            notify.send(()).unwrap();
        }));
        let stored = crate::storage::StoredDocument::blank(DocumentType::Courrier);
        let document = crate::document::Document::from_source("Noël écrit.").unwrap();
        let guard =
            crate::session::Session::new(stored.clone(), document.clone(), None, 7).target_guard();
        for valid in [true, false] {
            worker
                .sender
                .send(PreviewRequest {
                    guard,
                    request: RenderRequest {
                        revision: guard.revision,
                        source: document.source().into(),
                        source_map: document.source_map.clone(),
                        metadata: stored.metadata.clone(),
                        template: if valid {
                            stored.template.clone()
                        } else {
                            "#let courrier(".into()
                        },
                        assets: stored.assets.clone(),
                        dependencies: stored.dependencies.clone(),
                        export_dir: None,
                    },
                })
                .unwrap();
            // Le réveil suit la publication de la réponse, que le calcul réussisse ou non.
            notified.recv_timeout(StdDuration::from_secs(30)).unwrap();
            let response = worker.receiver.try_recv().unwrap();
            assert_eq!(response.guard, guard);
            assert_eq!(response.result.is_ok(), valid);
            assert!(!response.is_export);
        }
    }

    #[test]
    fn pruning_preserves_model_dependencies_computed_paths_and_shared_images() {
        let metadata = Metadata::demo(DocumentType::Courrier);
        let template = DocumentType::Courrier.default_model().replace(
            "  body\n",
            "  image(\"images/\" + \"shared\" + \".svg\", width: 12pt)\n  body\n",
        ) + "\n// Ancienne image : \"images/obsolete.svg\"\n#let future-logo = \"images/future.svg\"\n";
        assert!(
            template.contains("image(\"images/\" + \"shared\""),
            "injection de l’image dans le modèle"
        );
        // Des octets identiques ne signifient pas que les chemins sont interchangeables.
        let bytes: std::sync::Arc<[u8]> = include_bytes!("../../../fixtures/parc.svg")
            .as_slice()
            .into();
        let assets: Assets = ["shared", "future", "obsolete", "deleted"]
            .into_iter()
            .map(|name| (format!("images/{name}.svg"), bytes.clone()))
            .collect();
        let source = "#figure(image(\"images/shared.svg\"), caption: [Un parc.])";
        let expected: Vec<String> = vec!["images/future.svg".into(), "images/shared.svg".into()];
        for body in [source, "Texte sans image.", source, ""] {
            let retained = retained_assets(
                body,
                &metadata,
                &template,
                &assets,
                &crate::packages::for_template(&template),
            )
            .unwrap();
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
            include_bytes!("../../../fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        assert_eq!(
            retained_assets(
                "",
                &Metadata::demo(DocumentType::Courrier),
                "#let courrier(..args) = panic(\"modèle à réparer\")",
                &assets,
                &Default::default(),
            )
            .unwrap(),
            assets
        );
    }
}
