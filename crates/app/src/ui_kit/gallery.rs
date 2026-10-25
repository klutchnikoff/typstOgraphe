//! Feuille de composants : sert de capture de revue et de test de présentation.
use super::*;
use crate::icons::Icon;
use eframe::egui::{RichText, Ui};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gallery {
    pub text: String,
    pub area: String,
    pub orientation: Orientation,
    pub icon_active: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            text: "Service des équipements".into(),
            area: "12, place de la République\n35000 Valmont".into(),
            orientation: Orientation::Portrait,
            icon_active: true,
        }
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(11.0)
            .color(tokens(ui).muted),
    );
}

pub fn gallery(ui: &mut Ui, state: &mut Gallery) {
    section(ui, "Boutons");
    ui.horizontal_wrapped(|ui| {
        button(ui, ButtonKind::Primary, "Principal");
        button(ui, ButtonKind::Secondary, "Secondaire");
        button(ui, ButtonKind::Ghost, "Discret");
        button(ui, ButtonKind::Danger, "Destructif");
        button_enabled(ui, ButtonKind::Secondary, false, "Désactivé");
    });
    ui.horizontal(|ui| {
        for (icon, label) in [(Icon::Bold, "Gras"), (Icon::Italic, "Italique")] {
            icon_button(ui, icon, false, label);
        }
        if icon_button(ui, Icon::Link, state.icon_active, "Lien").clicked() {
            state.icon_active = !state.icon_active;
        }
        icon_button(ui, Icon::Settings, false, "Paramètres");
    });
    section(ui, "Champs et choix");
    ui.label("Destinataire");
    text_field(ui, "gallery-text", &mut state.text);
    ui.label("Adresse");
    text_area(ui, "gallery-area", &mut state.area, 2);
    let options = [
        (Orientation::Portrait, "Portrait".to_owned()),
        (Orientation::Landscape, "Paysage".to_owned()),
    ];
    segmented(ui, &mut state.orientation, &options);
    section(ui, "Notifications et touches");
    notice(
        ui,
        NoticeKind::Info,
        "Une proposition de l’agent est prête.",
    );
    notice(ui, NoticeKind::Success, "Document enregistré.");
    notice(
        ui,
        NoticeKind::Error,
        "Impossible d’écrire dans ce dossier.",
    );
    ui.horizontal(|ui| {
        ui.label("Liste à puces");
        keycap(ui, "⌘⇧8");
    });
}
