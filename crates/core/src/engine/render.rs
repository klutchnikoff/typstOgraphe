//! Cache dérivé du document validé, alimenté par la compilation publique ou le
//! worker natif. Une lecture ne modifie ni la sélection ni le document.
use super::*;
use crate::{navigation::RenderSync, rendering::RenderRequest, session::Caret};
use std::sync::{Arc, atomic::AtomicU64};
static NEXT_RENDER: AtomicU64 = AtomicU64::new(1);

pub(super) struct RenderState {
    info: RenderInfo,
    sync: Arc<RenderSync>,
}

/// Immutable input for background compilation, without access to the live engine.
pub struct RenderTask {
    guard: TargetGuard,
    request: RenderRequest,
}
pub struct CompiledRender {
    guard: TargetGuard,
    sync: Option<RenderSync>,
    diagnostics: Vec<Diagnostic>,
}
pub struct PageImageTask {
    sync: Arc<RenderSync>,
    page: usize,
}
impl RenderTask {
    pub fn compile(self) -> CompiledRender {
        let mut world = crate::rendering::TypstWorld::default();
        let result = crate::rendering::compile_request(&mut world, &self.request);
        let (sync, diagnostics) = match result {
            Ok((document, _)) => (
                Some(RenderSync::new(
                    self.guard.revision,
                    world,
                    document,
                    self.request.source_map,
                )),
                vec![],
            ),
            Err(cause) => (None, vec![Diagnostic::with_cause("render_compile", cause)]),
        };
        comemo::evict(10);
        CompiledRender {
            guard: self.guard,
            sync,
            diagnostics,
        }
    }
}
impl PageImageTask {
    pub fn execute(self) -> Result<Vec<u8>, CommandError> {
        self.sync
            .page_png(self.page)
            .map_err(|cause| {
                CommandError::with_cause(ErrorCode::Document, "render_compile", cause)
            })?
            .ok_or_else(|| CommandError::invalid("unknown render page"))
    }
}

