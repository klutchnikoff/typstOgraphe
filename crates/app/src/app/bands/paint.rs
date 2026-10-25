use super::*;
use crate::{icons::Icon, ui_kit::ButtonKind};

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
            let extra = editor::band_extra(&output.galley, &widget, self.band_inset());
            let rect = editor::widget_rect_with(&output.galley, &widget, extra)
                .translate(output.galley_pos.to_vec2());
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
        let (rect, strip) = self.band_rect(ui, view, output, inset);
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
        let spelling = matches!(band.content, BandContent::Spelling { .. });
        paint_band_background(
            ui,
            rect,
            strip,
            inset,
            spelling.then(|| pointer_x(output, inset)),
        );
        let mut action = None;
        let mut measured = 0.0;
        // Enfant détaché : sa largeur ne doit pas élargir la colonne de l'éditeur, dont
        // dépend justement la largeur du bandeau.
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("editor-band")
                .max_rect(rect.shrink2(egui::vec2(12.0, 8.0))),
        );
        let mut paint = |ui: &mut egui::Ui| {
            ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            ui.set_min_width((rect.width() - 24.0).max(1.0));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(band.title()).family(crate::theme::semibold()));
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
        };
        paint(&mut child);
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
impl App {
    /// Rectangle du bandeau et style : bande pleine largeur dans le cadre d'un widget,
    /// carte alignée sur la colonne des blocs ailleurs.
    fn band_rect(
        &self,
        ui: &egui::Ui,
        view: &Projection,
        output: &egui::text_edit::TextEditOutput,
        inset: editor::BandInset,
    ) -> (egui::Rect, bool) {
        let anchor = inset.anchor.min(view.len());
        let mut cursor = CCursor::new(anchor);
        cursor.prefer_next_row = true;
        let mut y = output.galley.pos_from_cursor(cursor).top();
        // La vignette d'une image précède sa ligne : le bandeau se place avant elle.
        if matches!(view.blocks[view.line_at(anchor)], BlockKind::Image(_)) {
            y -= editor::IMAGE_HEIGHT + editor::IMAGE_CAPTION_GAP;
        }
        let host = editor::band_host(view, inset);
        let (left, right) = match &host {
            Some(widget) => {
                let extra = editor::band_extra(&output.galley, widget, Some(inset));
                let frame = editor::widget_rect_with(&output.galley, widget, extra)
                    .translate(output.galley_pos.to_vec2());
                (frame.left() + 1.0, frame.right() - 1.0)
            }
            None => (
                output.galley_pos.x,
                output.galley_pos.x + output.galley.job.wrap.max_width,
            ),
        };
        // Dans une colonne très étroite, le bandeau reste dans la zone visible.
        let right = right.min(ui.clip_rect().right() - 2.0).max(left + 120.0);
        let top = output.galley_pos.y + y - inset.height;
        (
            egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, top + inset.height)),
            host.is_some(),
        )
    }
}

/// Abscisse du milieu du mot visé, dans les coordonnées de l'écran.
fn pointer_x(output: &egui::text_edit::TextEditOutput, inset: editor::BandInset) -> f32 {
    let mut start = CCursor::new(inset.anchor);
    start.prefer_next_row = true;
    let first = output.galley.pos_from_cursor(start);
    let last = output.galley.pos_from_cursor(CCursor::new(inset.end));
    let middle = if (first.top() - last.top()).abs() < 1.0 {
        (first.left() + last.left()) / 2.0
    } else {
        first.left()
    };
    output.galley_pos.x + middle
}

