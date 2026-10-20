//! Moteur Typst embarqué : fichiers virtuels et polices embarquées, sans réseau.
use crate::{
    document::{Metadata, SourceMap},
    navigation::RenderSync,
};
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
    foundations::{Bytes, Datetime, Duration},
    syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot},
    text::{Font, FontBook},
    utils::LazyHash,
};
use typst_layout::PagedDocument;

pub const TEMPLATE: &str = include_str!("../templates/courrier.typ");

#[derive(Clone)]
pub struct TypstWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    files: HashMap<FileId, Source>,
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
    fn set_source(&mut self, name: &str, text: &str) {
        let id = file_id(name);
        self.files
            .entry(id)
            .and_modify(|source| {
                source.replace(text);
            })
            .or_insert_with(|| Source::new(id, text.to_owned()));
    }
    #[cfg(test)]
    pub fn compile(
        &mut self,
        source: &str,
        metadata: &Metadata,
        compact: bool,
    ) -> Result<PagedDocument, String> {
        self.compile_with_template(source, metadata, compact, TEMPLATE)
    }
    pub fn compile_with_template(
        &mut self,
        source: &str,
        metadata: &Metadata,
        compact: bool,
        template: &str,
    ) -> Result<PagedDocument, String> {
        self.set_source("main.typ", &metadata.main_source(compact));
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
        self.source(id)
            .map(|s| Bytes::new(s.text().as_bytes().to_vec()))
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
    pub source_map: SourceMap,
    pub metadata: Metadata,
    pub compact: bool,
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
}
pub struct RenderResponse {
    pub revision: u64,
    pub result: Result<Rendered, String>,
    pub is_export: bool,
}

pub fn render(world: &mut TypstWorld, request: &RenderRequest) -> Result<Rendered, String> {
    let start = Instant::now();
    let document = world.compile_with_template(
        &request.source,
        &request.metadata,
        request.compact,
        &request.template,
    )?;
    if let Some(path) = &request.export_dir {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        if std::fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
        {
            return Err("Le dossier n’est pas vide. Choisissez un nouveau dossier pour préserver les fichiers existants.".into());
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
        write(
            path,
            "main.typ",
            request.metadata.main_source(request.compact).as_bytes(),
        )?;
        write(path, "contenu.typ", request.source.as_bytes())?;
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
    std::fs::write(path.join(name), data).map_err(|e| format!("Impossible d’écrire {name} : {e}"))
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
