//! One read-only resolution shared by availability, labels and activation.
use super::*;
use crate::editor::{Widget, WidgetKind};

pub(super) enum ContextualAction {
    FocusBand(String),
    Correct(crate::engine::SpellingIssue),
    Settings(Widget),
}

impl ContextualAction {
    pub(super) fn label(&self) -> String {
        match self {
            Self::FocusBand(title) => crate::tr!("contextual-focus-band", title = title.as_str()),
            Self::Correct(issue) => crate::tr!("contextual-correct", word = issue.word.as_str()),
            Self::Settings(widget) => settings_label(widget.kind),
        }
    }
}

pub(super) fn settings_label(kind: WidgetKind) -> String {
    crate::i18n::text(match kind {
        WidgetKind::List => "widget-list-settings",
        WidgetKind::Image(_) => "widget-image-settings",
        _ => "widget-settings",
    })
}

impl App {
    pub(super) fn contextual_action(&self) -> Option<ContextualAction> {
        if !self.document.open || self.document.ime.active() || self.bands.parameter_ime {
            return None;
        }
        if let Some(band) = &self.bands.open {
            return Some(ContextualAction::FocusBand(band.title()));
        }
        if let Some(issue) = self.spelling_reference_at(self.document.selection.primary.index)
            && self.valid_spelling_target(&issue)
        {
            return Some(ContextualAction::Correct(issue));
        }
        let widget = self.current_widget()?;
        (widget.kind.configurable() && !self.document.engine.is_read_only())
            .then_some(ContextualAction::Settings(widget))
    }
    pub(super) fn activate_contextual_action(&mut self) {
        match self.contextual_action() {
            Some(ContextualAction::FocusBand(_)) => {}
            Some(ContextualAction::Correct(issue)) => self.open_spelling_reference(issue),
            Some(ContextualAction::Settings(widget)) => self.open_widget_band(widget, false),
            None => return,
        }
        if let Some(band) = &mut self.bands.open {
            band.focus_pending = true;
            band.keyboard_focus = true;
        }
    }
    pub(super) fn ui_command_label(&self, command: Command) -> String {
        if command == Command::ContextualAction
            && let Some(action) = self.contextual_action()
        {
            return action.label();
        }
        command.label()
    }
    pub(super) fn contextual_hint(&self, ctx: &egui::Context, action: &ContextualAction) -> String {
        let shortcut = self.keymap.shortcut_text(ctx, Command::ContextualAction);
        let label = action.label();
        if shortcut.is_empty() {
            label
        } else {
            crate::tr!("command-shortcut", label = label, shortcut = shortcut)
        }
    }
    pub(super) fn widget_settings_hint(
        &self,
        ctx: &egui::Context,
        widget: &Widget,
        action: Option<&ContextualAction>,
    ) -> String {
        if !widget.kind.configurable() {
            return crate::tr!("widget-direct-parameters");
        }
        if let Some(action @ ContextualAction::Settings(current)) = action
            && current.range == widget.range
        {
            return self.contextual_hint(ctx, action);
        }
        settings_label(widget.kind)
    }
    pub(super) fn contextual_spelling_hint(
        &self,
        ui: &egui::Ui,
        output: &egui::text_edit::TextEditOutput,
    ) {
        let Some(action) = self.contextual_action() else {
            return;
        };
        let ContextualAction::Correct(issue) = &action else {
            return;
        };
        let Ok((start, end)) = self.document.engine.resolve_target(issue.target) else {
            return;
        };
        let hint = self.contextual_hint(ui.ctx(), &action);
        let range = start..end;
        for (index, (_, rect)) in editor::range_rects(&output.galley, std::slice::from_ref(&range))
            .into_iter()
            .enumerate()
        {
            let rect = rect
                .translate(output.galley_pos.to_vec2())
                .intersect(ui.clip_rect())
                .intersect(output.text_clip_rect);
            if rect.is_positive() {
                ui.interact(
                    rect,
                    ui.id().with(("contextual-spelling-hint", start, index)),
                    egui::Sense::hover(),
                )
                .on_hover_text(&hint);
            }
        }
    }
}
