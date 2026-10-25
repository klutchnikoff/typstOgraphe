//! Barre latérale droite (D40, L.2 bis) : barre étroite d’icônes carrées et tiroir qui
//! recouvre l’aperçu sans déplacer l’éditeur. Le composant est inactif sans panneau :
//! il ne réserve aucune largeur, ne dessine rien et ne garde aucun état ouvert.
use super::{icon_button, radius, tokens};
use crate::{icons::Icon, theme::scale};
use eframe::egui::{self, Rect, Sense, Stroke};

/// Largeur de la barre d’icônes.
pub const RAIL_WIDTH: f32 = 44.0;
/// Largeur minimale du tiroir, tant que la colonne la permet.
pub const DRAWER_MIN: f32 = 240.0;
const DEFAULT_FRACTION: f32 = 0.55;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidePanel {
    pub id: &'static str,
    pub icon: Icon,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideRail {
    pub open: Option<&'static str>,
    /// Part de la colonne d’aperçu occupée par le tiroir (bornée à 2/3).
    pub fraction: f32,
}
impl Default for SideRail {
    fn default() -> Self {
        Self {
            open: None,
            fraction: DEFAULT_FRACTION,
        }
    }
}

/// Zones issues d’une colonne d’aperçu : le contenu perd la largeur de la barre
/// uniquement si au moins un panneau est déclaré.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailLayout {
    pub content: Rect,
    pub rail: Option<Rect>,
}

pub fn reserve(bounds: Rect, panels: &[SidePanel]) -> RailLayout {
    if panels.is_empty() || bounds.width() <= RAIL_WIDTH {
        return RailLayout {
            content: bounds,
            rail: None,
        };
    }
    let (content, rail) = (
        Rect::from_min_max(
            bounds.min,
            egui::pos2(bounds.right() - RAIL_WIDTH, bounds.bottom()),
        ),
        Rect::from_min_max(
            egui::pos2(bounds.right() - RAIL_WIDTH, bounds.top()),
            bounds.max,
        ),
    );
    RailLayout {
        content,
        rail: Some(rail),
    }
}

impl SideRail {
    pub fn close(&mut self) {
        self.open = None;
    }
    pub fn toggle(&mut self, id: &'static str) {
        self.open = if self.open == Some(id) {
            None
        } else {
            Some(id)
        };
    }
    /// Plafond : 2/3 de la colonne d’aperçu.
    pub fn max_width(bounds: Rect) -> f32 {
        (bounds.width() * 2.0 / 3.0).floor()
    }
    /// Largeur effective : la préférence, bornée entre le minimum et le plafond ;
    /// si la colonne est trop étroite pour le minimum, le plafond l’emporte.
    pub fn drawer_width(&self, bounds: Rect) -> f32 {
        let max = Self::max_width(bounds);
        (bounds.width() * self.fraction).clamp(DRAWER_MIN.min(max), max)
    }

    /// Dessine la barre et, si un panneau est ouvert, le tiroir. `bounds` est la colonne
    /// d’aperçu entière. Sans panneau, ne fait rien. `body` dessine le contenu du
    /// panneau ouvert à partir de son identifiant.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        bounds: Rect,
        panels: &[SidePanel],
        enabled: bool,
        body: impl FnOnce(&'static str, &mut egui::Ui),
    ) {
        let layout = reserve(bounds, panels);
        let Some(rail) = layout.rail else {
            self.close();
            return;
        };
        if self
            .open
            .is_some_and(|id| panels.iter().all(|p| p.id != id))
        {
            self.close();
        }
        let t = tokens(ui);
        ui.painter().rect_filled(rail, 0.0, t.panel);
        ui.painter()
            .vline(rail.left(), rail.y_range(), Stroke::new(1.0_f32, t.border));
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("side-rail")
                .max_rect(rail.shrink2(egui::vec2(7.0, 8.0))),
            |ui| {
                if !enabled {
                    ui.disable();
                }
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.vertical(|ui| {
                    for panel in panels {
                        if icon_button(ui, panel.icon, self.open == Some(panel.id), &panel.label)
                            .clicked()
                        {
                            self.toggle(panel.id);
                        }
                    }
                });
            },
        );
        let Some(id) = self.open else { return };
        let panel = panels
            .iter()
            .find(|p| p.id == id)
            .expect("open panel is declared");
        self.drawer(ui, bounds, rail, panel, enabled, body);
    }

    fn drawer(
        &mut self,
        ui: &mut egui::Ui,
        bounds: Rect,
        rail: Rect,
        panel: &SidePanel,
        enabled: bool,
        body: impl FnOnce(&'static str, &mut egui::Ui),
    ) {
        let t = tokens(ui);
        let width = self.drawer_width(bounds);
        let rect = Rect::from_min_max(
            egui::pos2(rail.left() - width, bounds.top()),
            egui::pos2(rail.left(), bounds.bottom()),
        );
        // Zone de saisie du bord gauche : redimensionner sans dépasser 2/3.
        let edge = Rect::from_min_max(
            egui::pos2(rect.left() - 4.0, rect.top()),
            egui::pos2(rect.left() + 4.0, rect.bottom()),
        );
        let drag = ui.interact(edge, egui::Id::new("side-rail-edge"), Sense::drag());
        if drag.dragged()
            && let Some(pointer) = drag.interact_pointer_pos()
        {
            let wanted = (rail.left() - pointer.x).clamp(0.0, Self::max_width(bounds));
            self.fraction = (wanted / bounds.width()).clamp(0.05, 2.0 / 3.0);
        }
        drag.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        let mut close = false;
        egui::Area::new(egui::Id::new("side-rail-drawer"))
            .order(egui::Order::Middle)
            .fixed_pos(rect.min)
            .sense(Sense::click_and_drag())
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(t.panel)
                    .stroke(Stroke::new(1.0_f32, t.border))
                    .corner_radius(radius(0))
                    .shadow(egui::Shadow {
                        offset: [-6, 0],
                        blur: scale::RADIUS_DIALOG * 2,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(if ui.visuals().dark_mode {
                            110
                        } else {
                            36
                        }),
                    })
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        if !enabled {
                            ui.disable();
                        }
                        ui.set_min_size(rect.size() - egui::vec2(25.0, 17.0));
                        ui.set_max_size(rect.size() - egui::vec2(25.0, 17.0));
                        ui.horizontal(|ui| {
                            ui.set_min_height(super::CONTROL_HEIGHT);
                            ui.label(
                                egui::RichText::new(&panel.label).family(crate::theme::semibold()),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    close =
                                        icon_button(ui, Icon::Close, false, &panel.label).clicked();
                                },
                            );
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .id_salt(("side-rail-scroll", panel.id))
                            .auto_shrink([false, false])
                            .show(ui, |ui| body(panel.id, ui));
                    });
            });
        if close {
            self.close();
        }
    }
}
