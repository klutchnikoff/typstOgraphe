use super::*;

impl App {
    pub(super) fn rendering(&mut self, ctx: &egui::Context) {
        if !self.document.open {
            while self.preview.worker.receiver.try_recv().is_ok() {}
            return;
        }
        while let Ok(response) = self.preview.worker.receiver.try_recv() {
            match response.result {
                Ok(rendered) => {
                    if response.is_export {
                        self.dialogs.message = Some(crate::tr!(
                            "letter-exported",
                            path = rendered
                                .export_dir
                                .as_ref()
                                .map_or(String::new(), |p| p.display().to_string())
                        ));
                    }
                    if response.guard == self.document.engine.target_guard() {
                        let Ok(info) = self
                            .document
                            .engine
                            .install_render(response.guard, rendered.sync)
                        else {
                            continue;
                        };
                        self.preview.pages = rendered
                            .pages
                            .into_iter()
                            .enumerate()
                            .map(|(index, page)| {
                                ctx.load_texture(
                                    format!("typst-{}-{index}", response.guard.revision),
                                    page_image(page, ctx.input(|i| i.max_texture_side)),
                                    egui::TextureOptions::LINEAR,
                                )
                            })
                            .collect();
                        self.preview.elapsed = Some(rendered.elapsed);
                        self.document.presentation = rendered.presentation;
                        self.preview.error = None;
                        self.preview.revision = Some(response.guard.revision);
                        self.preview.render = Some(info);
                        self.preview.forward_pending = true;
                    }
                }
                Err(error) => {
                    if response.guard == self.document.engine.target_guard() || response.is_export {
                        self.preview.error = Some(crate::i18n::error(&error));
                        self.preview.revision = Some(response.guard.revision);
                    }
                }
            }
        }
        if self.preview.submitted != self.document.engine.revision()
            && self.document.engine.changed_at().elapsed() > Duration::from_millis(220)
        {
            let request = crate::rendering::PreviewRequest {
                guard: self.document.engine.target_guard(),
                request: self
                    .document
                    .engine
                    .preview_request()
                    .expect("open document"),
            };
            match self.preview.worker.sender.send(request) {
                Ok(()) => self.preview.submitted = self.document.engine.revision(),
                Err(_) => self.preview.error = Some(crate::tr!("renderer-stopped")),
            }
        }
        if self.preview.revision != Some(self.document.engine.revision()) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    pub(super) fn preview_ui(&mut self, ui: &mut egui::Ui) {
        self.preview_header(ui);
        let current = self.sync_is_current();
        let target = self
            .document
            .engine
            .selection_target(editor::to_core(self.document.selection))
            .ok();
        let highlights = if current {
            self.preview
                .render
                .as_ref()
                .zip(target)
                .and_then(|(render, target)| {
                    self.document
                        .engine
                        .selection_boxes(crate::engine::SelectionBoxesParams {
                            render: render.reference,
                            target: target.into(),
                        })
                        .ok()
                })
                .map(|result| result.boxes)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let target = if self.preview.forward_pending && current {
            self.preview.forward_pending = false;
            self.preview_location(self.document.selection.primary.index)
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
                            .render
                            .as_ref()
                            .and_then(|render| render.pages.get(index).copied())
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
    fn preview_header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::tr!("preview-title")).strong())
                .on_hover_text(crate::tr!("preview-hint"));
            if self.preview.revision != Some(self.document.engine.revision()) {
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
    }
    fn sync_is_current(&self) -> bool {
        self.preview.error.is_none()
            && self
                .preview
                .render
                .as_ref()
                .is_some_and(|render| render.reference.guard == self.document.engine.target_guard())
    }
    pub(super) fn preview_location(&self, character: usize) -> Option<PagePoint> {
        let render = self.preview.render.as_ref()?.reference;
        let caret = crate::engine::Caret {
            index: character,
            ..Default::default()
        };
        let target = self
            .document
            .engine
            .selection_target(crate::engine::Selection {
                primary: caret,
                secondary: caret,
            })
            .ok()?;
        self.document
            .engine
            .locate(crate::engine::LocateParams {
                render,
                position: crate::engine::RenderPosition::ToPreview {
                    target: target.into(),
                },
            })
            .ok()?
            .point
    }
    pub(super) fn navigate_backward(&mut self, location: PagePoint, ctx: &egui::Context) {
        if !self.sync_is_current() {
            return;
        }
        let Some(render) = self.preview.render.as_ref() else {
            return;
        };
        let target = self
            .document
            .engine
            .locate(crate::engine::LocateParams {
                render: render.reference,
                position: crate::engine::RenderPosition::ToSource { point: location },
            })
            .ok()
            .and_then(|result| result.target);
        if let Some(target) = target {
            if self.run_document_command(crate::engine::Command::SelectText(
                crate::engine::SelectTextParams {
                    target: target.into(),
                    part: None,
                },
            )) {
                self.preview.editor_scroll_pending = true;
                self.preview.forward_pending = false;
            }
            ctx.request_repaint();
        }
    }
}

/// Seule la texture d’affichage dépend du GPU ; le rendu Typst et ses coordonnées
/// restent à leur résolution d’origine. Une grande page ne doit pas faire paniquer egui.
pub(super) fn page_image(page: crate::rendering::PageImage, max_side: usize) -> egui::ColorImage {
    if page.size.into_iter().all(|side| side <= max_side) {
        return egui::ColorImage::from_rgba_premultiplied(page.size, &page.rgba);
    }
    use resvg::tiny_skia::{FilterQuality, Pixmap, PixmapPaint, PixmapRef, Transform};
    let scale = max_side as f64 / *page.size.iter().max().expect("two dimensions") as f64;
    let size = page
        .size
        .map(|side| (side as f64 * scale).floor().max(1.0) as u32);
    let source = PixmapRef::from_bytes(&page.rgba, page.size[0] as u32, page.size[1] as u32)
        .expect("valid Typst raster");
    let mut image = Pixmap::new(size[0], size[1]).expect("bounded preview raster");
    image.draw_pixmap(
        0,
        0,
        source,
        &PixmapPaint {
            quality: FilterQuality::Bicubic,
            ..Default::default()
        },
        Transform::from_scale(
            size[0] as f32 / page.size[0] as f32,
            size[1] as f32 / page.size[1] as f32,
        ),
        None,
    );
    egui::ColorImage::from_rgba_premultiplied(size.map(|side| side as usize), image.data())
}
