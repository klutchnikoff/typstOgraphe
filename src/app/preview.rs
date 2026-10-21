use super::*;

impl App {
    pub(super) fn request(&self, export_dir: Option<PathBuf>) -> RenderRequest {
        RenderRequest {
            revision: self.session.revision,
            source: self.session.document.source().to_owned(),
            source_map: self.session.document.source_map.clone(),
            metadata: self.session.metadata.clone(),
            template: self.session.template.clone(),
            assets: self.session.document.assets.clone(),
            dependencies: self.session.dependencies.clone(),
            export_dir,
        }
    }
    pub(super) fn rendering(&mut self, ctx: &egui::Context) {
        if !self.document_open {
            while self.preview.worker.receiver.try_recv().is_ok() {}
            return;
        }
        while let Ok(response) = self.preview.worker.receiver.try_recv() {
            match response.result {
                Ok(rendered) => {
                    if response.is_export {
                        self.message = Some(crate::tr!(
                            "letter-exported",
                            path = rendered
                                .export_dir
                                .as_ref()
                                .map_or(String::new(), |p| p.display().to_string())
                        ));
                    }
                    if response.revision == self.session.revision {
                        self.preview.pages = rendered
                            .pages
                            .into_iter()
                            .enumerate()
                            .map(|(index, page)| {
                                ctx.load_texture(
                                    format!("typst-{}-{index}", response.revision),
                                    egui::ColorImage::from_rgba_premultiplied(
                                        page.size, &page.rgba,
                                    ),
                                    egui::TextureOptions::LINEAR,
                                )
                            })
                            .collect();
                        self.preview.elapsed = Some(rendered.elapsed);
                        self.presentation = rendered.presentation;
                        self.preview.error = None;
                        self.preview.revision = Some(response.revision);
                        self.preview.sync = Some(rendered.sync);
                        self.preview.forward_pending = true;
                    }
                }
                Err(error) => {
                    if response.revision == self.session.revision || response.is_export {
                        self.preview.error = Some(error);
                        self.preview.revision = Some(response.revision);
                    }
                }
            }
        }
        if self.preview.submitted != self.session.revision
            && self.session.changed_at.elapsed() > Duration::from_millis(220)
        {
            match self.preview.worker.sender.send(self.request(None)) {
                Ok(()) => self.preview.submitted = self.session.revision,
                Err(_) => self.preview.error = Some(crate::tr!("renderer-stopped")),
            }
        }
        if self.preview.revision != Some(self.session.revision) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    pub(super) fn preview_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::tr!("preview-title")).strong())
                .on_hover_text(crate::tr!("preview-hint"));
            if self.preview.revision != Some(self.session.revision) {
                ui.spinner();
                ui.weak(crate::tr!("preview-updating"));
            } else if let Some(time) = self.preview.elapsed {
                let time = if self.capture.path.is_some() {
                    Duration::ZERO
                } else {
                    time
                };
                ui.weak(crate::tr!(
                    "preview-stats",
                    pages = self.preview.pages.len(),
                    milliseconds = time.as_millis() as f64
                ));
            }
        });
        ui.separator();
        if let Some(error) = &self.preview.error {
            ui.colored_label(Color32::from_rgb(183, 69, 57), crate::tr!("preview-failed"));
            ui.collapsing(crate::tr!("error-details"), |ui| {
                ui.label(error);
            });
        }
        let current = self.sync_is_current();
        let highlights = if current {
            self.preview
                .sync
                .as_ref()
                .map(|sync| sync.selection_highlights(self.range()))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let target = if self.preview.forward_pending && current {
            self.preview.forward_pending = false;
            self.preview
                .sync
                .as_ref()
                .and_then(|sync| sync.forward(self.selection.primary.index))
        } else {
            None
        };
        let mut clicked = None;
        egui::ScrollArea::vertical()
            .id_salt("preview-scroll")
            .show(ui, |ui| {
                let width = (ui.available_width() - 24.0).max(1.0);
                if self.preview.pages.is_empty() {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.label(crate::tr!("preview-composing"));
                    });
                }
                for (index, page) in self.preview.pages.iter().enumerate() {
                    let size = page.size_vec2();
                    let displayed = egui::vec2(width, width * size.y / size.x);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        let response = ui.add(
                            egui::Image::new(page)
                                .fit_to_exact_size(displayed)
                                .sense(egui::Sense::click()),
                        );
                        if let Some(size) = self
                            .preview
                            .sync
                            .as_ref()
                            .and_then(|sync| sync.page_size(index))
                        {
                            let painter = ui
                                .painter()
                                .with_clip_rect(ui.clip_rect().intersect(response.rect));
                            for highlight in highlights
                                .iter()
                                .filter(|highlight| highlight.page == index)
                            {
                                let points = highlight
                                    .corners
                                    .iter()
                                    .map(|point| {
                                        response.rect.min
                                            + egui::vec2(
                                                point[0] / size[0] * response.rect.width(),
                                                point[1] / size[1] * response.rect.height(),
                                            )
                                    })
                                    .collect();
                                painter.add(egui::Shape::convex_polygon(
                                    points,
                                    PREVIEW_SELECTION,
                                    egui::Stroke::NONE,
                                ));
                            }
                            if let Some(target) = target.filter(|target| target.page == index) {
                                let point = response.rect.min
                                    + egui::vec2(
                                        target.point[0] / size[0] * displayed.x,
                                        target.point[1] / size[1] * displayed.y,
                                    );
                                let passage =
                                    egui::Rect::from_center_size(point, egui::vec2(8.0, 32.0));
                                if !ui.clip_rect().contains_rect(passage) {
                                    ui.scroll_to_rect(passage, Some(egui::Align::Center));
                                }
                            }
                            if current
                                && response.double_clicked()
                                && let Some(pointer) = response.interact_pointer_pos()
                            {
                                let relative = pointer - response.rect.min;
                                clicked = Some(PagePoint {
                                    page: index,
                                    point: [
                                        relative.x / displayed.x * size[0],
                                        relative.y / displayed.y * size[1],
                                    ],
                                });
                            }
                        }
                    });
                    ui.add_space(10.0);
                }
            });
        if let Some(location) = clicked {
            self.navigate_backward(location, ui.ctx());
        }
    }
    pub(super) fn sync_is_current(&self) -> bool {
        self.preview.error.is_none()
            && self
                .preview
                .sync
                .as_ref()
                .is_some_and(|sync| sync.revision == self.session.revision)
    }
    pub(super) fn navigate_backward(&mut self, location: PagePoint, ctx: &egui::Context) {
        if !self.sync_is_current() {
            return;
        }
        if let Some(character) = self
            .preview
            .sync
            .as_ref()
            .and_then(|sync| sync.backward(location))
        {
            self.selection = CCursorRange::one(CCursor::new(
                character.min(self.session.document.view.len()),
            ));
            self.session.document.view.typing_style = if character > 0 {
                self.session.document.view.style_at(character - 1)
            } else {
                InlineStyle::default()
            };
            self.force_selection = true;
            self.preview.editor_scroll_pending = true;
            self.preview.forward_pending = false;
            self.session.typing_group = false;
            ctx.request_repaint();
        }
    }
}
