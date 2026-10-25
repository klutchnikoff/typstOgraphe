use super::*;

impl App {
    pub(super) fn information_ui(&mut self, ui: &mut egui::Ui) {
        let groups: Vec<_> =
            std::iter::once((InformationGroup::General, crate::tr!("group-general")))
                .chain(
                    self.document
                        .engine
                        .metadata()
                        .editor_groups()
                        .iter()
                        .map(|group| {
                            (
                                InformationGroup::Fields(group.id),
                                crate::i18n::text(group.label_key),
                            )
                        }),
                )
                .collect();
        let full_heading = match self.document.engine.metadata().document_type() {
            DocumentType::Courrier => crate::tr!("document-settings-heading"),
            DocumentType::CompteRendu => crate::tr!("minutes-settings-heading"),
        };
        let font = egui::TextStyle::Button.resolve(ui.style());
        let label_width = |label: &str| {
            ui.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(label.to_owned(), font.clone(), ui.visuals().text_color())
                    .size()
                    .x
            })
        };
        let controls_width: f32 = groups
            .iter()
            .map(|(_, label)| label_width(label) + 2.0 * ui.spacing().button_padding.x)
            .sum();
        let heading = if label_width(&full_heading)
            + controls_width
            + groups.len() as f32 * ui.spacing().item_spacing.x
            <= ui.available_width()
        {
            full_heading.clone()
        } else {
            crate::tr!("document-settings-short")
        };
        let mut clicked = None;
        ui.scope(|ui| {
            crate::toolbar::neutral_selection(ui);
            ui.spacing_mut().button_padding = egui::vec2(6.0, 0.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Label::new(heading).truncate())
                    .on_hover_text(&full_heading);
                for (group, label) in groups {
                    let active =
                        self.dialogs.information_open && self.dialogs.information_group == group;
                    if ui
                        .add(egui::Button::selectable(active, label.clone()).small())
                        .on_hover_text(if active {
                            crate::tr!("document-settings-close")
                        } else {
                            label
                        })
                        .clicked()
                    {
                        clicked = Some(group);
                    }
                }
            });
        });
        if let Some(group) = clicked {
            let close = self.dialogs.information_open && self.dialogs.information_group == group;
            if let Some(id) = self.document.history_field.take() {
                ui.ctx().memory_mut(|memory| memory.surrender_focus(id));
            }
            self.dialogs.information_group = group;
            if close || !self.dialogs.information_open {
                self.apply(Command::Information);
            }
            ui.ctx().request_repaint();
        }
        if self.dialogs.information_open {
            // Une seule rubrique est visible ; chaque rubrique garde son défilement.
            let max_height = (ui.available_height() * 0.45).min(320.0);
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt(("information-scroll", self.dialogs.information_group))
                .max_height(max_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add_enabled_ui(!self.document.engine.is_read_only(), |ui| {
                        match self.dialogs.information_group {
                            InformationGroup::General => self.general_information_ui(ui),
                            InformationGroup::Fields(group) => {
                                self.information_fields_ui(ui, group);
                                if group == "sender" {
                                    self.document_signature_ui(ui);
                                }
                            }
                        }
                    });
                });
        }
    }
    fn general_information_ui(&mut self, ui: &mut egui::Ui) {
        let fixed_page = self.document.engine.fixed_page();
        let mut options = self.document.engine.metadata().options().clone();
        let mut previous = options.clone();
        egui::Grid::new("general-fields")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(crate::tr!("field-document-type"));
                ui.weak(self.keymap.registry().document_type().label());
                ui.end_row();
                ui.label(crate::tr!("field-template"));
                self.template_selector(ui);
                options = self.document.engine.metadata().options().clone();
                previous = options.clone();
                ui.end_row();
                ui.label(crate::tr!("field-paper"));
                if let Some((paper, _)) = fixed_page {
                    ui.label(paper.label())
                        .on_hover_text(crate::tr!("model-fixed-page"));
                } else {
                    egui::ComboBox::from_id_salt("document-paper")
                        .selected_text(options.paper.label())
                        .show_ui(ui, |ui| {
                            for paper in PaperFormat::ALL {
                                ui.selectable_value(&mut options.paper, paper, paper.label());
                            }
                        });
                }
                ui.end_row();
                ui.label(crate::tr!("field-orientation"));
                if let Some((_, landscape)) = fixed_page {
                    ui.label(crate::i18n::text(if landscape {
                        "orientation-landscape"
                    } else {
                        "orientation-portrait"
                    }))
                    .on_hover_text(crate::tr!("model-fixed-page"));
                } else {
                    egui::ComboBox::from_id_salt("document-orientation")
                        .selected_text(if options.landscape {
                            crate::tr!("orientation-landscape")
                        } else {
                            crate::tr!("orientation-portrait")
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut options.landscape,
                                false,
                                crate::tr!("orientation-portrait"),
                            );
                            ui.selectable_value(
                                &mut options.landscape,
                                true,
                                crate::tr!("orientation-landscape"),
                            );
                        });
                }
                ui.end_row();
                ui.label(crate::tr!("field-document-language"));
                egui::ComboBox::from_id_salt("document-language")
                    .selected_text(options.language.label())
                    .show_ui(ui, |ui| {
                        for language in DocumentLanguage::ALL {
                            ui.selectable_value(&mut options.language, language, language.label());
                        }
                    });
                ui.end_row();
                ui.label(crate::tr!("field-bullet-marker"));
                self.bullet_marker_ui(ui);
                ui.end_row();
            });
        use crate::engine::DocumentOption;
        for option in [
            (options.paper != previous.paper).then_some(DocumentOption::Paper(options.paper)),
            (options.landscape != previous.landscape)
                .then_some(DocumentOption::Landscape(options.landscape)),
            (options.language != previous.language)
                .then_some(DocumentOption::Language(options.language)),
        ]
        .into_iter()
        .flatten()
        {
            self.set_document_option(option);
        }
    }
    pub(super) fn set_document_option(&mut self, option: crate::engine::DocumentOption) -> bool {
        self.run_document_command(crate::engine::Command::SetOption(
            crate::engine::SetOptionParams {
                guard: self.document.engine.target_guard(),
                option,
            },
        ))
    }
    pub(super) fn bullet_marker_ui(&mut self, ui: &mut egui::Ui) {
        let policy = self.document.presentation.clone();
        let previous = self
            .document
            .engine
            .metadata()
            .options()
            .bullet_marker
            .clone();
        if policy.bullet_markers.is_empty() && !policy.custom_bullet_marker {
            if previous.is_some() {
                if ui
                    .button(crate::tr!("bullet-marker-restore-automatic"))
                    .clicked()
                {
                    self.set_document_option(crate::engine::DocumentOption::BulletMarker(None));
                    self.dialogs.marker_draft = None;
                }
            } else {
                ui.weak(crate::tr!("bullet-marker-automatic"))
                    .on_hover_text(crate::tr!("bullet-marker-model-controlled"));
            }
            return;
        }
        let guard = self.document.engine.target_guard();
        let mut marker = previous.clone();
        let mut draft = None;
        ui.vertical(|ui| {
            egui::ComboBox::from_id_salt("document-bullet-marker")
                .selected_text(
                    marker
                        .as_deref()
                        .map_or_else(|| crate::tr!("bullet-marker-automatic"), str::to_owned),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut marker, None, crate::tr!("bullet-marker-automatic"));
                    for choice in &policy.bullet_markers {
                        ui.selectable_value(&mut marker, Some(choice.clone()), choice);
                    }
                    if policy.custom_bullet_marker
                        && ui
                            .selectable_label(marker.is_some(), crate::tr!("bullet-marker-custom"))
                            .clicked()
                    {
                        marker = Some("◆".into());
                    }
                });
            if policy.custom_bullet_marker
                && let Some(value) = &mut marker
            {
                let mut input = self
                    .dialogs
                    .marker_draft
                    .as_ref()
                    .filter(|(g, _)| *g == guard && previous.as_deref() == Some(value.as_str()))
                    .map_or_else(|| value.clone(), |(_, s)| s.clone());
                ui.add(
                    egui::TextEdit::singleline(&mut input)
                        .id_salt("custom-bullet-marker")
                        .desired_width(55.0),
                );
                if crate::document::valid_bullet_marker(&input) {
                    *value = input.clone();
                } else {
                    ui.colored_label(
                        crate::theme::ERROR_INLINE,
                        crate::tr!("bullet-marker-invalid"),
                    );
                }
                draft = Some(input);
            }
        })
        .response
        .on_hover_text(crate::tr!("bullet-marker-model-choices"));
        if marker != previous {
            self.set_document_option(crate::engine::DocumentOption::BulletMarker(marker));
        }
        self.dialogs.marker_draft = draft.map(|s| (self.document.engine.target_guard(), s));
    }
}
