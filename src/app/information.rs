use super::*;

impl App {
    pub(super) fn information_ui(&mut self, ui: &mut egui::Ui) {
        let before = self.snapshot();
        let old_revision = self.document.engine.revision;
        let groups: Vec<_> =
            std::iter::once((InformationGroup::General, crate::tr!("group-general")))
                .chain(
                    self.document
                        .engine
                        .metadata
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
        let full_heading = match self.document.engine.metadata.document_type() {
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
                .show(ui, |ui| match self.dialogs.information_group {
                    InformationGroup::General => self.general_information_ui(ui),
                    InformationGroup::Fields(group) => {
                        self.document.engine.metadata.editor_group_ui(ui, group);
                        if group == "sender" {
                            self.document_signature_ui(ui);
                        }
                    }
                });
        }
        if self.document.engine.revision == old_revision {
            let focused = ui.ctx().memory(|memory| memory.focused());
            let group = focused
                .filter(|id| egui::TextEdit::load_state(ui.ctx(), *id).is_some())
                .map_or(EditGroup::Discrete, |id| EditGroup::Metadata(id.value()));
            let previous = before.selection;
            let result = self
                .document
                .engine
                .record(before, self.document.selection.into(), group);
            self.accept_edit(result, previous);
        }
    }
    fn general_information_ui(&mut self, ui: &mut egui::Ui) {
        let fixed_page = crate::document_types::models::ALL
            .iter()
            .find(|model| model.source == self.document.engine.template)
            .and_then(|model| model.page);
        egui::Grid::new("general-fields")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(crate::tr!("field-document-type"));
                ui.weak(self.keymap.registry().document_type().label());
                ui.end_row();
                ui.label(crate::tr!("field-template"));
                self.template_selector(ui);
                ui.end_row();
                ui.label(crate::tr!("field-paper"));
                if let Some((paper, _)) = fixed_page {
                    ui.label(paper.label())
                        .on_hover_text(crate::tr!("model-fixed-page"));
                } else {
                    egui::ComboBox::from_id_salt("document-paper")
                        .selected_text(self.document.engine.metadata.options().paper.label())
                        .show_ui(ui, |ui| {
                            for paper in PaperFormat::ALL {
                                ui.selectable_value(
                                    &mut self.document.engine.metadata.options_mut().paper,
                                    paper,
                                    paper.label(),
                                );
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
                        .selected_text(if self.document.engine.metadata.options().landscape {
                            crate::tr!("orientation-landscape")
                        } else {
                            crate::tr!("orientation-portrait")
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.document.engine.metadata.options_mut().landscape,
                                false,
                                crate::tr!("orientation-portrait"),
                            );
                            ui.selectable_value(
                                &mut self.document.engine.metadata.options_mut().landscape,
                                true,
                                crate::tr!("orientation-landscape"),
                            );
                        });
                }
                ui.end_row();
                ui.label(crate::tr!("field-document-language"));
                egui::ComboBox::from_id_salt("document-language")
                    .selected_text(self.document.engine.metadata.options().language.label())
                    .show_ui(ui, |ui| {
                        for language in DocumentLanguage::ALL {
                            ui.selectable_value(
                                &mut self.document.engine.metadata.options_mut().language,
                                language,
                                language.label(),
                            );
                        }
                    });
                ui.end_row();
                ui.label(crate::tr!("field-bullet-marker"));
                self.bullet_marker_ui(ui);
                ui.end_row();
            });
    }
    pub(super) fn bullet_marker_ui(&mut self, ui: &mut egui::Ui) {
        let policy = &self.document.presentation;
        if policy.bullet_markers.is_empty() && !policy.custom_bullet_marker {
            if self
                .document
                .engine
                .metadata
                .options()
                .bullet_marker
                .is_some()
            {
                if ui
                    .button(crate::tr!("bullet-marker-restore-automatic"))
                    .clicked()
                {
                    self.document.engine.metadata.options_mut().bullet_marker = None;
                }
            } else {
                ui.weak(crate::tr!("bullet-marker-automatic"))
                    .on_hover_text(crate::tr!("bullet-marker-model-controlled"));
            }
            return;
        }
        let marker = &mut self.document.engine.metadata.options_mut().bullet_marker;
        ui.vertical(|ui| {
            egui::ComboBox::from_id_salt("document-bullet-marker")
                .selected_text(
                    marker
                        .as_deref()
                        .map_or_else(|| crate::tr!("bullet-marker-automatic"), str::to_owned),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(marker, None, crate::tr!("bullet-marker-automatic"));
                    for choice in &policy.bullet_markers {
                        ui.selectable_value(marker, Some(choice.clone()), choice);
                    }
                    if policy.custom_bullet_marker
                        && ui
                            .selectable_label(marker.is_some(), crate::tr!("bullet-marker-custom"))
                            .clicked()
                    {
                        *marker = Some("◆".into());
                    }
                });
            if policy.custom_bullet_marker
                && let Some(value) = marker
            {
                ui.add(
                    egui::TextEdit::singleline(value)
                        .id_salt("custom-bullet-marker")
                        .desired_width(55.0),
                );
                if !crate::document::valid_bullet_marker(value) {
                    ui.colored_label(
                        Color32::from_rgb(183, 69, 57),
                        crate::tr!("bullet-marker-invalid"),
                    );
                }
            }
        })
        .response
        .on_hover_text(crate::tr!("bullet-marker-model-choices"));
    }
}