impl Engine {
    /// Instantané pour le worker ; son appelant conserve la garde avec la demande.
    pub fn preview_request(&self) -> Result<RenderRequest, CommandError> {
        if !self.open {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        Ok(RenderRequest {
            revision: self.session.revision,
            source: self.session.document.source().into(),
            source_map: self.session.document.source_map.clone(),
            metadata: self.session.metadata.clone(),
            template: self.session.template.clone(),
            assets: self.session.document.assets.clone(),
            dependencies: self.session.dependencies.clone(),
            export_dir: None,
        })
    }
    /// Point d’entrée réservé au worker natif. Les données Typst ne font pas
    /// partie des paramètres JSON et ne peuvent pas être injectées par un appel.
    pub fn install_render(
        &self,
        guard: TargetGuard,
        sync: RenderSync,
    ) -> Result<RenderInfo, CommandError> {
        if !self.open {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        self.check_guard(guard)?;
        if sync.revision != guard.revision {
            return Err(CommandError::new(ErrorCode::Document, "stale_revision"));
        }
        let info = RenderInfo {
            reference: RenderReference {
                guard,
                id: RenderId(crate::identity::next(&NEXT_RENDER)),
            },
            pages: (0..sync.page_count())
                .map(|page| sync.page_size(page).expect("rendered page"))
                .collect(),
        };
        *self.render.borrow_mut() = Some(RenderState {
            info: info.clone(),
            sync: Arc::new(sync),
        });
        Ok(info)
    }
    /// Current derived render, without compilation or document changes.
    pub fn render_info(&self) -> Option<RenderInfo> {
        self.render
            .borrow()
            .as_ref()
            .filter(|state| self.open && state.info.reference.guard == self.target_guard())
            .map(|state| state.info.clone())
    }
    fn checked_render(
        &self,
        reference: RenderReference,
    ) -> Result<std::cell::Ref<'_, RenderState>, CommandError> {
        if !self.open {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        self.check_guard(reference.guard)?;
        std::cell::Ref::filter_map(self.render.borrow(), |state| {
            state
                .as_ref()
                .filter(|state| state.info.reference == reference)
        })
        .map_err(|_| CommandError::new(ErrorCode::NotFound, "render_unavailable"))
    }

    /// Lecture binaire pour les adaptateurs : rasterise une seule page (indice à
    /// partir de zéro) du rendu gardé, sans recompilation, édition ou écriture.
    /// Les octets PNG ne font pas partie du protocole JSON du catalogue.
    pub fn render_page_png(
        &self,
        reference: RenderReference,
        page: usize,
    ) -> Result<Vec<u8>, CommandError> {
        self.prepare_page_image(reference, page)?.execute()
    }
    /// Retain an immutable render for rasterization outside the owning UI thread.
    pub fn prepare_page_image(
        &self,
        reference: RenderReference,
        page: usize,
    ) -> Result<PageImageTask, CommandError> {
        let state = self.checked_render(reference)?;
        if page >= state.sync.page_count() {
            return Err(CommandError::invalid("unknown render page"));
        }
        Ok(PageImageTask {
            sync: state.sync.clone(),
            page,
        })
    }
    /// Capture validated published input; no compilation or file IO here.
    pub fn prepare_render(&self, p: GuardParams) -> Result<RenderTask, CommandError> {
        self.check_rule(AvailabilityRule::Read)?;
        self.check_guard(p.guard)?;
        Ok(RenderTask {
            guard: p.guard,
            request: self.preview_request()?,
        })
    }
    /// Install a worker result only if its document and revision are still current.
    /// Prefer an already-current native render so its references remain usable.
    pub fn complete_render(&self, compiled: CompiledRender) -> Result<CompileResult, CommandError> {
        self.check_rule(AvailabilityRule::Read)?;
        self.check_guard(compiled.guard)?;
        if let Some(render) = self.render_info() {
            return Ok(CompileResult {
                guard: compiled.guard,
                render: Some(render),
                diagnostics: vec![],
            });
        }
        Ok(CompileResult {
            guard: compiled.guard,
            render: compiled
                .sync
                .map(|sync| self.install_render(compiled.guard, sync))
                .transpose()?,
            diagnostics: compiled.diagnostics,
        })
    }
    /// Synchronous non-UI convenience; adapters may run `RenderTask` on a worker.
    pub fn compile(&self, p: GuardParams) -> Result<CompileResult, CommandError> {
        self.check_rule(AvailabilityRule::Read)?;
        self.check_guard(p.guard)?;
        if let Some(render) = self.render_info() {
            return Ok(CompileResult {
                guard: p.guard,
                render: Some(render),
                diagnostics: vec![],
            });
        }
        self.complete_render(self.prepare_render(p)?.compile())
    }

    pub fn locate(&self, p: LocateParams) -> Result<super::LocateResult, CommandError> {
        let state = self.checked_render(p.render)?;
        let (target, point) = match p.position {
            RenderPosition::ToPreview { target } => {
                let selection = self.resolve_query_target(target)?;
                (
                    Some(self.selection_target(selection)?),
                    state.sync.forward(selection.primary.index),
                )
            }
            RenderPosition::ToSource { point } => {
                let size = state
                    .sync
                    .page_size(point.page)
                    .ok_or_else(|| CommandError::invalid("unknown render page"))?;
                if point
                    .point
                    .iter()
                    .zip(size)
                    .any(|(value, bound)| !value.is_finite() || *value < 0.0 || *value > bound)
                {
                    return Err(CommandError::invalid("point outside render page"));
                }
                let target = state
                    .sync
                    .backward(point)
                    .map(|index| {
                        let caret = Caret {
                            index,
                            ..Default::default()
                        };
                        self.selection_target(Selection {
                            primary: caret,
                            secondary: caret,
                        })
                    })
                    .transpose()?;
                (target, Some(point))
            }
        };
        Ok(LocateResult {
            render: p.render,
            target,
            point,
        })
    }
    pub fn selection_boxes(
        &self,
        p: SelectionBoxesParams,
    ) -> Result<super::SelectionBoxesResult, CommandError> {
        let state = self.checked_render(p.render)?;
        let range = self.resolve_query_target(p.target)?.range();
        Ok(SelectionBoxesResult {
            render: p.render,
            boxes: state.sync.selection_highlights(range),
        })
    }
}
