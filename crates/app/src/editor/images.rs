//! images : détail interne de l’adaptateur egui.
use super::*;
pub(super) fn image_frame(galley: &Galley, range: Range<usize>) -> egui::Rect {
    let top =
        galley.pos_from_cursor(CCursor::new(range.start)).top() - IMAGE_HEIGHT - IMAGE_CAPTION_GAP;
    let mut end = CCursor::new(range.end);
    end.prefer_next_row = range.len() == 1;
    let bottom = galley.pos_from_cursor(end).bottom() + IMAGE_PADDING;
    egui::Rect::from_min_max(
        egui::pos2(0.0, top),
        egui::pos2(galley.job.wrap.max_width, bottom),
    )
}

#[derive(Clone, Copy)]
pub enum ImageAction {
    Select,
    Replace,
    Information,
    Remove,
}

pub fn paint_images(
    ui: &mut egui::Ui,
    view: &Projection,
    output: &egui::text_edit::TextEditOutput,
    textures: &std::collections::HashMap<String, egui::TextureHandle>,
    _selection: Range<usize>,
    _editor: EditorStyle,
) -> Option<(u64, ImageAction)> {
    let mut action = None;
    let painter = ui.painter_at(output.text_clip_rect);
    for (line, range) in view.line_ranges().iter().enumerate() {
        let BlockKind::Image(id) = view.blocks[line] else {
            continue;
        };
        let Some(image) = view.images.get(&id) else {
            continue;
        };
        let rect =
            image_frame(&output.galley, range.clone()).translate(output.galley_pos.to_vec2());
        let image_area =
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), IMAGE_HEIGHT));
        if !rect.intersects(output.text_clip_rect) {
            continue;
        }
        let inset = rect.width() * 0.1;
        painter.hline(
            rect.left() + inset..=rect.right() - inset,
            image_area.bottom() - 2.0,
            Stroke::new(
                0.75_f32,
                ui.visuals().weak_text_color().gamma_multiply(0.45),
            ),
        );
        if let Some(texture) = textures.get(&image.path) {
            let available = egui::Rect::from_min_max(
                image_area.min + egui::vec2(IMAGE_PADDING, 10.0),
                image_area.max - egui::vec2(IMAGE_PADDING, 14.0),
            );
            let native = texture.size_vec2();
            let factor = (available.width() / native.x).min(available.height() / native.y);
            let picture = egui::Rect::from_center_size(available.center(), native * factor);
            painter.image(
                texture.id(),
                picture,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let response = ui.interact(
            image_area.intersect(output.text_clip_rect),
            ui.id().with(("image", id)),
            // Réserver aussi le début d’un glissement : sinon le TextEdit situé
            // dessous déplace le curseur dès l’appui, avant le clic des boutons.
            egui::Sense::click_and_drag(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                true,
                if image.alt.is_empty() {
                    crate::tr!("image-label")
                } else {
                    image.alt.clone()
                },
            )
        });
        if response.clicked() {
            action = Some((id, ImageAction::Select));
        }
    }
    action
}
