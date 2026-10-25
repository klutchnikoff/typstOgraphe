//! Panneaux de la barre latérale droite (D40, L.2 bis). Aucun panneau n'est déclaré
//! en production : les panneaux réels (plan, assistance, récents) relèvent du chantier G.
use super::*;

impl App {
    pub(super) fn side_panel_ui(&mut self, id: &'static str, ui: &mut egui::Ui) {
        #[cfg(test)]
        if id == "test" {
            ui.label("Panneau de test");
            return;
        }
        let _ = (id, ui);
    }
}
