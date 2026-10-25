use super::*;

fn textures(
    ctx: &egui::Context,
    snapshot: &ProposalReview,
    caches: &mut [std::collections::HashMap<String, egui::TextureHandle>; 2],
) {
    for (side, assets) in [&snapshot.before.assets, &snapshot.after.assets]
        .into_iter()
        .enumerate()
    {
        let cache = &mut caches[side];
        for (path, data) in assets {
            if !cache.contains_key(path)
                && let Ok(image) =
                    crate::assets::thumbnail(data, path.rsplit('.').next().unwrap_or(""))
            {
                cache.insert(
                    path.clone(),
                    ctx.load_texture(
                        format!("review-{side}-{path}"),
                        egui::ColorImage::from_rgba_premultiplied(image.size, &image.rgba),
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
        }
    }
}
pub(super) fn comparison(
    ui: &mut egui::Ui,
    r: &mut ReviewState,
    style: crate::preferences::EditorStyle,
) {
    textures(ui.ctx(), &r.snapshot, &mut r.textures);
    ui.columns(2, |columns| {
        for (index, document, ranges) in [
            (0, &r.snapshot.original, &r.changes.removed),
            (1, &r.snapshot.candidate, &r.changes.added),
        ] {
            let ui = &mut columns[index];
            ui.strong(crate::i18n::text(if index == 1 {
                "review-proposed"
            } else if r.status == ProposalStatus::Stale {
                "review-base"
            } else {
                "review-current"
            }));
            ui.weak(crate::i18n::text(if index == 1 {
                "review-added"
            } else {
                "review-removed"
            }));
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt(("review-copy", index))
                .show(ui, |ui| {
                    let mut text = document.view.text.as_str();
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width| {
                        editor::layout(ui, &document.view, text.as_str(), width, style)
                    };
                    let background = ui.painter().add(egui::Shape::Noop);
                    let output = egui::TextEdit::multiline(&mut text)
                        .id_salt(("review-text", index))
                        .interactive(false)
                        .desired_width(f32::INFINITY)
                        .frame(false)
                        .layouter(&mut layouter)
                        .show(ui);
                    editor::paint_decorations(ui, &document.view, &output, style, background);
                    editor::paint_images(
                        ui,
                        &document.view,
                        &output,
                        &r.textures[index],
                        0..0,
                        style,
                    );
                    let color = if index == 0 {
                        ui.visuals().error_fg_color
                    } else {
                        if ui.visuals().dark_mode {
                            Color32::from_rgb(130, 200, 155)
                        } else {
                            Color32::from_rgb(32, 118, 66)
                        }
                    };
                    let painter = ui.painter_at(output.text_clip_rect);
                    for (_, rect) in editor::range_rects(&output.galley, ranges) {
                        let rect = rect.translate(output.galley_pos.to_vec2());
                        painter.rect_filled(rect, 1.0, color.gamma_multiply(0.12));
                        painter.hline(
                            rect.x_range(),
                            if index == 0 {
                                rect.center().y
                            } else {
                                rect.bottom() - 1.0
                            },
                            egui::Stroke::new(1.0_f32, color),
                        );
                    }
                });
        }
    });
}
pub(super) fn information(ui: &mut egui::Ui, r: &mut ReviewState) {
    textures(ui.ctx(), &r.snapshot, &mut r.textures);
    egui::ScrollArea::vertical()
        .id_salt("review-information-scroll")
        .show(ui, |ui| {
            ui.columns(3, |columns| {
                columns[0].strong(crate::tr!("review-field"));
                columns[1].strong(crate::i18n::text(if r.status == ProposalStatus::Stale {
                    "review-base"
                } else {
                    "review-current"
                }));
                columns[2].strong(crate::tr!("review-proposed"));
            });
            ui.separator();
            for row in &r.changes.rows {
                ui.columns(3, |columns| {
                    for (column, text) in
                        columns
                            .iter_mut()
                            .zip([&row.label, &row.before, &row.after])
                    {
                        column.add(
                            egui::Label::new(if text.is_empty() {
                                crate::tr!("review-empty")
                            } else {
                                text.clone()
                            })
                            .wrap(),
                        );
                    }
                });
                ui.separator();
            }
            for (number, path) in r.changes.resources.iter().enumerate() {
                ui.columns(3, |columns| {
                    let signature = [
                        r.snapshot.before.metadata.signature_path(),
                        r.snapshot.after.metadata.signature_path(),
                    ]
                    .contains(&Some(path.as_str()));
                    columns[0].label(if signature {
                        crate::tr!("review-signature")
                    } else {
                        crate::tr!("review-resource", name = (number + 1).to_string())
                    });
                    for (side, assets) in [&r.snapshot.before.assets, &r.snapshot.after.assets]
                        .into_iter()
                        .enumerate()
                    {
                        let column = &mut columns[side + 1];
                        if assets.contains_key(path) {
                            if let Some(texture) = r.textures[side].get(path) {
                                column.add(
                                    egui::Image::new(texture)
                                        .max_size(egui::vec2(column.available_width(), 160.0)),
                                );
                            } else {
                                column.label(crate::tr!("review-resource-present"));
                            }
                        } else {
                            column.weak(crate::tr!("review-empty"));
                        }
                    }
                });
                ui.separator();
            }
        });
}
