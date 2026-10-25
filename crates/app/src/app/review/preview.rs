use super::*;
pub(super) struct CandidatePreview {
    worker: RenderWorker,
    pages: Vec<egui::TextureHandle>,
    error: Option<String>,
    ready: bool,
}
impl CandidatePreview {
    pub(super) fn ready(&self) -> bool {
        self.ready || self.error.is_some()
    }
}
pub(super) fn show(ui: &mut egui::Ui, r: &mut ReviewState) {
    if r.preview.is_none() {
        let ctx = ui.ctx().clone();
        let worker = RenderWorker::spawn(std::sync::Arc::new(move || ctx.request_repaint()));
        let after = &r.snapshot.after;
        let request = crate::rendering::RenderRequest {
            revision: r.snapshot.guard.revision,
            source: after.source.clone(),
            source_map: r.snapshot.candidate.source_map.clone(),
            metadata: after.metadata.clone(),
            template: after.template.clone(),
            assets: after.assets.clone(),
            dependencies: after.dependencies.clone(),
            export_dir: None,
        };
        let error = worker
            .sender
            .send(crate::rendering::PreviewRequest {
                guard: r.snapshot.guard,
                request,
            })
            .err()
            .map(|_| crate::tr!("renderer-stopped"));
        r.preview = Some(CandidatePreview {
            worker,
            pages: vec![],
            error,
            ready: false,
        });
    }
    let preview = r.preview.as_mut().unwrap();
    while let Ok(response) = preview.worker.receiver.try_recv() {
        preview.ready = true;
        match response.result {
            Ok(rendered) => {
                preview.pages = rendered
                    .pages
                    .into_iter()
                    .enumerate()
                    .map(|(index, page)| {
                        ui.ctx().load_texture(
                            format!("proposal-page-{}-{index}", r.snapshot.id),
                            super::super::preview::page_image(
                                page,
                                ui.input(|i| i.max_texture_side),
                            ),
                            egui::TextureOptions::LINEAR,
                        )
                    })
                    .collect();
            }
            Err(error) => preview.error = Some(crate::i18n::error(&error)),
        }
    }
    if let Some(error) = &preview.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    if !preview.ready && preview.error.is_none() {
        ui.spinner();
        ui.label(crate::tr!("preview-composing"));
    }
    egui::ScrollArea::vertical()
        .id_salt("candidate-preview")
        .show(ui, |ui| {
            let width = (ui.available_width() - 20.0).max(1.0);
            for page in &preview.pages {
                ui.add(egui::Image::new(page).fit_to_exact_size(egui::vec2(
                    width,
                    width * page.size_vec2().y / page.size_vec2().x,
                )));
                ui.add_space(12.0);
            }
        });
}
