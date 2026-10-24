//! Champs egui locaux ; seules les commandes du moteur publient leurs valeurs.
use super::*;
use crate::{
    document_types::{FieldKind, FieldValue},
    engine::{Command as Core, MetadataInput, SetMetadataParams},
};

impl App {
    pub(super) fn information_fields_ui(&mut self, ui: &mut egui::Ui, group: &str) {
        let fields = self.document.engine.metadata().field_values();
        let width = (ui.available_width() - 120.0).max(80.0);
        egui::Grid::new(format!("{group}-fields"))
            .num_columns(2)
            .show(ui, |ui| {
                for field in fields.iter().filter(|f| f.group == group) {
                    self.information_field_ui(ui, field, width);
                    ui.end_row();
                }
            });
    }
    fn information_field_ui(&mut self, ui: &mut egui::Ui, field: &FieldValue, width: f32) {
        let guard = self.document.engine.target_guard();
        let label = ui.label(crate::i18n::text(field.label_key));
        let mut value = field.value.clone();
        let is_date = field.kind == FieldKind::Date;
        let edit = |ui: &mut egui::Ui, value: &mut String, width| {
            let widget = if field.multiline {
                egui::TextEdit::multiline(value).desired_rows(3)
            } else {
                egui::TextEdit::singleline(value)
            };
            let widget = widget.id_salt(field.editor_id).desired_width(width);
            if is_date {
                widget.hint_text(crate::tr!("date-input-hint"))
            } else {
                widget
            }
            .show(ui)
        };
        let mut today = false;
        let mut output = if is_date {
            ui.horizontal_wrapped(|ui| {
                let output = edit(ui, &mut value, (width - 100.0).max(80.0));
                today = ui
                    .button(crate::tr!("date-today"))
                    .on_hover_text(crate::tr!("date-today-help"))
                    .clicked();
                output
            })
            .inner
        } else {
            edit(ui, &mut value, width)
        };
        let resolve_today = is_date && (today || crate::dates::is_today(&value));
        if today {
            value = crate::dates::today();
        } else if resolve_today {
            crate::dates::normalize(&mut value);
        }
        if output.response.changed() || today || (is_date && output.response.lost_focus()) {
            let input = if output.response.has_focus() && !today {
                MetadataInput::Typing
            } else {
                MetadataInput::Discrete
            };
            self.run_document_command(Core::SetMetadata(SetMetadataParams {
                guard,
                field: field.field,
                value: value.clone(),
                input,
            }));
            let published = self
                .document
                .engine
                .metadata()
                .field_values()
                .into_iter()
                .find(|f| f.field == field.field)
                .unwrap()
                .value;
            if resolve_today || published != value {
                output
                    .state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(published.chars().count()),
                    )));
            }
        }
        let response = output.response.labelled_by(label.id);
        output.state.clear_undoer();
        output.state.store(ui.ctx(), response.id);
    }
}
