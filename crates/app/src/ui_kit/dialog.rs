use super::{radius, tokens};
use crate::theme::{self, scale};
use eframe::egui::{self, Align, Layout, RichText};

/// Enveloppe modale commune : titre, description facultative, corps, pied d’actions.
/// Largeurs usuelles : 420, 480 ou 540 points.
pub struct Dialog<'a> {
    id: &'a str,
    title: &'a str,
    description: Option<&'a str>,
    width: f32,
}

impl<'a> Dialog<'a> {
    pub fn new(id: &'a str, title: &'a str) -> Self {
        Self {
            id,
            title,
            description: None,
            width: 480.0,
        }
    }
    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    /// Le pied est ajouté de droite à gauche : appeler l'action principale en premier.
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        body: impl FnOnce(&mut egui::Ui) -> R,
        footer_contents: impl FnOnce(&mut egui::Ui),
    ) -> egui::ModalResponse<R> {
        self.show_with(ctx, |ui| {
            let inner = body(ui);
            ui.add_space(14.0);
            footer(ui, footer_contents);
            inner
        })
    }
    /// Variante à un seul contenu, pour un corps et un pied qui partagent le même état :
    /// le contenu appelle lui-même `footer`.
    pub fn show_with<R>(
        self,
        ctx: &egui::Context,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::ModalResponse<R> {
        let dark = ctx.style().visuals.dark_mode;
        let t = crate::theme::Tokens::new(dark);
        let frame = egui::Frame::new()
            .fill(t.raised)
            .stroke(egui::Stroke::new(1.0_f32, t.border))
            .corner_radius(radius(scale::RADIUS_DIALOG))
            .inner_margin(egui::Margin::symmetric(22, 18))
            .shadow(egui::Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 0,
                color: egui::Color32::from_black_alpha(if dark { 115 } else { 40 }),
            });
        egui::Modal::new(egui::Id::new(self.id))
            .backdrop_color(theme::scrim(dark))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(self.width);
                ui.label(
                    RichText::new(self.title)
                        .size(17.0)
                        .family(theme::semibold()),
                );
                if let Some(description) = self.description {
                    ui.label(RichText::new(description).color(tokens(ui).muted));
                }
                ui.add_space(8.0);
                content(ui)
            })
    }
}

/// Pied d'actions aligné à droite (de droite à gauche : l'action principale d'abord).
pub fn footer(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    ui.with_layout(Layout::right_to_left(Align::Center), contents);
}

/// Pied avec une action à part, tout à gauche (destructive, comme « Ne pas enregistrer »),
/// et les autres à droite : l'action principale en premier appel, donc tout à droite.
pub fn footer_split(
    ui: &mut egui::Ui,
    aside: impl FnOnce(&mut egui::Ui),
    actions: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        aside(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), actions);
    });
}
