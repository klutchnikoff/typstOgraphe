use super::*;
use crate::icons::Icon;

impl App {
    pub(in crate::app) fn widget_controls(
        &mut self,
        ui: &mut egui::Ui,
        view: &Projection,
        output: &egui::text_edit::TextEditOutput,
    ) -> Option<EditorAction> {
        let mut action = None;
        let clip = ui.clip_rect().intersect(output.text_clip_rect);
        let contextual = self.contextual_action();
        for widget in editor::widgets(view) {
            let rect =
                editor::widget_rect(&output.galley, &widget).translate(output.galley_pos.to_vec2());
            let header = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width(), editor::WIDGET_HEADER),
            );
            self.bands.hit_regions.push(header.intersect(clip));
            if !header.intersects(clip) {
                continue;
            }
            ui.scope_builder(
                egui::UiBuilder::new()
                    .id_salt(("widget-header", widget.range.start))
                    .max_rect(header.shrink2(egui::vec2(8.0, 2.0))),
                |ui| {
                    ui.set_clip_rect(clip.intersect(header));
                    ui.horizontal(|ui| {
                        ui.weak(egui::RichText::new(widget.kind.label()).small());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let enabled = !self.document.engine.is_read_only()
                                && self.document.ime.draft.is_none()
                                && !self.bands.parameter_ime;
                            if icon_button(ui, Icon::Trash, enabled, &crate::tr!("widget-delete"))
                                .clicked()
                            {
                                action = Some(EditorAction::Remove(widget.clone()));
                            }
                            if icon_button(
                                ui,
                                Icon::Settings,
                                enabled && widget.kind.configurable(),
                                &self.widget_settings_hint(ui.ctx(), &widget, contextual.as_ref()),
                            )
                            .clicked()
                            {
                                action = Some(EditorAction::Settings(widget.clone()));
                            }
                        });
                    });
                },
            );
        }
        action
    }
    pub(in crate::app) fn paint_band(
        &mut self,
        ui: &mut egui::Ui,
        view: &Projection,
        output: &egui::text_edit::TextEditOutput,
    ) -> Option<EditorAction> {
        let inset = self.band_inset()?;
        let line = view.line_at(inset.anchor.min(view.len()));
        let start = view.line_ranges()[line].start;
        let mut cursor = CCursor::new(start);
        cursor.prefer_next_row = true;
        let mut y = output.galley.pos_from_cursor(cursor).top();
        if matches!(view.blocks[line], BlockKind::Image(_)) {
            y -= editor::IMAGE_HEIGHT + editor::IMAGE_CAPTION_GAP;
        }
        if editor::widgets(view).iter().any(|w| w.lines.start == line) {
            y -= editor::WIDGET_HEADER;
        }
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                ui.clip_rect().left(),
                output.galley_pos.y + y - inset.height,
            ),
            egui::vec2(ui.clip_rect().width(), inset.height),
        );
        if self.bands.open.as_ref()?.focus_pending {
            ui.scroll_to_rect(rect, Some(egui::Align::Min));
        }
        if !rect.intersects(output.text_clip_rect) {
            return None;
        }
        self.bands.hit_regions.push(rect.intersect(ui.clip_rect()));
        let submit = ui.is_enabled() && std::mem::take(&mut self.bands.submit_pending);
        let suggestions = self
            .spelling
            .checker
            .suggestions
            .as_ref()
            .and_then(|(_, s)| s.as_deref());
        let band = self.bands.open.as_mut()?;
        if self.document.engine.resolve_target(band.target).is_err() {
            band.error = Some(crate::tr!("edit-stale-draft"));
        }
        let fill = ui.visuals().selection.bg_fill.gamma_multiply(0.16);
        ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
        ui.painter().rect_filled(rect, 0.0, fill);
        ui.painter().hline(
            rect.x_range(),
            rect.top(),
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
        let mut action = None;
        let mut measured = 0.0;
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("editor-band")
                .max_rect(rect.shrink2(egui::vec2(12.0, 8.0))),
            |ui| {
                ui.set_clip_rect(rect.intersect(ui.clip_rect()));
                ui.set_min_width((rect.width() - 24.0).max(1.0));
                ui.horizontal(|ui| {
                    ui.strong(band.title());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close = icon_button(ui, Icon::Close, true, &crate::tr!("widget-close"));
                        if band.keyboard_focus
                            && band.focus_pending
                            && matches!(band.content, BandContent::Spelling { .. })
                            && suggestions.is_some_and(|s| s.is_empty())
                        {
                            close.request_focus();
                        }
                        if close.clicked() || (submit && close.has_focus()) {
                            action = Some(BandAction::Close);
                        }
                    });
                });
                let body_action = band_body(ui, band, suggestions, submit);
                if action.is_none() {
                    action = body_action;
                }
                measured = ui.min_rect().height() + 16.0;
            },
        );
        band.focus_pending = band.keyboard_focus
            && matches!(band.content, BandContent::Spelling { .. })
            && suggestions.is_none();
        if !band.focus_pending {
            band.keyboard_focus = false;
        }
        if (measured - band.height).abs() > 1.0 {
            band.height = measured;
            ui.ctx().request_repaint();
        }
        action.map(EditorAction::Band)
    }
}
fn icon_button(ui: &mut egui::Ui, icon: Icon, enabled: bool, label: &str) -> egui::Response {
    let color = ui.visuals().text_color();
    let response = ui
        .add_enabled(
            enabled,
            egui::Button::image(icon.image(ui.ctx(), crate::toolbar::ICON_SIZE).tint(color))
                .frame(false)
                .min_size(egui::vec2(25.0, 24.0)),
        )
        .on_hover_text(label)
        .on_disabled_hover_text(label);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response
}

