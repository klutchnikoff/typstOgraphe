//! Composants communs de l’interface (D40, L.2) : boutons à variantes, champs,
//! contrôle segmenté, enveloppe de dialogue, notifications et touches de raccourci.
//!
//! Chaque composant lit les jetons du thème courant (`crate::theme`), conserve les
//! informations d’accessibilité des widgets egui et ne change ni le focus ni la saisie.
mod button;
mod dialog;
mod field;
mod gallery;
mod notice;
mod segmented;
mod side_rail;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

pub use button::{ButtonKind, button, button_enabled, icon_button, icon_style, icon_toggle};
pub use dialog::{Dialog, footer};
pub use field::{field_style, text_area, text_field};
pub use gallery::{Gallery, Orientation, gallery};
pub use notice::{NoticeKind, keycap, notice};
pub use segmented::{segmented, segmented_toggle};
pub use side_rail::{DRAWER_MIN, RAIL_WIDTH, RailLayout, SidePanel, SideRail, reserve};

use crate::theme::Tokens;
use eframe::egui;

/// Hauteur minimale d’un contrôle cliquable.
pub const CONTROL_HEIGHT: f32 = 30.0;
/// Hauteur d’un champ de saisie sur une ligne.
pub const FIELD_HEIGHT: f32 = 32.0;

pub(crate) fn tokens(ui: &egui::Ui) -> Tokens {
    Tokens::new(ui.visuals().dark_mode)
}
pub(crate) fn radius(value: u8) -> egui::CornerRadius {
    egui::CornerRadius::same(value)
}
