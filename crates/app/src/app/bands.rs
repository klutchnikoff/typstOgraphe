//! One guarded, nonmodal interaction; no draft fields enter the published text.
mod actions;
mod capture;
mod paint;
use super::*;
use crate::editor::{Widget, WidgetKind};
use crate::engine::{BlockTarget, SelectionTarget};

pub(super) enum BandContent {
    Image {
        reference: BlockTarget,
        alt: String,
    },
    List {
        kind: BlockKind,
    },
    Link {
        url: String,
    },
    Remove {
        kind: WidgetKind,
        image: Option<BlockTarget>,
    },
    Spelling {
        issue: crate::engine::SpellingIssue,
    },
}
pub(super) struct Band {
    pub target: SelectionTarget,
    pub content: BandContent,
    return_selection: SelectionTarget,
    return_focus: Option<egui::Id>,
    pub error: Option<String>,
    pub height: f32,
    pub focus_pending: bool,
}
#[derive(Default)]
pub(super) struct EditorBands {
    pub open: Option<Band>,
    pub last_focus: Option<egui::Id>,
    pub hit_regions: Vec<egui::Rect>,
    pub pointer_capture: bool,
    pub field_focused: bool,
    pub parameter_ime: bool,
}
pub(super) enum BandAction {
    Close,
    Apply,
    Remove,
    ReplaceImage,
    Correct(String),
    Spelling(Command),
}
pub(super) enum EditorAction {
    Image(u64, editor::ImageAction),
    Settings(Widget),
    Remove(Widget),
    Band(BandAction),
}
impl Band {
    pub fn title(&self) -> String {
        match &self.content {
            BandContent::Image { .. } => crate::tr!("widget-image-settings"),
            BandContent::List { .. } => crate::tr!("widget-list-settings"),
            BandContent::Link { .. } => crate::tr!("link-title"),
            BandContent::Remove { kind, .. } => {
                crate::tr!("widget-delete-title", kind = kind.label())
            }
            BandContent::Spelling { .. } => crate::tr!("widget-spelling"),
        }
    }
}
impl App {
    pub(super) fn track_band_input(&mut self, ctx: &egui::Context) {
        self.bands.field_focused = self.bands.open.is_some()
            && ctx.memory(|m| m.has_focus(egui::Id::new("editor-band-input")));
        if self.bands.open.is_none() {
            self.bands.parameter_ime = false;
            self.bands.last_focus = ctx.memory(|m| m.focused());
        } else if self.bands.field_focused || self.bands.parameter_ime {
            ctx.input(|input| {
                for event in &input.events {
                    if let egui::Event::Ime(ime) = event {
                        self.bands.parameter_ime =
                            matches!(ime, egui::ImeEvent::Enabled | egui::ImeEvent::Preedit(_));
                    }
                }
            });
        }
    }
    pub(super) fn show_band(&mut self, target: SelectionTarget, content: BandContent) {
        if self.document.ime.draft.is_some() || self.bands.parameter_ime {
            return;
        }
        let return_selection = self
            .document
            .engine
            .selection_target(editor::to_core(self.document.selection))
            .expect("published selection");
        let return_focus = self
            .bands
            .open
            .as_ref()
            .and_then(|b| b.return_focus)
            .or(self.bands.last_focus);
        self.bands.open = Some(Band {
            target,
            content,
            return_selection,
            return_focus,
            error: None,
            height: 150.0,
            focus_pending: true,
        });
        self.bands.parameter_ime = false;
        self.bands.field_focused = false;
        self.document.history_field = None;
    }
    pub(super) fn close_band(&mut self, ctx: &egui::Context) {
        let Some(band) = self.bands.open.take() else {
            return;
        };
        self.bands.parameter_ime = false;
        self.bands.field_focused = false;
        let retained = if band.return_selection.guard == self.document.engine.target_guard() {
            Some(band.return_selection)
        } else {
            self.document
                .engine
                .position_map()
                .and_then(|p| p.map_retained(band.return_selection).ok())
        };
        if let Some((start, end)) =
            retained.and_then(|t| self.document.engine.resolve_target(t).ok())
        {
            self.document.selection = editor::cursor(start..end);
            let _ = self
                .document
                .engine
                .set_adapter_selection(editor::to_core(self.document.selection));
            editor::store_cursor(ctx, self.document.selection);
        }
        if let Some(id) = band.return_focus {
            ctx.memory_mut(|m| m.request_focus(id));
        } else {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new(editor::EDITOR_ID)));
        }
        self.document.force_selection = false;
        self.view_selection_changed(ViewOrigin::User);
    }
    pub(super) fn band_inset(&self) -> Option<editor::BandInset> {
        let band = self.bands.open.as_ref()?;
        let target = if band.target.guard == self.document.engine.target_guard() {
            band.target
        } else {
            self.document
                .engine
                .position_map()
                .and_then(|p| p.map_retained(band.target).ok())
                .unwrap_or(band.target)
        };
        let start = self
            .document
            .engine
            .resolve_target(target)
            .map(|(start, _)| start)
            .unwrap_or(0);
        Some(editor::BandInset {
            anchor: start,
            height: band.height,
        })
    }
    pub(super) fn dispatch_editor_action(&mut self, action: EditorAction, ctx: &egui::Context) {
        match action {
            EditorAction::Image(id, action) => match action {
                editor::ImageAction::Select => {
                    self.select_image_part(id, crate::engine::SelectionPart::Body)
                }
                editor::ImageAction::Replace => self.choose_image(Some(id)),
                editor::ImageAction::Information => self.open_image_info(id),
                editor::ImageAction::Remove => self.remove_image(id),
            },
            EditorAction::Settings(widget) => self.open_widget_band(widget, false),
            EditorAction::Remove(widget) => self.open_widget_band(widget, true),
            EditorAction::Band(action) => self.apply_band_action(action, ctx),
        }
    }
    fn open_widget_band(&mut self, widget: Widget, remove: bool) {
        let Ok(target) = self
            .document
            .engine
            .selection_target(editor::to_core(editor::cursor(widget.range.clone())))
        else {
            return;
        };
        let content = if remove {
            BandContent::Remove {
                kind: widget.kind,
                image: match widget.kind {
                    WidgetKind::Image(id) => self.document.engine.image_target(id),
                    _ => None,
                },
            }
        } else {
            match widget.kind {
                WidgetKind::Image(id) => {
                    self.open_image_info(id);
                    return;
                }
                WidgetKind::List => BandContent::List {
                    kind: self.document.engine.document().view.blocks[widget.lines.start],
                },
                _ => return,
            }
        };
        self.show_band(target, content);
    }
}
