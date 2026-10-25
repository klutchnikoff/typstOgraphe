//! Reproducible native QA, isolated from user documents and preferences.
use super::*;
impl App {
    pub fn capture_widgets(&mut self, ctx: &egui::Context, scenario: &str) {
        if scenario.starts_with("review") {
            self.capture_review(ctx, scenario);
            return;
        }
        let mut stored = StoredDocument::demo(DocumentType::Courrier);
        stored.source="Madame, Monsieur,\n\nNous souhaitons confirmer la réservvation de la salle des Tilleuls pour notre prochaine rencontre.\n\n- Accueil des participants\n- Installation de la salle\n- Trente places assises\n- Un vidéoprojecteur\n\n#figure(image(\"images/parc.svg\", alt: \"Allée arborée et bancs du parc.\"), caption: [Le parc des Tilleuls, à proximité de la salle.])\n\n#quote(block: true, attribution: [Projet associatif])[Un lieu accessible à toutes et à tous, propice aux échanges.]\n\nJe vous remercie pour votre retour.".into();
        stored.assets.insert(
            "images/parc.svg".into(),
            include_bytes!("../../../../../fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        let mode = scenario.strip_suffix("-dark").unwrap_or(scenario);
        if mode == "list-empty" {
            stored.source = "Madame, Monsieur,\n\n- Un premier élément suffisamment long pour se replier et vérifier le retrait du texte dans la colonne de gauche.\n- ".into();
        }
        if mode == "spelling-block" {
            stored.source = "Madame, Monsieur,\n\nNous aurons besoin des équipements suivants :\n\n- Accueil des participants\n- Installation de la salle\n- Un vidéoprojecteure\n- Trente places assises\n\nJe vous remercie pour votre retour.".into();
        }
        if mode == "quote" {
            stored.source="Madame, Monsieur,\n\n/ Accueil: Un temps d’échange avec les associations.\n/ Équipement: Une salle accessible et un vidéoprojecteur.\n\n#quote(block: true, attribution: [Projet associatif])[Un lieu accessible à toutes et à tous, propice aux échanges.\n\nLa participation des habitants est au cœur de notre démarche.]\n\nJe vous remercie pour votre retour.".into();
        }
        if mode == "notes" {
            stored.source = format!(
                "{}Madame, Monsieur,\n\nJe vous remercie pour votre retour.\n\n#postscript[Vous pouvez me joindre le matin.\n\nUne permanence est assurée le jeudi.]\n\n#attachment[Formulaire d’inscription]\n#attachment[Programme de la rencontre]",
                crate::document::LETTER_NOTES_IMPORT
            );
        }
        self.install_document(stored, None)
            .expect("valid widgets fixture");
        self.preferences = Preferences::default();
        let dark = scenario.ends_with("-dark");
        self.preferences.appearance = if dark {
            Appearance::Dark
        } else {
            Appearance::Light
        };
        ctx.set_theme(self.preferences.appearance.theme());
        let scenario = scenario.strip_suffix("-dark").unwrap_or(scenario);
        match scenario {
            "context-list" | "context-spelling" => {
                let text = &self.document.engine.document().view.text;
                let needle = if scenario == "context-list" {
                    "Accueil"
                } else {
                    "réservvation"
                };
                let at = text[..text.find(needle).expect("context fixture")]
                    .chars()
                    .count();
                self.document.selection = editor::cursor(at..at);
                self.document.force_selection = true;
                // Synchronize the fixture before requesting suggestions so the
                // first frame does not invalidate their worker generation.
                self.refresh_spelling(ctx);
                self.apply(Command::ContextualAction);
                self.spelling.capture_active = scenario == "context-spelling";
            }
            "list" | "remove" => {
                let widget = editor::widgets(&self.document.engine.document().view)
                    .into_iter()
                    .find(|w| w.kind == WidgetKind::List)
                    .expect("list fixture");
                self.open_widget_band(widget, scenario == "remove");
            }
            "quote" | "notes" => {}
            "palette" => {
                self.dialogs.palette_open = true;
                self.dialogs.palette_query = "li".into();
            }
            "list-empty" => {
                let at = self.document.engine.document().view.len();
                self.document.selection = editor::cursor(at..at);
                self.document.force_selection = true;
            }
            "spelling" | "spelling-block" => self.spelling.capture_active = true,
            "link" => {
                self.document.selection = editor::cursor(0..7);
                self.open_link();
            }
            _ => self.open_image_info(1),
        }
    }
}
