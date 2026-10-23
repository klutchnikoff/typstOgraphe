//! Un rendu immuable enregistré par l’adaptateur natif, lu par les mêmes requêtes
//! que les futurs transports. Une lecture ne déplace jamais la sélection.
use super::*;
use crate::{navigation::RenderSync, rendering::RenderRequest, session::Caret};
use std::sync::atomic::AtomicU64;
static NEXT_RENDER: AtomicU64 = AtomicU64::new(1);

pub(super) struct RenderState {
    info: RenderInfo,
    sync: RenderSync,
}

impl Engine {
    /// Instantané pour le worker ; son appelant conserve la garde avec la demande.
    pub(crate) fn preview_request(&self) -> Result<RenderRequest, CommandError> {
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
    pub(crate) fn install_render(
        &mut self,
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
        self.render = Some(RenderState {
            info: info.clone(),
            sync,
        });
        Ok(info)
    }
    pub(super) fn render_info(&self) -> Option<&RenderInfo> {
        self.render
            .as_ref()
            .filter(|state| self.open && state.info.reference.guard == self.target_guard())
            .map(|state| &state.info)
    }
    fn checked_render(&self, reference: RenderReference) -> Result<&RenderState, CommandError> {
        if !self.open {
            return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
        }
        self.check_guard(reference.guard)?;
        self.render
            .as_ref()
            .filter(|state| state.info.reference == reference)
            .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "render_unavailable"))
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
