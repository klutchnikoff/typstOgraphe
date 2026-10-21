use super::*;

struct EditorFrame {
    before: Snapshot,
    old_range: CCursorRange,
    view: Projection,
    lock_tab: bool,
    style: crate::preferences::EditorStyle,
    matches: Vec<Range<usize>>,
    active: Option<usize>,
}

impl App {
    pub(super) fn editor_ui(&mut self, ui: &mut egui::Ui) {
        self.toolbar(ui);
        ui.separator();
        self.information_ui(ui);
        ui.separator();
        self.search_ui(ui);
        self.refresh_spelling(ui.ctx());
        let frame = self.editor_frame(ui.ctx());
        let mut image_action = None;
        egui::ScrollArea::vertical()
            .id_salt("editor-scroll")
            .show(ui, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        (frame.style.size * 22.0 / 18.0).round() as i8,
                        12,
                    ))
                    .show(ui, |ui| {
                        image_action = self.editor_text(ui, &frame);
                    });
            });
        if let Some((id, action)) = image_action {
            match action {
                editor::ImageAction::Select(at) => {
                    self.selection = editor::cursor(at..at + 1);
                    self.force_selection = true;
                    self.forward_pending = true;
                }
                editor::ImageAction::Replace => self.choose_image(Some(id)),
                editor::ImageAction::Information => {
                    self.image_alt
                        .clone_from(&self.session.document.view.images[&id].alt);
                    self.image_info = Some(id);
                }
                editor::ImageAction::Remove => self.remove_image(id),
            }
        }
    }
    fn editor_frame(&mut self, ctx: &egui::Context) -> EditorFrame {
        let before = self.snapshot();
        let old_range = self.selection;
        if self.force_selection {
            let search_focus = self.search_focus(ctx);
            editor::restore_cursor(ctx, self.selection);
            if let Some(id) = search_focus {
                ctx.memory_mut(|memory| memory.request_focus(id));
            }
            self.force_selection = false;
        }
        for image in self.session.document.view.images.values() {
            if !self.image_textures.contains_key(&image.path)
                && let Some(data) = self.session.document.assets.get(&image.path)
                && let Ok(thumbnail) =
                    crate::assets::thumbnail(data, image.path.rsplit('.').next().unwrap())
            {
                self.image_textures.insert(
                    image.path.clone(),
                    ctx.load_texture(&image.path, thumbnail, egui::TextureOptions::LINEAR),
                );
            }
        }
        let view = self.session.document.view.clone();
        let lock_tab = view.annotation_at(self.range()).is_some()
            || (view.blocks[view.line_at(self.selection.primary.index)] == BlockKind::Description
                || view.blocks[view.line_at(self.selection.primary.index)].is_letter_note());
        let editor_style = self.preferences.editor;
        let matches = if self.search_open {
            self.search.matches.clone()
        } else {
            vec![]
        };
        let active = self.search.selected(&self.range());
        EditorFrame {
            before,
            old_range,
            view,
            lock_tab,
            style: editor_style,
            matches,
            active,
        }
    }
    fn editor_text(
        &mut self,
        ui: &mut egui::Ui,
        frame: &EditorFrame,
    ) -> Option<(u64, editor::ImageAction)> {
        let view = &frame.view;
        let matches = &frame.matches;
        let active = frame.active;
        let editor_style = frame.style;
        let lock_tab = frame.lock_tab;
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
            if matches.is_empty() {
                editor::layout(ui, view, text.as_str(), width, editor_style)
            } else {
                editor::layout_with_search(
                    ui,
                    view,
                    text.as_str(),
                    width,
                    editor_style,
                    matches,
                    active,
                )
            }
        };
        let background = ui.painter().add(egui::Shape::Noop);
        let output = egui::TextEdit::multiline(&mut self.session.document.view)
            .lock_focus(lock_tab)
            .id(egui::Id::new(editor::EDITOR_ID))
            .desired_width(f32::INFINITY)
            .desired_rows(25)
            .frame(false)
            .font(editor_style.font.font_id(editor_style.size, 0))
            .layouter(&mut layouter)
            .show(ui);
        self.editor_feedback(ui, output, frame, background)
    }
    fn editor_feedback(
        &mut self,
        ui: &mut egui::Ui,
        output: egui::text_edit::TextEditOutput,
        frame: &EditorFrame,
        background: egui::layers::ShapeIdx,
    ) -> Option<(u64, editor::ImageAction)> {
        let matches = &frame.matches;
        let editor_style = frame.style;
        let before = &frame.before;
        let old_range = frame.old_range;
        let clicked_term = editor::paint_decorations(
            ui,
            &self.session.document.view,
            &output,
            editor_style,
            background,
        );
        if let Some(range) = output.cursor_range {
            self.selection = range;
        }
        if self.search_open
            && output.response.clicked()
            && let Some(pointer) = output.response.interact_pointer_pos()
            && let Some((index, _)) =
                editor::range_rects(&output.galley, matches)
                    .iter()
                    .find(|(_, rect)| {
                        rect.translate(output.galley_pos.to_vec2())
                            .contains(pointer)
                    })
        {
            self.selection = editor::cursor(matches[*index].clone());
            self.force_selection = true;
        }
        if let Some(at) = clicked_term {
            self.selection = editor::cursor(at..at);
            self.force_selection = true;
        }
        self.editor_spelling(ui, &output, &frame.view);
        if self.editor_scroll_pending {
            if self.search_focus(ui.ctx()).is_none() {
                output.response.request_focus();
            }
            let cursor = output.galley.pos_from_cursor(self.selection.primary);
            ui.scroll_to_rect(
                cursor.translate(output.galley_pos.to_vec2()),
                Some(egui::Align::Center),
            );
            self.editor_scroll_pending = false;
        }
        let image_action = editor::paint_images(
            ui,
            &self.session.document.view,
            &output,
            &self.image_textures,
            self.range(),
            editor_style,
        );
        let changed = output.response.changed();
        let mut state = output.state;
        if self.force_selection {
            state.cursor.set_char_range(Some(self.selection));
        }
        state.clear_undoer();
        state.store(ui.ctx(), output.response.id);
        if changed {
            self.remember(before.clone(), true);
            if self.search_open {
                self.refresh_search();
            }
            ui.ctx().request_repaint();
        } else if self.selection != old_range {
            let selected = self.range();
            let pos = self.selection.primary.index;
            self.session.document.view.typing_style = if !selected.is_empty() {
                self.session.document.view.style_at(selected.start)
            } else if pos > 0 {
                self.session.document.view.style_at(pos - 1)
            } else {
                InlineStyle::default()
            };
            self.session.typing_group = false;
        }
        if self.selection != old_range {
            self.forward_pending = true;
            ui.ctx().request_repaint();
        }
        image_action
    }
    fn editor_spelling(
        &mut self,
        ui: &mut egui::Ui,
        output: &egui::text_edit::TextEditOutput,
        view: &Projection,
    ) {
        if view.text == self.session.document.view.text {
            let editing = output.response.has_focus()
                && self.selection.is_empty()
                && self.session.changed_at.elapsed() < Duration::from_millis(700);
            let ranges: Vec<_> = self
                .spelling
                .issues
                .iter()
                .filter(|issue| {
                    !editing
                        || !(issue.range.start <= self.selection.primary.index
                            && self.selection.primary.index <= issue.range.end)
                })
                .map(|issue| issue.range.clone())
                .collect();
            editor::paint_spelling(ui, output, &ranges);
            self.spelling_context_menu(output);
            if editing {
                ui.ctx().request_repaint_after(
                    Duration::from_millis(700).saturating_sub(self.session.changed_at.elapsed()),
                );
            }
        }
    }
    pub(super) fn columns_ui(&mut self, ui: &mut egui::Ui, disabled: bool) -> [egui::Rect; 2] {
        let bounds = ui.available_rect_before_wrap();
        let divider_width = 8.0;
        let usable = (bounds.width() - divider_width).max(1.0);
        let minimum = 48.0_f32.min(usable * 0.25);
        let mut left_width =
            (usable * self.preferences.editor_fraction).clamp(minimum, usable - minimum);
        let divider = egui::Rect::from_min_size(
            egui::pos2(bounds.left() + left_width, bounds.top()),
            egui::vec2(divider_width, bounds.height()),
        );
        let response = ui.interact(
            divider,
            ui.id().with("column-divider"),
            if disabled {
                egui::Sense::hover()
            } else {
                egui::Sense::click_and_drag()
            },
        );
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            left_width =
                (pointer.x - bounds.left() - divider_width / 2.0).clamp(minimum, usable - minimum);
            self.preferences.editor_fraction = left_width / usable;
        } else if response.double_clicked() {
            self.preferences.editor_fraction = 0.5;
            left_width = usable * 0.5;
        }
        let editor_rect = egui::Rect::from_min_max(
            bounds.min,
            egui::pos2(bounds.left() + left_width, bounds.bottom()),
        );
        let preview_rect = egui::Rect::from_min_max(
            egui::pos2(editor_rect.right() + divider_width, bounds.top()),
            bounds.max,
        );
        for (id, rect, preview) in [
            ("preview-column", preview_rect, true),
            ("editor-column", editor_rect, false),
        ] {
            ui.scope_builder(egui::UiBuilder::new().id_salt(id).max_rect(rect), |ui| {
                ui.set_clip_rect(rect);
                if disabled {
                    ui.disable();
                }
                egui::Frame::central_panel(ui.style()).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    if preview {
                        self.preview_ui(ui);
                    } else {
                        self.editor_ui(ui);
                    }
                });
            });
        }
        let stroke = if response.hovered() || response.dragged() {
            ui.visuals().widgets.hovered.fg_stroke
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };
        ui.painter().vline(
            editor_rect.right() + divider_width / 2.0,
            bounds.y_range(),
            stroke,
        );
        response
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
            .on_hover_text(crate::tr!("column-divider-hint"));
        [editor_rect, preview_rect]
    }
}
