use super::*;

struct EditorFrame {
    before: crate::engine::NativeEdit,
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
                editor::ImageAction::Select => {
                    self.select_image_part(id, crate::engine::SelectionPart::Body)
                }
                editor::ImageAction::Replace => self.choose_image(Some(id)),
                editor::ImageAction::Information => self.open_image_info(id),
                editor::ImageAction::Remove => self.remove_image(id),
            }
        }
    }
    fn editor_frame(&mut self, ctx: &egui::Context) -> EditorFrame {
        if self.document.force_selection {
            let search_focus = self.search_focus(ctx);
            editor::restore_cursor(ctx, self.document.selection);
            if let Some(id) = search_focus {
                ctx.memory_mut(|memory| memory.request_focus(id));
            }
            self.document.force_selection = false;
        }
        // Une sélection imposée peut avoir focalisé l’éditeur depuis le début
        // de la frame. Revérifier la destination des événements restants.
        self.prepare_ime(ctx);
        let before = self.snapshot();
        let old_range = self.document.selection;
        for image in self.document.engine.document().view.images.values() {
            if !self.image_textures.contains_key(&image.path)
                && let Some(data) = self.document.engine.document().assets.get(&image.path)
                && let Ok(thumbnail) =
                    crate::assets::thumbnail(data, image.path.rsplit('.').next().unwrap())
            {
                self.image_textures.insert(
                    image.path.clone(),
                    ctx.load_texture(
                        &image.path,
                        egui::ColorImage::from_rgba_premultiplied(thumbnail.size, &thumbnail.rgba),
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
        }
        let view = self.document.ime.draft.as_ref().map_or_else(
            || self.document.engine.document().view.clone(),
            |draft| draft.input.view.clone(),
        );
        let lock_tab = view.annotation_at(self.range()).is_some()
            || (view.blocks[view.line_at(self.document.selection.primary.index)]
                == BlockKind::Description
                || view.blocks[view.line_at(self.document.selection.primary.index)]
                    .is_letter_note());
        let editor_style = self.preferences.editor;
        let matches = if self.search.open && self.document.ime.draft.is_none() {
            self.search.engine.matches.clone()
        } else {
            vec![]
        };
        let active = self.search.engine.selected(&self.range());
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
        let mut draft = crate::document::edits::TextInput {
            view: frame.view.clone(),
        };
        let output = egui::TextEdit::multiline(&mut editor::InputBuffer(&mut draft))
            .lock_focus(lock_tab)
            .id(egui::Id::new(editor::EDITOR_ID))
            .desired_width(f32::INFINITY)
            .desired_rows(25)
            .frame(false)
            .font(editor_style.font.font_id(editor_style.size, 0))
            .layouter(&mut layouter)
            .show(ui);
        self.editor_feedback(ui, output, frame, background, draft)
    }
    fn editor_feedback(
        &mut self,
        ui: &mut egui::Ui,
        output: egui::text_edit::TextEditOutput,
        frame: &EditorFrame,
        background: egui::layers::ShapeIdx,
        draft: crate::document::edits::TextInput,
    ) -> Option<(u64, editor::ImageAction)> {
        let matches = &frame.matches;
        let editor_style = frame.style;
        let before = &frame.before;
        let old_range = frame.old_range;
        let clicked_term =
            editor::paint_decorations(ui, &draft.view, &output, editor_style, background);
        self.scroll_ime(ui, &output);
        if self.document.ime.draft.is_some() {
            // La sélection native matérialise la préédition. La sélection
            // publiée, utilisée par les compteurs et l’aperçu, reste inchangée.
            let selection = self.document.ime.draft.as_ref().unwrap().selection;
            if !output.response.has_focus() {
                self.cancel_ime();
            }
            editor::paint_images(
                ui,
                &draft.view,
                &output,
                &self.image_textures,
                selection.as_sorted_char_range(),
                editor_style,
            );
            let mut state = output.state;
            state.clear_undoer();
            state.store(ui.ctx(), output.response.id);
            return None;
        }
        if let Some(range) = output.cursor_range {
            self.document.selection = range;
        }
        if self.search.open
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
            self.select_search_result(*index);
        }
        if let Some(at) = clicked_term {
            self.select_description_term(at);
        }
        self.editor_spelling(ui, &output, &frame.view);
        if self.preview.editor_scroll_pending {
            if self.search_focus(ui.ctx()).is_none() {
                output.response.request_focus();
            }
            let cursor = output
                .galley
                .pos_from_cursor(self.document.selection.primary);
            ui.scroll_to_rect(
                cursor.translate(output.galley_pos.to_vec2()),
                Some(egui::Align::Center),
            );
            self.preview.editor_scroll_pending = false;
        }
        let image_action = editor::paint_images(
            ui,
            &draft.view,
            &output,
            &self.image_textures,
            self.range(),
            editor_style,
        );
        let changed = output.response.changed() || draft.view.edit_error().is_some();
        let mut state = output.state;
        if self.document.force_selection {
            state.cursor.set_char_range(Some(self.document.selection));
        }
        state.clear_undoer();
        state.store(ui.ctx(), output.response.id);
        if changed {
            let result = self.document.engine.commit_native_edit(
                before.clone(),
                draft.view,
                editor::to_core(self.document.selection),
                crate::engine::NativeInput::Typing,
            );
            self.accept_edit(result, before.selection());
            if self.search.open {
                self.refresh_search();
            }
            ui.ctx().request_repaint();
        } else if self.document.selection != old_range {
            self.document
                .engine
                .update_adapter_cursor(editor::to_core(self.document.selection))
                .expect("native cursor in the published projection");
        }
        if self.document.selection != old_range {
            self.preview.forward_pending = true;
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
        if view.text == output.galley.text() {
            let editing = output.response.has_focus()
                && self.document.selection.is_empty()
                && self.document.engine.changed_at().elapsed() < Duration::from_millis(700);
            let ranges: Vec<_> = self
                .spelling
                .checker
                .issues
                .iter()
                .filter(|issue| {
                    !editing
                        || !(issue.range.start <= self.document.selection.primary.index
                            && self.document.selection.primary.index <= issue.range.end)
                })
                .map(|issue| issue.range.clone())
                .collect();
            editor::paint_spelling(ui, output, &ranges);
            self.spelling_context_menu(output);
            if editing {
                ui.ctx().request_repaint_after(
                    Duration::from_millis(700)
                        .saturating_sub(self.document.engine.changed_at().elapsed()),
                );
            }
        }
    }
    /// La proportion appartient aux préférences, les rectangles à la frame.
    /// Seuls les bords gardent une petite zone accessible pour chaque colonne.
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
