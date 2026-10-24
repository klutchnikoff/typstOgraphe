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
    selection: Range<usize>,
    editor: EditorStyle,
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
        let selected = selection.start <= range.end && selection.end >= range.start;
        painter.rect_stroke(
            rect,
            5.0,
            Stroke::new(
                if selected { 1.5_f32 } else { 0.75 },
                if selected {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().widgets.noninteractive.bg_stroke.color
                },
            ),
            egui::StrokeKind::Inside,
        );
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
                image_area.min + egui::vec2(IMAGE_PADDING, 34.0),
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
        if selected {
            let mut x = rect.left() + 7.0;
            for (label, kind, width) in [
                (crate::tr!("image-replace"), ImageAction::Replace, 90.0),
                (
                    crate::tr!("image-information"),
                    ImageAction::Information,
                    112.0,
                ),
                (crate::tr!("image-remove"), ImageAction::Remove, 90.0),
            ] {
                let button = egui::Rect::from_min_size(
                    egui::pos2(x, rect.top() + 5.0),
                    egui::vec2(width, 24.0),
                );
                if button.intersects(output.text_clip_rect)
                    && ui.put(button, egui::Button::new(label).small()).clicked()
                {
                    action = Some((id, kind));
                }
                x += width + 5.0;
            }
        } else {
            painter.text(
                rect.min + egui::vec2(10.0, 8.0),
                egui::Align2::LEFT_TOP,
                crate::tr!("image-label"),
                editor.font.font_id(editor.size * 0.8, 0),
                ui.visuals().weak_text_color(),
            );
        }
    }
    action
}