fn band_body(
    ui: &mut egui::Ui,
    band: &mut Band,
    suggestions: Option<&[String]>,
    submit: bool,
) -> Option<BandAction> {
    let mut action = None;
    let mut apply = false;
    match &mut band.content {
        BandContent::Image { alt, .. } => {
            ui.label(crate::tr!("image-alt-label"));
            let response = ui.add(
                egui::TextEdit::singleline(alt)
                    .id(egui::Id::new("editor-band-input"))
                    .desired_width(f32::INFINITY),
            );
            if band.focus_pending {
                response.request_focus();
            }
            ui.horizontal_wrapped(|ui| {
                if band_button(ui, crate::tr!("image-replace"), submit) {
                    action = Some(BandAction::ReplaceImage);
                }
                ui.weak(crate::tr!("widget-caption-direct"));
            });
            apply = true;
        }
        BandContent::List { kind } => {
            ui.horizontal_wrapped(|ui| {
                for (value, key) in [
                    (BlockKind::Bullet, "widget-bullets"),
                    (BlockKind::Numbered, "widget-numbered"),
                    (BlockKind::Description, "widget-descriptions"),
                ] {
                    let response = ui.radio_value(kind, value, crate::i18n::text(key));
                    if band.keyboard_focus && band.focus_pending && *kind == value {
                        response.request_focus();
                    }
                }
            });
            ui.weak(crate::tr!("widget-list-scope"));
            apply = true;
        }
        BandContent::Link { url } => {
            ui.label(crate::tr!("field-address"));
            let response = ui.add(
                egui::TextEdit::singleline(url)
                    .id(egui::Id::new("editor-band-input"))
                    .desired_width(f32::INFINITY)
                    .hint_text("https://…"),
            );
            if band.focus_pending {
                response.request_focus();
            }
            if band_button(ui, Command::RemoveLink.short_label(), submit) {
                action = Some(BandAction::Remove);
            }
            apply = true;
        }
        BandContent::Remove { kind, .. } => {
            ui.label(crate::i18n::text(if *kind == WidgetKind::List {
                "widget-delete-list-help"
            } else {
                "widget-delete-help"
            }));
            ui.horizontal(|ui| {
                if band_button(ui, crate::tr!("cancel"), submit) {
                    action = Some(BandAction::Close);
                }
                if focused_band_button(
                    ui,
                    crate::tr!("widget-delete"),
                    submit,
                    band.keyboard_focus && band.focus_pending,
                ) {
                    action = Some(BandAction::Remove);
                }
            });
        }
        BandContent::Spelling { issue } => {
            action = spelling_controls(
                ui,
                &issue.word,
                suggestions,
                submit,
                band.keyboard_focus && band.focus_pending,
            );
        }
    }
    if let Some(error) = &band.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    if apply {
        ui.horizontal(|ui| {
            if band_button(ui, crate::tr!("cancel"), submit) {
                action = Some(BandAction::Close);
            }
            if band_button(ui, crate::tr!("apply"), submit) {
                action = Some(BandAction::Apply);
            }
        });
    }
    action.or_else(|| {
        if !submit {
            return None;
        }
        match &band.content {
            BandContent::Image { .. } | BandContent::List { .. } | BandContent::Link { .. } => {
                Some(BandAction::Apply)
            }
            BandContent::Remove { .. } => Some(BandAction::Remove),
            BandContent::Spelling { .. } => suggestions
                .and_then(|s| s.first())
                .cloned()
                .map(BandAction::Correct),
        }
    })
}

fn spelling_controls(
    ui: &mut egui::Ui,
    word: &str,
    suggestions: Option<&[String]>,
    submit: bool,
    focus: bool,
) -> Option<BandAction> {
    let mut action = None;
    ui.label(word);
    if let Some(suggestions) = suggestions {
        ui.horizontal_wrapped(|ui| {
            if suggestions.is_empty() {
                ui.weak(crate::tr!("spelling-no-suggestions"));
            }
            for (index, suggestion) in suggestions.iter().enumerate() {
                if focused_band_button(ui, suggestion, submit, index == 0 && focus) {
                    action = Some(BandAction::Correct(suggestion.clone()));
                }
            }
        });
    } else {
        ui.weak(crate::tr!("spelling-loading-suggestions"));
    }
    ui.horizontal_wrapped(|ui| {
        for command in [Command::IgnoreSpelling, Command::LearnSpelling] {
            if band_button(ui, command.label(), submit) {
                action = Some(BandAction::Spelling(command));
            }
        }
    });
    action
}

fn band_button(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, submit: bool) -> bool {
    focused_band_button(ui, label, submit, false)
}
fn focused_band_button(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    submit: bool,
    focus: bool,
) -> bool {
    let response = ui.button(label);
    if focus {
        response.request_focus();
    }
    response.clicked() || (submit && response.has_focus())
}
