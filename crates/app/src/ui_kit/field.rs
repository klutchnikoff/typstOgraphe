use super::{radius, tokens};
use crate::theme::scale;
use eframe::egui::{self, Response, Stroke, StrokeKind};
use std::hash::Hash;

/// Style des champs de saisie (fond, bordure forte, bordure d'accent au focus) pour les
/// `TextEdit` dont le code reste propre à l'écran. À appliquer dans un `ui.scope`.
pub fn field_style(ui: &mut egui::Ui) {
    let t = tokens(ui);
    let visuals = ui.visuals_mut();
    visuals.extreme_bg_color = t.field;
    visuals.selection.stroke = Stroke::new(1.0_f32, t.accent);
    let widgets = &mut visuals.widgets;
    for state in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
    ] {
        state.bg_stroke = Stroke::new(1.0_f32, t.border_strong);
        state.corner_radius = radius(scale::RADIUS_CONTROL);
        state.expansion = 0.0;
    }
}

/// Anneau de focus : trait d’accent doux autour du champ, sans changer sa taille.
fn focus_ring(ui: &egui::Ui, response: &Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(1.5),
            radius(scale::RADIUS_CONTROL + 2),
            Stroke::new(3.0_f32, tokens(ui).accent_soft),
            StrokeKind::Outside,
        );
    }
}

fn show(ui: &mut egui::Ui, edit: impl FnOnce(&mut egui::Ui) -> Response) -> Response {
    ui.scope(|ui| {
        field_style(ui);
        let response = edit(ui);
        focus_ring(ui, &response);
        response
    })
    .inner
}

/// Champ sur une ligne, pleine largeur. `id` doit rester stable entre les frames.
pub fn text_field(ui: &mut egui::Ui, id: impl Hash, text: &mut String) -> Response {
    show(ui, |ui| {
        ui.add(
            egui::TextEdit::singleline(text)
                .id_salt(id)
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(9, 6)),
        )
    })
}

/// Zone de texte de `rows` lignes, pleine largeur.
pub fn text_area(ui: &mut egui::Ui, id: impl Hash, text: &mut String, rows: usize) -> Response {
    show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(text)
                .id_salt(id)
                .desired_rows(rows)
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(9, 6)),
        )
    })
}
