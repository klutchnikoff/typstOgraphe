//! Deterministic native review fixture; no files, preferences or external agent.
use super::*;
impl App {
    pub(in crate::app) fn capture_review(&mut self, ctx: &egui::Context, scenario: &str) {
        let mut stored = StoredDocument::demo(DocumentType::Courrier);
        stored.source="Cheres collègues,\n\nJe voudrais savoir si vous avez pu regarder le document envoyé hier, par mes soins, qui concerne la production de blé de l’année suivante.\n\nCette information a un charactère particulièrement importnt pour les décideurs.\n\nJe vous remercie pour votre retour.".into();
        self.install_document(stored, None).expect("review fixture");
        self.preferences = Preferences::default();
        self.preferences.appearance = if scenario.ends_with("-dark") {
            Appearance::Dark
        } else {
            Appearance::Light
        };
        ctx.set_theme(self.preferences.appearance.theme());
        self.start_assistance(ctx);
        let command = |id: &str, params: serde_json::Value| crate::engine::Call {
            id: id.to_owned().try_into().unwrap(),
            params,
        };
        let image = crate::engine::ImageInput::from(&crate::assets::ImageData {
            extension: "svg".into(),
            data: include_bytes!("../../../../../fixtures/signature.svg")
                .as_slice()
                .into(),
        });
        let steps = vec![
            command(
                "text.insert",
                serde_json::json!({"selector":{"select":"Cheres"},"text":"Chères"}),
            ),
            command(
                "text.insert",
                serde_json::json!({"selector":{"select":"Je voudrais savoir si vous avez pu regarder le document envoyé hier, par mes soins, qui concerne"},"text":"Avez-vous pu examiner le document que je vous ai envoyé hier sur"}),
            ),
            command(
                "text.insert",
                serde_json::json!({"selector":{"select":"Cette information a un charactère particulièrement importnt pour les décideurs."},"text":"Cette information est particulièrement importante pour les décideurs."}),
            ),
            command(
                "text.bold",
                serde_json::json!({"selector":{"select":"production de blé"},"value":"on"}),
            ),
            command(
                "metadata.set",
                serde_json::json!({"field":"letter.subject","value":"Production de blé — demande d’informations"}),
            ),
            command("document.set_signature", serde_json::json!({"image":image})),
        ];
        self.assistance
            .session
            .as_mut()
            .expect("local invitation")
            .prepare_proposal(&self.document.engine, "native-review", steps)
            .expect("valid proposal");
        self.poll_assistance();
        self.apply(Command::ReviewProposal);
        if scenario.contains("information") {
            self.assistance.review.as_mut().unwrap().tab = Tab::Information;
        }
        if scenario.contains("preview") {
            self.assistance.review.as_mut().unwrap().tab = Tab::Preview;
        }
        if scenario.contains("stale") {
            self.apply(Command::CloseReview);
            self.document
                .engine
                .run_call(command(
                    "text.insert",
                    serde_json::json!({"text":"Une précision ajoutée. "}),
                ))
                .unwrap();
            self.document.selection = editor::to_editor(self.document.engine.selection());
            self.poll_assistance();
            self.apply(Command::ReviewProposal);
        }
    }
    pub(in crate::app) fn review_capture_ready(&self) -> bool {
        self.assistance.review.as_ref().is_none_or(|r| {
            !r.open || r.tab != Tab::Preview || r.preview.as_ref().is_some_and(|p| p.ready())
        })
    }
}
