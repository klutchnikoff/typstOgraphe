use super::{CONTROL_HEIGHT, radius, tokens};
use crate::theme::{self, scale};
use eframe::egui::{self, Color32, Response, RichText, Stroke, WidgetInfo, WidgetType};

/// Contrôle segmenté pour 2 à 4 choix exclusifs. `response.changed()` indique un
/// changement de valeur ; chaque segment est un bouton radio pour l’accessibilité.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, String)],
) -> Response {
    let current = *value;
    let (mut response, clicked) = segments(ui, options, |option| *option == current);
    if let Some(option) = clicked.filter(|option| *option != current) {
        *value = option;
        response.mark_changed();
    }
    response
}

/// Variante où le segment actif peut être cliqué de nouveau (ouvrir/fermer une
/// rubrique) : retourne le segment cliqué, qu’il soit actif ou non.
pub fn segmented_toggle<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    selected: Option<T>,
    options: &[(T, String)],
) -> Option<T> {
    segments(ui, options, |option| selected == Some(*option)).1
}

fn segments<T: Copy>(
    ui: &mut egui::Ui,
    options: &[(T, String)],
    is_selected: impl Fn(&T) -> bool,
) -> (Response, Option<T>) {
    let t = tokens(ui);
    // Le segment actif ressort du fond du groupe : plus clair en sombre, surélevé en clair.
    let raised = if ui.visuals().dark_mode {
        theme::mix(t.hover, Color32::WHITE, 0.12)
    } else {
        t.raised
    };
    let mut clicked = None;
    let response = egui::Frame::new()
        .fill(t.hover)
        .corner_radius(radius(scale::RADIUS_CARD))
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.spacing_mut().button_padding = egui::vec2(11.0, 3.0);
                for (option, label) in options {
                    let selected = is_selected(option);
                    let segment = ui.scope(|ui| {
                        let widgets = &mut ui.visuals_mut().widgets;
                        let (rest, text) = if selected {
                            (raised, t.text)
                        } else {
                            (Color32::TRANSPARENT, t.muted)
                        };
                        let hover = if selected {
                            raised
                        } else {
                            theme::mix(t.hover, t.text, 0.06)
                        };
                        for (state, fill, color) in [
                            (&mut widgets.inactive, rest, text),
                            (&mut widgets.hovered, hover, t.text),
                            (&mut widgets.active, raised, t.text),
                        ] {
                            state.weak_bg_fill = fill;
                            state.bg_stroke = Stroke::NONE;
                            state.fg_stroke = Stroke::new(1.0_f32, color);
                            state.corner_radius = radius(scale::RADIUS_CONTROL);
                            state.expansion = 0.0;
                        }
                        let text = if selected {
                            RichText::new(label).family(theme::semibold())
                        } else {
                            RichText::new(label)
                        };
                        ui.add(
                            egui::Button::new(text).min_size(egui::vec2(0.0, CONTROL_HEIGHT - 6.0)),
                        )
                    });
                    let segment = segment.inner;
                    segment.widget_info(|| {
                        WidgetInfo::selected(
                            WidgetType::RadioButton,
                            segment.enabled(),
                            selected,
                            label,
                        )
                    });
                    if segment.clicked() {
                        clicked = Some(*option);
                    }
                }
            });
        })
        .response;
    (response, clicked)
}
