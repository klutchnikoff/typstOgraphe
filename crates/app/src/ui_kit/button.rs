use super::{CONTROL_HEIGHT, radius, tokens};
use crate::{
    icons::Icon,
    theme::{self, scale},
};
use eframe::egui::{self, Color32, Response, Stroke, WidgetInfo, WidgetText, WidgetType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Action principale d’une surface : une seule par dialogue ou bandeau.
    Primary,
    Secondary,
    Ghost,
    /// Action destructive : contour et texte de danger.
    Danger,
}

fn style(ui: &mut egui::Ui, kind: ButtonKind) {
    let t = tokens(ui);
    let (rest, hover, press, stroke, text, hover_text) = match kind {
        ButtonKind::Primary => (
            t.accent,
            theme::mix(t.accent, Color32::WHITE, 0.1),
            theme::mix(t.accent, Color32::BLACK, 0.12),
            Stroke::new(1.0_f32, t.accent),
            t.on_accent,
            t.on_accent,
        ),
        ButtonKind::Secondary => (
            Color32::TRANSPARENT,
            t.hover,
            theme::mix(t.hover, t.text, 0.08),
            Stroke::new(1.0_f32, t.border_strong),
            t.text,
            t.text,
        ),
        ButtonKind::Ghost => (
            Color32::TRANSPARENT,
            t.hover,
            theme::mix(t.hover, t.text, 0.08),
            Stroke::NONE,
            t.muted,
            t.text,
        ),
        ButtonKind::Danger => (
            Color32::TRANSPARENT,
            t.danger_soft,
            theme::mix(t.danger_soft, t.danger, 0.2),
            Stroke::new(1.0_f32, t.danger),
            t.danger,
            t.danger,
        ),
    };
    ui.spacing_mut().button_padding = egui::vec2(14.0, 5.0);
    let widgets = &mut ui.visuals_mut().widgets;
    for (state, fill, color) in [
        (&mut widgets.inactive, rest, text),
        (&mut widgets.hovered, hover, hover_text),
        (&mut widgets.active, press, hover_text),
    ] {
        state.weak_bg_fill = fill;
        state.bg_fill = fill;
        state.bg_stroke = stroke;
        state.fg_stroke = Stroke::new(1.0_f32, color);
        state.corner_radius = radius(scale::RADIUS_CONTROL);
        state.expansion = 0.0;
    }
}

pub fn button(ui: &mut egui::Ui, kind: ButtonKind, label: impl Into<WidgetText>) -> Response {
    button_enabled(ui, kind, true, label)
}

pub fn button_enabled(
    ui: &mut egui::Ui,
    kind: ButtonKind,
    enabled: bool,
    label: impl Into<WidgetText>,
) -> Response {
    ui.scope(|ui| {
        style(ui, kind);
        ui.add_enabled(
            enabled,
            egui::Button::new(label).min_size(egui::vec2(0.0, CONTROL_HEIGHT)),
        )
    })
    .inner
}

/// Style des boutons de barre d'outils : transparents au repos, fond doux au survol,
/// état actif en accent doux. À appliquer dans un `ui.scope` autour des boutons.
pub fn icon_style(ui: &mut egui::Ui) {
    let t = tokens(ui);
    let visuals = ui.visuals_mut();
    visuals.selection.bg_fill = t.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, t.accent);
    let widgets = &mut visuals.widgets;
    for (state, fill) in [
        (&mut widgets.inactive, Color32::TRANSPARENT),
        (&mut widgets.hovered, t.hover),
        (&mut widgets.active, theme::mix(t.hover, t.text, 0.08)),
        (&mut widgets.open, t.hover),
    ] {
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::NONE;
        state.corner_radius = radius(scale::RADIUS_CONTROL);
        state.expansion = 0.0;
    }
}

/// Bouton carré à icône monochrome, sans info-bulle : l'appelant décrit l'action.
pub fn icon_toggle(ui: &mut egui::Ui, icon: Icon, active: bool, label: &str) -> Response {
    let response = ui
        .scope(|ui| {
            icon_style(ui);
            ui.add(
                egui::Button::new(icon.image(ui.ctx(), crate::toolbar::ICON_SIZE))
                    .image_tint_follows_text_color(true)
                    .min_size(egui::vec2(CONTROL_HEIGHT, CONTROL_HEIGHT))
                    .frame_when_inactive(false)
                    .selected(active),
            )
        })
        .inner;
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::Button, response.enabled(), active, label)
    });
    response
}

/// Bouton carré à icône monochrome ; `active` marque un état (gras, lien…).
/// `label` nourrit l'info-bulle et l'accessibilité.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, active: bool, label: &str) -> Response {
    icon_toggle(ui, icon, active, label).on_hover_text(label)
}
