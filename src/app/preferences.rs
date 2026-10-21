use super::*;

impl App {
    pub(super) fn preferences_ui(&mut self, ctx: &egui::Context) {
        if !self.preferences_open {
            return;
        }
        let response = egui::Modal::new(egui::Id::new("preferences")).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.heading(crate::tr!("preferences-title"));
            ui.horizontal(|ui| {
                for (page, label) in [
                    (PreferencesPage::Profile, crate::tr!("preferences-profile")),
                    (
                        PreferencesPage::Appearance,
                        crate::tr!("preferences-appearance"),
                    ),
                    (
                        PreferencesPage::Spelling,
                        crate::tr!("preferences-spelling"),
                    ),
                ] {
                    ui.selectable_value(&mut self.preferences_page, page, label);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("preferences-content")
                .max_height((ctx.content_rect().height() - 180.0).max(160.0))
                .show(ui, |ui| match self.preferences_page {
                    PreferencesPage::Profile => self.profile_preferences_ui(ui),
                    PreferencesPage::Appearance => self.appearance_preferences_ui(ui),
                    PreferencesPage::Spelling => self.spelling_preferences_ui(ui),
                });
            ui.add_space(8.0);
            if ui.button(crate::tr!("close")).clicked() {
                self.preferences_open = false;
            }
        });
        ctx.set_theme(self.preferences.appearance.theme());
        if response.should_close() {
            self.preferences_open = false;
        }
        if !self.preferences_open {
            self.force_selection = true;
        }
    }
    pub(super) fn appearance_preferences_ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("appearance-preferences")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(crate::tr!("editor-font"));
                egui::ComboBox::from_id_salt("editor-font")
                    .selected_text(self.preferences.editor.font.label())
                    .show_ui(ui, |ui| {
                        for font in EditorFont::ALL {
                            ui.selectable_value(
                                &mut self.preferences.editor.font,
                                font,
                                font.label(),
                            );
                        }
                    });
                ui.end_row();
                ui.label(crate::tr!("editor-font-size"));
                ui.add(
                    egui::Slider::new(&mut self.preferences.editor.size, 10.0..=36.0).step_by(1.0),
                );
                ui.end_row();
                ui.label(crate::tr!("appearance"));
                egui::ComboBox::from_id_salt("appearance")
                    .selected_text(self.preferences.appearance.label())
                    .show_ui(ui, |ui| {
                        for appearance in Appearance::ALL {
                            ui.selectable_value(
                                &mut self.preferences.appearance,
                                appearance,
                                appearance.label(),
                            );
                        }
                    });
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.weak(crate::tr!("preferences-help"));
    }
    pub(super) fn spelling_preferences_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.preferences.spelling.enabled,
            crate::tr!("cmd-spelling-toggle"),
        );
        ui.weak(crate::tr!("spelling-dictionaries"));
        egui::CollapsingHeader::new(crate::tr!("spelling-personal-dictionary")).show(ui, |ui| {
            let mut remove = None;
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for (language, words) in &self.preferences.spelling.personal {
                        for word in words {
                            ui.horizontal(|ui| {
                                ui.label(format!("{word} ({language})"));
                                if ui
                                    .small_button(crate::tr!("spelling-forget-word"))
                                    .clicked()
                                {
                                    remove = Some((language.clone(), word.clone()));
                                }
                            });
                        }
                    }
                });
            if let Some((language, word)) = remove {
                self.preferences
                    .spelling
                    .personal
                    .get_mut(&language)
                    .unwrap()
                    .remove(&word);
            }
            ui.weak(crate::tr!("spelling-personal-help"));
        });
    }
    pub(super) fn profile_preferences_ui(&mut self, ui: &mut egui::Ui) {
        ui.weak(crate::tr!("profile-help"));
        ui.add_space(8.0);
        egui::Grid::new("profile-fields")
            .num_columns(2)
            .show(ui, |ui| {
                let profile = &mut self.preferences.profile;
                for (key, value, multiline) in [
                    ("field-full-name", &mut profile.name, false),
                    ("field-organisation", &mut profile.organisation, false),
                    ("field-role", &mut profile.role, false),
                    ("field-sender-address", &mut profile.address, true),
                    ("field-email", &mut profile.email, false),
                    ("field-phone", &mut profile.phone, false),
                ] {
                    let label = ui.label(crate::i18n::text(key));
                    let edit = if multiline {
                        egui::TextEdit::multiline(value).desired_rows(3)
                    } else {
                        egui::TextEdit::singleline(value)
                    };
                    ui.add(edit.id_salt(("profile", key)).desired_width(310.0))
                        .labelled_by(label.id);
                    ui.end_row();
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(crate::tr!("signature-label"));
            if ui
                .button(if self.preferences.profile.signature.is_some() {
                    crate::tr!("image-replace")
                } else {
                    crate::tr!("signature-choose")
                })
                .clicked()
            {
                self.choose_signature(true);
            }
            if self.preferences.profile.signature.is_some()
                && ui.button(crate::tr!("image-remove")).clicked()
            {
                self.preferences.profile.signature = None;
                self.profile_signature = None;
                self.profile_error = None;
            }
        });
        if let (Some(name), Some(image)) = (
            self.preferences.profile.signature.clone(),
            self.profile_signature.clone(),
        ) {
            self.signature_preview(ui, &format!("profile:{name}"), &image);
        } else if self.preferences.profile.signature.is_none() {
            ui.weak(crate::tr!("signature-none"));
        }
        ui.weak(crate::tr!("profile-signature-help"));
        if let Some(error) = &self.profile_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
    pub(super) fn persist_preferences(
        &mut self,
        storage: &mut dyn eframe::Storage,
        now: Instant,
        force: bool,
    ) {
        if self.capture.path.is_some() {
            return;
        }
        if let Some(delay) = self.preferences_tracker.observe(&self.preferences, now)
            && (force || delay.is_zero())
        {
            self.preferences_tracker.save(&self.preferences, storage);
        }
    }
    pub fn capture_profile(&mut self, ctx: &egui::Context, preferences: bool) {
        self.preferences.appearance = Appearance::Light;
        ctx.set_theme(egui::ThemePreference::Light);
        self.preferences.profile = crate::profile::Profile {
            name: "Camille Martin".into(),
            organisation: "Association des Tilleuls".into(),
            address: "12, rue des Tilleuls\n35000 Rennes".into(),
            email: "camille@example.org".into(),
            phone: "02 00 00 00 00".into(),
            role: "Trésorière".into(),
            signature: Some("signature-demo.svg".into()),
        };
        self.profile_signature = Some(crate::assets::ImageData {
            extension: "svg".into(),
            data: include_bytes!("../../tests/fixtures/signature.svg")
                .as_slice()
                .into(),
        });
        self.create_document(DocumentType::Courrier);
        let before = self.snapshot();
        if let crate::document_types::Metadata::Courrier(meta) = &mut self.session.metadata {
            meta.destinataire = "Secrétariat de la mairie".into();
            meta.objet = "Confirmation de notre rendez-vous".into();
            meta.lieu = "Rennes".into();
            meta.date = "2026-10-21".into();
        }
        self.session.document.view.insert_text("Madame, Monsieur,\n\nJe vous confirme notre rendez-vous pour préparer la prochaine rencontre de l’association.\n\nJe vous remercie pour votre disponibilité.\n\nBien cordialement,", 0);
        self.remember(before, false);
        self.preferences_page = PreferencesPage::Profile;
        self.preferences_open = preferences;
        if !preferences {
            self.information_group = InformationGroup::Fields("information");
            self.information_open = true;
        }
    }
}
