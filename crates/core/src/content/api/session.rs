use super::{Call, archive_error, content_error, decode};
use crate::content::{
    archive::Archive,
    commands::{Style, Switch},
    contract::{Guard, Settings, State},
    editing::{ContextId, Direction, EditingSession, Selection},
    project::Project as StoredProject,
    shared::{Publication, SharedDocument},
};
use crate::{
    DocumentLease,
    assets::Assets,
    document_types::{DocumentType, Metadata},
    packages,
    protocol::{CommandError, ErrorCode, ExportFormat},
    rendering::{RenderRequest, TypstWorld},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct New {
    #[serde(rename = "type")]
    kind: DocumentType,
    model: Option<String>,
    metadata: Option<Metadata>,
    #[serde(default)]
    discard_changes: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Open {
    path: PathBuf,
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    discard_changes: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    directory: PathBuf,
    format: ExportFormat,
    #[serde(default)]
    page_images: bool,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Save {
    Current,
    As { path: PathBuf },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    steps: Vec<Call>,
    save: Option<Save>,
}
fn error(reason: &str) -> CommandError {
    CommandError::new(ErrorCode::Unavailable, reason)
}
fn identity() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
struct Project {
    engine: EditingSession,
    context: ContextId,
    storage: StoredProject,
    saved_origin: bool,
    path: Option<PathBuf>,
    lease: Option<DocumentLease>,
    saved: Vec<u8>,
    saved_assets: Vec<String>,
}
impl Project {
    fn dirty(&self) -> bool {
        self.saved != self.engine.update()
            || self.saved_assets.iter().ne(self.engine.assets().keys())
    }
}
pub struct Session {
    project: Option<Project>,
    pub(super) world: TypstWorld,
}
impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
impl Session {
    pub fn new() -> Self {
        Self {
            project: None,
            world: TypstWorld::default(),
        }
    }
    pub fn is_open(&self) -> bool {
        self.project.is_some()
    }
    pub fn guard(&self) -> Option<&Guard> {
        self.project.as_ref().map(|p| p.engine.guard())
    }
    pub fn state(&self) -> Result<&State, CommandError> {
        Ok(self.project()?.engine.state())
    }
    pub fn assets(&self) -> Result<&Assets, CommandError> {
        Ok(self.project()?.engine.assets())
    }
    pub fn file_path(&self) -> Option<&Path> {
        self.project.as_ref().and_then(|p| p.path.as_deref())
    }
    pub fn read_only(&self) -> bool {
        self.project
            .as_ref()
            .is_some_and(|p| p.path.is_some() && p.lease.is_none())
    }
    pub fn dirty(&self) -> bool {
        self.project.as_ref().is_some_and(Project::dirty)
    }
    fn project(&self) -> Result<&Project, CommandError> {
        self.project.as_ref().ok_or_else(|| error("no_document"))
    }
    fn writable(&self) -> Result<(), CommandError> {
        self.project()?;
        if self.read_only() {
            Err(error("read_only"))
        } else {
            Ok(())
        }
    }
    fn replace(
        &mut self,
        storage: StoredProject,
        path: Option<PathBuf>,
        lease: Option<DocumentLease>,
        saved: bool,
    ) -> Result<Value, CommandError> {
        let archive = storage.current().clone();
        let bytes = crate::content::shared::verify_or_initialize(&archive.state(), &archive.yjs)
            .map_err(content_error)?;
        let guard = Guard {
            document: identity(),
            revision: 0,
        };
        let mut engine =
            EditingSession::open(guard.clone(), &bytes, archive.assets).map_err(content_error)?;
        let context = engine.open_context(&guard).map_err(content_error)?;
        let checkpoint = engine.update();
        let saved_assets = if saved {
            engine.assets().keys().cloned().collect()
        } else {
            vec![]
        };
        self.project = Some(Project {
            engine,
            context,
            storage,
            saved_origin: saved,
            path,
            lease,
            saved: if saved { checkpoint } else { vec![] },
            saved_assets,
        });
        self.query("document.inspect", json!({}))
    }
    fn check_discard(&self, discard: bool) -> Result<(), CommandError> {
        if self.dirty() && !discard {
            Err(error("unsaved_changes"))
        } else {
            Ok(())
        }
    }
    pub fn fork(&self) -> Result<Self, CommandError> {
        let storage = self.stored_project()?;
        let mut copy = Self::new();
        copy.replace(storage, None, None, false)?;
        copy.project.as_mut().unwrap().saved_origin = self.project()?.saved_origin;
        Ok(copy)
    }
    pub fn archive(&self) -> Result<Archive, CommandError> {
        let p = self.project()?;
        Ok(Self::archive_for(
            p.engine.state(),
            p.engine.update(),
            p.engine.assets().clone(),
            p.storage
                .dependencies_for(&p.engine.state().settings.template),
        ))
    }
    pub fn stored_project(&self) -> Result<StoredProject, CommandError> {
        self.project()?
            .storage
            .with_current(self.archive()?)
            .map_err(archive_error)
    }
    fn archive_for(
        s: &State,
        yjs: Vec<u8>,
        assets: Assets,
        dependencies: packages::Dependencies,
    ) -> Archive {
        Archive {
            document: s.document.clone(),
            metadata: s.settings.metadata.clone(),
            template: s.settings.template.clone(),
            model_id: s.settings.model_id.clone(),
            assets,
            dependencies,
            yjs,
        }
    }
    pub fn save_as(&mut self, path: &Path) -> Result<Value, CommandError> {
        if std::fs::symlink_metadata(path).is_ok() {
            return Err(error("file_exists"));
        }
        let mut lease = DocumentLease::acquire(path)?;
        let storage = self.stored_project()?;
        let storage = if self.project()?.saved_origin {
            storage.independent_copy()
        } else {
            storage
        };
        storage.save(&mut lease).map_err(archive_error)?;
        let p = self.project.as_mut().unwrap();
        p.storage = storage;
        p.saved_origin = true;
        p.path = Some(path.into());
        p.lease = Some(lease);
        p.saved = p.engine.update();
        p.saved_assets = p.engine.assets().keys().cloned().collect();
        Ok(json!({"saved":path,"guard":p.engine.guard()}))
    }
    pub fn save(&mut self) -> Result<Value, CommandError> {
        self.writable()?;
        let storage = self.stored_project()?;
        let p = self.project.as_mut().unwrap();
        storage
            .save(
                p.lease
                    .as_mut()
                    .ok_or_else(|| error("save_path_required"))?,
            )
            .map_err(archive_error)?;
        p.storage = storage;
        p.saved_origin = true;
        p.saved = p.engine.update();
        p.saved_assets = p.engine.assets().keys().cloned().collect();
        Ok(json!({"saved":p.path,"guard":p.engine.guard()}))
    }
    pub fn query(&mut self, id: &str, params: Value) -> Result<Value, CommandError> {
        super::check_params(id, &params)?;
        match id {
            "document_types.list" => return Ok(super::types()),
            "document_types.describe" => {
                return Ok(super::type_description(decode(params["type"].clone())?));
            }
            "commands.list" => return Ok(super::catalogue()),
            "capabilities.describe" => {
                return super::describe(params["id"].as_str().unwrap_or(""))
                    .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"));
            }
            "spelling.suggestions" => {
                return Ok(
                    json!({"suggestions":crate::spelling::suggestions(decode(params["language"].clone())?,params["word"].as_str().ok_or_else(||content_error(super::invalid("word")))?).map_err(|e|CommandError::with_cause(ErrorCode::Document,"spelling",e))?}),
                );
            }
            _ => {}
        }
        if id == "document.inspect" && !self.is_open() {
            return Ok(json!({"open":false,"guard":null}));
        }
        if id == "render.locate" || id == "render.selection_boxes" {
            return self.render_query(id, &params);
        }
        let p = self.project.as_mut().ok_or_else(|| error("no_document"))?;
        match id {
            "document.inspect" => Ok(
                json!({"project_id":p.storage.project_id(),"guard":p.engine.guard(),"state":p.engine.state(),"document_type":p.engine.state().settings.metadata.document_type(),"path":p.path,"read_only":p.lease.is_none()&&p.path.is_some(),"dirty":p.dirty(),"context":p.engine.context(&p.engine.guard().clone(),p.context).map_err(content_error)?}),
            ),
            "context.get" => Ok(json!(
                p.engine
                    .context(&p.engine.guard().clone(), p.context)
                    .map_err(content_error)?
            )),
            "commands.available" => Ok(json!(
                p.engine
                    .availability(p.engine.guard(), &decode(params["command"].clone())?)
            )),
            "render.compile" => {
                let doc = self.compile()?;
                Ok(
                    json!({"guard":self.guard(),"pages":doc.pages().iter().map(|p|[p.frame.size().x.to_pt(),p.frame.size().y.to_pt()]).collect::<Vec<_>>()}),
                )
            }
            _ => super::queries::run(p.engine.state(), p.engine.assets(), id, &params)
                .map_err(content_error),
        }
    }
    pub fn execute(&mut self, id: &str, params: Value) -> Result<Value, CommandError> {
        super::check_params(id, &params)?;
        match id {
            "document.new" => {
                let p: New = decode(params)?;
                self.check_discard(p.discard_changes)?;
                let metadata = p.metadata.unwrap_or_else(|| Metadata::blank(p.kind));
                if metadata.document_type() != p.kind {
                    return Err(content_error(super::invalid("metadata.type")));
                }
                let model = p
                    .kind
                    .models()
                    .find(|m| p.model.as_ref().is_none_or(|id| m.model.id() == *id))
                    .ok_or_else(|| error("creation_model"))?;
                let state = State::new(
                    serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph"}]}))
                        .unwrap(),
                    Settings {
                        metadata,
                        template: model.source.into(),
                        model_id: model.model.id().into(),
                    },
                );
                let archive = Self::archive_for(
                    &state,
                    SharedDocument::from_state(&state)
                        .map_err(content_error)?
                        .update(),
                    Assets::default(),
                    packages::for_template(model.source),
                );
                self.replace(
                    StoredProject::new(archive).map_err(archive_error)?,
                    None,
                    None,
                    false,
                )
            }
            "document.open" => {
                let p: Open = decode(params)?;
                self.check_discard(p.discard_changes)?;
                let lease = (!p.read_only)
                    .then(|| DocumentLease::acquire(&p.path))
                    .transpose()?;
                let a = StoredProject::read(&p.path).map_err(archive_error)?;
                if let Some(l) = &lease {
                    l.check()?
                }
                self.replace(a, Some(p.path), lease, true)
            }
            "document.close" => {
                self.check_discard(params["discard_changes"].as_bool().unwrap_or(false))?;
                self.project = None;
                Ok(json!({"closed":true}))
            }
            "document.save_as" => self.save_as(Path::new(
                params["path"]
                    .as_str()
                    .ok_or_else(|| content_error(super::invalid("path")))?,
            )),
            "document.save" => self.save(),
            "document.export" => {
                let p: Export = decode(params)?;
                self.export(&p.directory, p.format, p.page_images)
            }
            "document.batch" => {
                let p: Batch = decode(params)?;
                self.batch(&p.steps, p.save)
            }
            "context.set" | "context.style" | "context.insert" | "edit.undo" | "edit.redo" => {
                self.writable()?;
                let p = self.project.as_mut().unwrap();
                let guard = p.engine.guard().clone();
                match id {
                    "context.set" => {
                        let selection: Option<Selection> = decode(params["selection"].clone())?;
                        let styles: Vec<Style> = decode(params["typing_styles"].clone())?;
                        Ok(json!(
                            p.engine
                                .set_context(&guard, p.context, selection, styles)
                                .map_err(content_error)?
                        ))
                    }
                    "context.style" => {
                        let style: Style = decode(params["style"].clone())?;
                        let value: Switch = decode(params["value"].clone())?;
                        p.engine
                            .typing_style(&guard, p.context, style, value)
                            .map_err(content_error)?;
                        Ok(json!(
                            p.engine.context(&guard, p.context).map_err(content_error)?
                        ))
                    }
                    "context.insert" => {
                        let text = decode(params["text"].clone())?;
                        Ok(json!(
                            p.engine
                                .insert(&guard, p.context, text)
                                .map_err(content_error)?
                        ))
                    }
                    _ => Ok(json!(
                        p.engine
                            .history(
                                &guard,
                                p.context,
                                if id == "edit.undo" {
                                    Direction::Undo
                                } else {
                                    Direction::Redo
                                }
                            )
                            .map_err(content_error)?
                    )),
                }
            }
            _ => self.batch(
                &[Call {
                    id: id.into(),
                    params,
                }],
                None,
            ),
        }
    }
    pub fn batch(&mut self, steps: &[Call], save: Option<Save>) -> Result<Value, CommandError> {
        self.writable()?;
        if steps.is_empty() || steps.len() > super::MAX_BATCH_STEPS {
            return Err(content_error(super::invalid("steps")));
        }
        let p = self.project.as_mut().unwrap();
        let guard = p.engine.guard().clone();
        let context = p.engine.context(&guard, p.context).map_err(content_error)?;
        if context.pending {
            return Err(content_error(
                crate::content::contract::Error::EditingContext {
                    reason: "pending_draft".into(),
                },
            ));
        }
        let prepared = p
            .engine
            .prepare_calls(&guard, steps)
            .map_err(content_error)?;
        if prepared.changed() && guard.revision == u64::MAX {
            return Err(content_error(
                crate::content::contract::Error::StateLimitExceeded,
            ));
        }
        let mut new_lease = None;
        let mut stored = None;
        if let Some(ref save) = save {
            let archive = Self::archive_for(
                prepared.state(),
                prepared.checkpoint(),
                prepared.assets(p.engine.assets()).clone(),
                p.storage
                    .dependencies_for(&prepared.state().settings.template),
            );
            let storage = p.storage.with_current(archive).map_err(archive_error)?;
            match save {
                Save::Current => {
                    storage
                        .save(
                            p.lease
                                .as_mut()
                                .ok_or_else(|| error("save_path_required"))?,
                        )
                        .map_err(archive_error)?;
                    stored = Some(storage);
                }
                Save::As { path } => {
                    if std::fs::symlink_metadata(path).is_ok() {
                        return Err(error("file_exists"));
                    }
                    let mut lease = DocumentLease::acquire(path)?;
                    let storage = if p.saved_origin {
                        storage.independent_copy()
                    } else {
                        storage
                    };
                    storage.save(&mut lease).map_err(archive_error)?;
                    stored = Some(storage);
                    new_lease = Some((path.clone(), lease));
                }
            }
        }
        let publication: Publication = p
            .engine
            .publish_prepared(&guard, p.context, prepared)
            .map_err(content_error)?;
        if let Some((path, lease)) = new_lease {
            p.path = Some(path);
            p.lease = Some(lease)
        }
        if save.is_some() {
            p.storage = stored.expect("prepared saved owner");
            p.saved_origin = true;
            p.saved = p.engine.update();
            p.saved_assets = p.engine.assets().keys().cloned().collect();
        }
        Ok(json!({"publication":publication,"saved":save.is_some(),"path":p.path}))
    }
    pub fn compile(&mut self) -> Result<typst_layout::PagedDocument, CommandError> {
        let a = self.archive()?;
        let source = a.validate().map_err(archive_error)?;
        self.world.set_dependencies(&a.dependencies);
        self.world.set_assets(&a.assets);
        self.world
            .compile_with_template(&source, &a.metadata, &a.template)
            .map_err(|e| CommandError::with_cause(ErrorCode::Template, "export_compile", e))
    }
    pub fn page_png(&mut self, page: u32) -> Result<Vec<u8>, CommandError> {
        let document = self.compile()?;
        let index = page
            .checked_sub(1)
            .ok_or_else(|| error("cli_render_page"))? as usize;
        crate::rendering::page_png(&document, index)
            .map_err(|e| CommandError::with_cause(ErrorCode::Template, "export_compile", e))?
            .ok_or_else(|| error("cli_render_page"))
    }
    pub fn export(
        &mut self,
        directory: &Path,
        format: ExportFormat,
        images: bool,
    ) -> Result<Value, CommandError> {
        if !self.state()?.settings.metadata.ready_for_export() {
            return Err(error("export_required_fields"));
        }
        let a = self.archive()?;
        let source = a.validate().map_err(archive_error)?;
        let assets = crate::content::retained_assets(
            &a.document,
            &a.metadata,
            &a.template,
            &a.assets,
            &a.dependencies,
        )
        .map_err(|s| {
            content_error(crate::content::contract::Error::InvalidDocument { reason: s })
        })?;
        let document = self.compile()?;
        let request = RenderRequest {
            source,

            metadata: a.metadata,
            template: a.template,
            assets,
            dependencies: a.dependencies,
        };
        crate::exporting::write_document(&document, &request, directory, format, images)
            .map_err(|e| CommandError::with_cause(ErrorCode::Io, "file_export", e))?;
        Ok(json!({"directory":directory,"guard":self.guard(),"format":format,"page_images":images}))
    }
}
