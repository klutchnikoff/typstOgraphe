use super::*;

impl App {
    /// Adaptateur transitoire des cibles préparées par les familles non migrées.
    pub(super) fn focus_annotation_target(&mut self, range: Range<usize>) {
        self.document.selection = editor::cursor(range);
        if self.document.selection.is_empty() {
            self.document.selection.primary.prefer_next_row = true;
            self.document.selection.secondary.prefer_next_row = true;
        }
        self.document.force_selection = true;
        self.preview.editor_scroll_pending = true;
        self.preview.forward_pending = true;
        self.document.engine.typing_group = false;
    }
    fn structured_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        use crate::engine::{Command as Core, SelectTextParams, SelectionPart, TargetParams};
        if !ctx.input(|i| i.key_pressed(egui::Key::Tab) || i.key_pressed(egui::Key::Enter)) {
            return false;
        }
        let enabled = |command: &Core| {
            matches!(
                self.document.engine.available(command),
                crate::engine::Availability::Enabled
            )
        };
        let select = |part| {
            Core::SelectText(SelectTextParams {
                part: Some(part),
                ..Default::default()
            })
        };
        let part = if enabled(&select(SelectionPart::Annotation)) {
            Some(SelectionPart::Annotation)
        } else if enabled(&select(SelectionPart::Definition)) {
            Some(SelectionPart::Definition)
        } else {
            None
        };
        let exit = Core::ExitBlock(TargetParams::default());
        let split = Core::SplitBlock(TargetParams::default());
        let exit_enabled = enabled(&exit);
        let split_enabled = enabled(&split);
        let shift = ctx.input(|i| i.events.iter().any(|event| matches!(event,
            egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } if modifiers.shift
        )));
        if (part.is_some() || (exit_enabled && !shift))
            && ctx.input_mut(|i| {
                i.consume_key(
                    if shift {
                        egui::Modifiers::SHIFT
                    } else {
                        egui::Modifiers::NONE
                    },
                    egui::Key::Tab,
                )
            })
        {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            let command = if let Some(part) = part {
                select(if shift {
                    match part {
                        SelectionPart::Annotation => SelectionPart::Body,
                        _ => SelectionPart::Term,
                    }
                } else {
                    part
                })
            } else {
                exit
            };
            self.run_document_command(command);
            return true;
        }
        if (split_enabled || part.is_some())
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.run_document_command(split);
            return true;
        }
        false
    }
    pub(super) fn text_shortcuts(&mut self, ctx: &egui::Context) {
        if self.composing_input(ctx) {
            return;
        }
        self.sync_published_selection();
        if self.structured_shortcuts(ctx) || self.image_shortcut(ctx) {
            return;
        }
        self.typing_shortcut(ctx);
    }
    fn typing_shortcut(&mut self, ctx: &egui::Context) {
        use crate::engine::{Command as Core, InputRules, InsertTextParams, TextTarget};
        let Some(text) = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Text(text) = event {
                    Some(text.clone())
                } else {
                    None
                }
            })
        }) else {
            return;
        };
        let params = InsertTextParams {
            target: TextTarget::default(),
            text: text.clone(),
            input_rules: InputRules::Typing,
        };
        if self.document.engine.transforms_input(&params) {
            self.run_document_command(Core::InsertText(params));
            ctx.input_mut(|i| {
                i.events
                    .retain(|event| !matches!(event, egui::Event::Text(value) if value == &text))
            });
        }
    }
    fn image_shortcut(&mut self, ctx: &egui::Context) -> bool {
        let range = self.range();
        let line = self.document.engine.document.view.line_at(range.start);
        let start = self.document.engine.document.view.line_ranges()[line].start;
        if let BlockKind::Image(id) = self.document.engine.document.view.blocks[line] {
            let object_selected = range == (start..start + 1);
            let deleting = ctx
                .input(|i| i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete));
            if object_selected && deleting {
                ctx.input_mut(|i| {
                    i.events.retain(|e| {
                        !matches!(
                            e,
                            egui::Event::Key {
                                key: egui::Key::Backspace | egui::Key::Delete,
                                ..
                            }
                        )
                    })
                });
                self.remove_image(id);
                return true;
            }
            if range.is_empty()
                && range.start == start + 1
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
            {
                self.document.selection = editor::cursor(start..start + 1);
                self.document.force_selection = true;
                return true;
            }
        }
        false
    }
}