fn paint_band_background(
    ui: &egui::Ui,
    rect: egui::Rect,
    strip: bool,
    inset: editor::BandInset,
    pointer: Option<f32>,
) {
    let t = crate::theme::Tokens::new(ui.visuals().dark_mode);
    let painter = ui.painter();
    let fill = if strip { t.accent_soft } else { t.panel };
    if strip {
        painter.rect_filled(rect, 0.0, fill);
        let line = egui::Stroke::new(
            1.0_f32,
            if inset.placement == editor::BandPlacement::AfterHeader {
                t.border
            } else {
                t.accent
            },
        );
        painter.hline(rect.x_range(), rect.top(), line);
        painter.hline(
            rect.x_range(),
            rect.bottom(),
            egui::Stroke::new(1.0_f32, t.accent),
        );
    } else {
        painter.rect_filled(rect, crate::theme::scale::RADIUS_CARD, fill);
        painter.rect_stroke(
            rect,
            crate::theme::scale::RADIUS_CARD,
            egui::Stroke::new(1.0_f32, t.accent),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(x) = pointer {
        // Pointe vers le mot : un triangle qui dépasse du bord bas du bandeau.
        let x = x.clamp(rect.left() + 18.0, rect.right() - 18.0);
        let bottom = rect.bottom();
        let tip = [
            egui::pos2(x - 6.0, bottom),
            egui::pos2(x + 6.0, bottom),
            egui::pos2(x, bottom + 6.0),
        ];
        painter.add(egui::Shape::convex_polygon(
            tip.to_vec(),
            fill,
            egui::Stroke::NONE,
        ));
        let accent = egui::Stroke::new(1.0_f32, t.accent);
        painter.line_segment([tip[0], tip[2]], accent);
        painter.line_segment([tip[2], tip[1]], accent);
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
            let focus = band.keyboard_focus && band.focus_pending;
            let labels = [crate::tr!("cancel"), crate::tr!("widget-delete")];
            band_footer(ui, &labels, |ui| {
                if kinded_band_button(ui, ButtonKind::Ghost, crate::tr!("cancel"), submit, false) {
                    action = Some(BandAction::Close);
                }
                if kinded_band_button(
                    ui,
                    ButtonKind::Danger,
                    crate::tr!("widget-delete"),
                    submit,
                    focus,
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
        let labels = [crate::tr!("cancel"), crate::tr!("apply")];
        band_footer(ui, &labels, |ui| {
            if kinded_band_button(ui, ButtonKind::Ghost, crate::tr!("cancel"), submit, false) {
                action = Some(BandAction::Close);
            }
            if kinded_band_button(ui, ButtonKind::Primary, crate::tr!("apply"), submit, false) {
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
        if suggestions.is_empty() {
            ui.weak(crate::tr!("spelling-no-suggestions"));
        }
        wrapped_buttons(ui, suggestions, |ui, index| {
            let suggestion = &suggestions[index];
            if focused_band_button(ui, suggestion, submit, index == 0 && focus) {
                action = Some(BandAction::Correct(suggestion.clone()));
            }
        });
    } else {
        ui.weak(crate::tr!("spelling-loading-suggestions"));
    }
    let commands = [Command::IgnoreSpelling, Command::LearnSpelling];
    let labels = commands.map(|command| command.label());
    wrapped_buttons(ui, &labels, |ui, index| {
        if band_button(ui, labels[index].clone(), submit) {
            action = Some(BandAction::Spelling(commands[index]));
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
    kinded_band_button(ui, ButtonKind::Secondary, label, submit, focus)
}
fn kinded_band_button(
    ui: &mut egui::Ui,
    kind: ButtonKind,
    label: impl Into<egui::WidgetText>,
    submit: bool,
    focus: bool,
) -> bool {
    let response = crate::ui_kit::button(ui, kind, label);
    if focus {
        response.request_focus();
    }
    response.clicked() || (submit && response.has_focus())
}
/// Largeur d'un bouton `ui_kit` : texte et marges de 14 points de chaque côté.
fn button_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let color = ui.visuals().text_color();
    ui.fonts_mut(|fonts| fonts.layout_no_wrap(label.to_owned(), font, color).size().x) + 28.0
}

/// Actions du bandeau alignées à droite, dans l'ordre naturel de tabulation : un espace
/// calculé sur la largeur des boutons précède la rangée.
fn band_footer(ui: &mut egui::Ui, labels: &[String], contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let widths: f32 = labels.iter().map(|label| button_width(ui, label)).sum();
        let gaps = ui.spacing().item_spacing.x * labels.len().saturating_sub(1) as f32;
        ui.add_space((ui.available_width() - widths - gaps).max(0.0));
        contents(ui);
    });
}

/// Rangées de boutons qui passent à la ligne par boutons entiers.
fn wrapped_buttons(
    ui: &mut egui::Ui,
    labels: &[String],
    mut button: impl FnMut(&mut egui::Ui, usize),
) {
    let (limit, gap) = (ui.available_width(), ui.spacing().item_spacing.x);
    let mut rows: Vec<Vec<usize>> = vec![vec![]];
    let mut used = 0.0;
    for (index, label) in labels.iter().enumerate() {
        let width = button_width(ui, label);
        if used > 0.0 && used + gap + width > limit {
            rows.push(vec![]);
            used = 0.0;
        }
        used += if used > 0.0 { gap + width } else { width };
        rows.last_mut().expect("row").push(index);
    }
    for row in rows {
        ui.horizontal(|ui| {
            for index in row {
                button(ui, index);
            }
        });
    }
}
