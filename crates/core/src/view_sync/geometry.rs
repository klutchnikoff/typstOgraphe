use super::*;
use crate::{
    engine::{Engine, LocateParams, RenderPosition, SelectionBoxesParams},
    navigation::PagePoint,
};

impl ViewSync {
    fn current_render(&self, engine: &Engine) -> Option<RenderReference> {
        let render = self.render?;
        (engine.is_open()
            && !self.render_failed
            && self.matches_guard(engine.target_guard())
            && render.guard == engine.target_guard())
        .then_some(render)
    }

    /// Lecture des surbrillances et consommation d'au plus une demande avant.
    /// Un rendu périmé/échoué laisse la demande en attente. Sur un rendu courant,
    /// une localisation sans point la consomme, comme le parcours egui existant.
    pub fn preview_actions(&mut self, engine: &Engine) -> Vec<ViewAction> {
        let render = self.current_render(engine);
        let target = self.selection;
        let boxes = render
            .zip(target)
            .and_then(|(render, target)| {
                engine
                    .selection_boxes(SelectionBoxesParams {
                        render,
                        target: target.into(),
                    })
                    .ok()
            })
            .map_or_else(Vec::new, |result| result.boxes);
        let mut actions = vec![ViewAction::Highlights {
            render,
            target,
            boxes,
        }];
        if self.forward_pending
            && let Some((render, target)) = render.zip(target)
        {
            self.forward_pending = false;
            if let Some(point) = engine
                .locate(LocateParams {
                    render,
                    position: RenderPosition::ToPreview {
                        target: target.into(),
                    },
                })
                .ok()
                .and_then(|result| result.point)
            {
                actions.push(ViewAction::RevealInPreview {
                    render,
                    target,
                    point,
                });
            }
        }
        actions
    }

    /// À consommer lorsque le placement de l'éditeur est prêt. Le choix du focus
    /// et le défilement restent à l'adaptateur, notamment si la recherche est active.
    pub fn take_editor_action(&mut self, engine: &Engine) -> Option<ViewAction> {
        let target = self.selection?;
        if !self.editor_pending
            || !engine.is_open()
            || target.guard != engine.target_guard()
            || engine.resolve_target(target).is_err()
        {
            return None;
        }
        self.editor_pending = false;
        Some(ViewAction::RevealInEditor { target })
    }

    /// Localisation inverse sans sélection ni intention nouvelle. L'appelant
    /// applique `text.select`, puis signale seulement son succès avec origine
    /// Preview. Un clic sur une décoration ou un refus ne supprime pas le suivi.
    pub fn preview_target(&self, engine: &Engine, point: PagePoint) -> Option<SelectionTarget> {
        engine
            .locate(LocateParams {
                render: self.current_render(engine)?,
                position: RenderPosition::ToSource { point },
            })
            .ok()?
            .target
    }
}
