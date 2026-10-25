use super::{radius, tokens};
use crate::theme::scale;
use eframe::egui::{self, Response, RichText, Stroke, WidgetText};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Neutral,
    Info,
    Success,
    Error,
}

/// Bandeau d’information : fond doux, trait et texte de la même famille.
pub fn notice(ui: &mut egui::Ui, kind: NoticeKind, text: impl Into<WidgetText>) -> Response {
    let t = tokens(ui);
    let (fill, line, ink) = match kind {
        NoticeKind::Neutral => (t.hover, t.border, t.text),
        NoticeKind::Info => (t.accent_soft, t.accent, t.text),
        NoticeKind::Success => (t.ok_soft, t.ok, t.text),
        NoticeKind::Error => (t.danger_soft, t.danger, t.text),
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, line))
        .corner_radius(radius(scale::RADIUS_CARD))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(text.into().color(ink));
        })
        .response
}

/// Touche de raccourci : petit cadre à fond de panneau, texte à chasse fixe.
pub fn keycap(ui: &mut egui::Ui, text: &str) -> Response {
    let t = tokens(ui);
    egui::Frame::new()
        .fill(t.panel)
        .stroke(Stroke::new(1.0_f32, t.border))
        .corner_radius(radius(4))
        .inner_margin(egui::Margin::symmetric(5, 0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace().size(11.5).color(t.muted));
        })
        .response
}

/// Pastille d'état (barre d'état) : texte court sur fond doux, cliquable.
pub fn chip(ui: &mut egui::Ui, kind: NoticeKind, text: impl Into<String>) -> Response {
    let t = tokens(ui);
    let (fill, ink) = match kind {
        NoticeKind::Neutral => (t.hover, t.muted),
        NoticeKind::Info => (t.accent_soft, t.accent),
        NoticeKind::Success => (t.ok_soft, t.ok),
        NoticeKind::Error => (t.danger_soft, t.danger),
    };
    let text = text.into();
    let shown = egui::Frame::new()
        .fill(fill)
        .corner_radius(radius(scale::RADIUS_DIALOG))
        .inner_margin(egui::Margin::symmetric(9, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(&text).size(scale::TEXT[1]).color(ink));
        })
        .response;
    let response = ui.interact(shown.rect, shown.id.with("chip"), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &text)
    });
    response
}
