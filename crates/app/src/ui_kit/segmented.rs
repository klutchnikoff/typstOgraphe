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
    let t = tokens(ui);
    // Le segment actif ressort du fond du groupe : plus clair en sombre, surélevé en clair.
    let raised = if ui.visuals().dark_mode {
        theme::mix(t.hover, Color32::WHITE, 0.12)
    } else {
        t.raised
    };
    let mut changed = false;
    let mut response = egui::Frame::new()
        .fill(t.hover)
        .corner_radius(radius(scale::RADIUS_CARD))
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.spacing_mut().button_padding = egui::vec2(11.0, 3.0);
                for (option, label) in options {
                    let selected = *value == *option;
                    let segment = ui.scope(|ui| {
                        let widgets = &mut ui.visuals_mut().widgets;
                        let (rest, text) = if selected {
                            (raised, t.text)
                        } else {
                            (Color32::TRANSPARENT, t.muted)
                        };
                        for (state, fill, color) in [
                            (&mut widgets.inactive, rest, text),
                            (
                                &mut widgets.hovered,
                                if selected {
                                    raised
                                } else {
                                    theme::mix(t.hover, t.text, 0.06)
                                },
                                t.text,
                            ),
                            (&mut widgets.active, raised, t.text),
                        ] {
                            state.weak_bg_fill = fill;
                            state.bg_stroke = Stroke::NONE;
                            state.fg_stroke = Stroke::new(1.0_f32, color);
                            state.corner_radius = radius(scale::RADIUS_CONTROL);
                            state.expansion = 0.0;
                        }
                        ui.add(
                            egui::Button::new(if selected {
                                RichText::new(label).family(theme::semibold())
                            } else {
                                RichText::new(label)
                            })
                            .min_size(egui::vec2(0.0, CONTROL_HEIGHT - 6.0)),
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
                    if segment.clicked() && !selected {
                        *value = *option;
                        changed = true;
                    }
                }
            });
        })
        .response;
    if changed {
        response.mark_changed();
    }
    response
}
