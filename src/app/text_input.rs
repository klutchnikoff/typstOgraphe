use super::*;

struct ShortcutInput {
    range: Range<usize>,
    line: usize,
    lines: Vec<Range<usize>>,
    start: usize,
    prefix: String,
    typed: Option<String>,
}

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
    fn annotation_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        use crate::engine::{AnnotationPart, Command as Core, SelectTextParams, TargetParams};
        self.sync_published_selection();
        let select = Core::SelectText(SelectTextParams {
            part: Some(AnnotationPart::Annotation),
            ..Default::default()
        });
        if !matches!(
            self.document.engine.available(&select),
            crate::engine::Availability::Enabled
        ) {
            return false;
        }
        let shift = ctx.input(|i| i.events.iter().any(|event| matches!(event,
            egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } if modifiers.shift
        )));
        let tab = ctx.input_mut(|i| {
            i.consume_key(
                if shift {
                    egui::Modifiers::SHIFT
                } else {
                    egui::Modifiers::NONE
                },
                egui::Key::Tab,
            )
        });
        if tab {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            self.run_document_command(Core::SelectText(SelectTextParams {
                part: Some(if shift {
                    AnnotationPart::Body
                } else {
                    AnnotationPart::Annotation
                }),
                ..Default::default()
            }));
            return true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            self.run_document_command(Core::SplitBlock(TargetParams::default()));
            return true;
        }
        false
    }
    pub(super) fn text_shortcuts(&mut self, ctx: &egui::Context) {
        if self.composing_input(ctx) {
            return;
        }
        if self.annotation_shortcuts(ctx) {
            return;
        }
        let input = self.shortcut_input(ctx);
        if self.image_shortcut(ctx, &input) {
            return;
        }
        if self.block_shortcut(ctx, &input) {
            return;
        }
        if self.letter_note_shortcut(ctx, &input) {
            return;
        }
        if self.description_shortcut(ctx, &input) {
            return;
        }
        if !input.range.is_empty() {
            return;
        }
        self.inline_shortcut(ctx, &input);
        self.list_shortcut(ctx, &input);
    }
    fn shortcut_input(&self, ctx: &egui::Context) -> ShortcutInput {
        let range = self.range();
        let line = self.document.engine.document.view.line_at(range.start);
        let lines = self.document.engine.document.view.line_ranges();
        let start = lines[line].start;
        let prefix: String = self
            .document
            .engine
            .document
            .view
            .text
            .chars()
            .skip(start)
            .take(range.start - start)
            .collect();
        let typed = ctx.input(|i| {
            if !range.is_empty() {
                return None;
            }
            i.events.iter().find_map(|event| {
                if let egui::Event::Text(text) = event {
                    Some(text.clone())
                } else {
                    None
                }
            })
        });
        ShortcutInput {
            range,
            line,
            lines,
            start,
            prefix,
            typed,
        }
    }
    fn image_shortcut(&mut self, ctx: &egui::Context, input: &ShortcutInput) -> bool {
        let range = input.range.clone();
        let line = input.line;
        let start = input.start;
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
    fn block_shortcut(&mut self, ctx: &egui::Context, input: &ShortcutInput) -> bool {
        let range = input.range.clone();
        let start = input.start;
        let prefix = &input.prefix;
        let typed = &input.typed;
        if typed.as_deref() == Some(" ") {
            let kind = match prefix.as_str() {
                "=" => Some(BlockKind::Heading(1)),
                "==" => Some(BlockKind::Heading(2)),
                "===" => Some(BlockKind::Heading(3)),
                "-" => Some(BlockKind::Bullet),
                "+" | "1." => Some(BlockKind::Numbered),
                "/" => Some(BlockKind::Description),
                ">" => Some(BlockKind::Quote),
                _ => None,
            };
            if let Some(kind) = kind {
                let mut before = self.snapshot();
                before.document.view.insert_text(" ", range.start);
                before.document.sync_source();
                before.selection = editor::cursor(range.start + 1..range.start + 1).into();
                self.document
                    .engine
                    .document
                    .view
                    .delete_char_range(start..range.start);
                self.document
                    .engine
                    .document
                    .view
                    .set_block(start..start, kind);
                self.document.selection = editor::cursor(start..start);
                self.document.force_selection = true;
                self.remember(before, false);
                ctx.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e,egui::Event::Text(t) if t==" "))
                });
                return true;
            }
        }
        false
    }
    fn letter_note_shortcut(&mut self, ctx: &egui::Context, input: &ShortcutInput) -> bool {
        let range = input.range.clone();
        let line = input.line;
        let lines = &input.lines;
        let start = input.start;
        if self.document.engine.document.view.blocks[line].is_letter_note() {
            if range.is_empty()
                && lines[line].is_empty()
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                let before = self.snapshot();
                self.document.engine.document.view.blocks[line] = BlockKind::Paragraph;
                self.remember(before, false);
                self.focus_annotation_target(start..start);
                return true;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                let before = self.snapshot();
                let (_, group) = self
                    .document
                    .engine
                    .document
                    .view
                    .letter_note_groups()
                    .into_iter()
                    .find(|(_, lines)| lines.contains(&line))
                    .expect("bloc de courrier");
                let at = lines[group.end - 1].end;
                self.document.engine.document.view.insert_text("\n", at);
                let next = self.document.engine.document.view.line_at(at + 1);
                self.document.engine.document.view.blocks[next] = BlockKind::Paragraph;
                self.remember(before, false);
                self.focus_annotation_target(at + 1..at + 1);
                return true;
            }
        }
        false
    }
    fn description_shortcut(&mut self, ctx: &egui::Context, input: &ShortcutInput) -> bool {
        let range = input.range.clone();
        let line = input.line;
        let lines = &input.lines;
        let start = input.start;
        let prefix = &input.prefix;
        let typed = &input.typed;
        if self.document.engine.document.view.blocks[line] == BlockKind::Description {
            let text = &self.document.engine.document.view.text[self
                .document
                .engine
                .document
                .view
                .byte_index(lines[line].start)
                ..self
                    .document
                    .engine
                    .document
                    .view
                    .byte_index(lines[line].end)];
            let separator = text.chars().position(|c| c == '\t').map(|at| start + at);
            let shift = ctx.input(|i| i.events.iter().any(|event| matches!(event,
                egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } if modifiers.shift
            )));
            if ctx.input_mut(|i| {
                i.consume_key(
                    if shift {
                        egui::Modifiers::SHIFT
                    } else {
                        egui::Modifiers::NONE
                    },
                    egui::Key::Tab,
                )
            }) {
                // egui prépare la navigation du focus au début de la frame,
                // avant que l’éditeur ne consomme la touche.
                ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                if shift {
                    self.document.selection = editor::cursor(start..start);
                } else if let Some(at) = separator {
                    self.document.selection = editor::cursor(at + 1..at + 1);
                } else {
                    let before = self.snapshot();
                    self.document
                        .engine
                        .document
                        .view
                        .insert_text("\t", range.start);
                    self.document.selection = editor::cursor(range.start + 1..range.start + 1);
                    self.remember(before, false);
                }
                self.document.force_selection = true;
                return true;
            }
            if (separator.is_none() || separator == Some(range.start))
                && prefix.ends_with(':')
                && typed.as_deref() == Some(" ")
            {
                let mut before = self.snapshot();
                before.document.view.insert_text(" ", range.start);
                before.document.sync_source();
                before.selection = editor::cursor(range.start + 1..range.start + 1).into();
                self.document
                    .engine
                    .document
                    .view
                    .delete_char_range(range.start - 1..range.start);
                if separator.is_none() {
                    self.document
                        .engine
                        .document
                        .view
                        .insert_text("\t", range.start - 1);
                }
                self.document.selection = editor::cursor(range.start..range.start);
                self.document.force_selection = true;
                self.remember(before, false);
                ctx.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e, egui::Event::Text(t) if t == " "))
                });
                return true;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                let before = self.snapshot();
                if text.trim().is_empty() && range.is_empty() {
                    self.document
                        .engine
                        .document
                        .view
                        .delete_char_range(lines[line].clone());
                    self.document
                        .engine
                        .document
                        .view
                        .set_block(start..start, BlockKind::Paragraph);
                    self.document.selection = editor::cursor(start..start);
                } else {
                    let in_text = separator.is_some_and(|at| range.start > at);
                    self.document
                        .engine
                        .document
                        .view
                        .delete_char_range(range.clone());
                    self.document
                        .engine
                        .document
                        .view
                        .insert_text(if in_text { "\n\t" } else { "\n" }, range.start);
                    self.document.selection = editor::cursor(range.start + 1..range.start + 1);
                }
                self.remember(before, false);
                self.document.force_selection = true;
                return true;
            }
        }
        false
    }
    fn inline_shortcut(&mut self, ctx: &egui::Context, input: &ShortcutInput) {
        let range = input.range.clone();
        let start = input.start;
        let prefix = &input.prefix;
        let typed = &input.typed;
        if typed.as_deref() == Some("*") || typed.as_deref() == Some("_") {
            let delimiter = typed.as_ref().and_then(|s| s.chars().next()).unwrap_or('*');
            let chars: Vec<_> = prefix.chars().collect();
            if let Some(position) = chars.iter().rposition(|c| *c == delimiter)
                && position + 1 < chars.len()
                && (position == 0 || chars[position - 1].is_whitespace())
            {
                let mut before = self.snapshot();
                before
                    .document
                    .view
                    .insert_text(&delimiter.to_string(), range.start);
                before.document.sync_source();
                before.selection = editor::cursor(range.start + 1..range.start + 1).into();
                self.document
                    .engine
                    .document
                    .view
                    .delete_char_range(start + position..start + position + 1);
                let mark = if delimiter == '*' {
                    Mark::Bold
                } else {
                    Mark::Italic
                };
                let selected = start + position..range.start - 1;
                if !self
                    .document
                    .engine
                    .document
                    .view
                    .mark_state(selected.clone(), mark)
                    .0
                {
                    self.document
                        .engine
                        .document
                        .view
                        .toggle_mark(selected, mark);
                }
                self.document.engine.document.view.typing_style = InlineStyle::default();
                self.document.selection = editor::cursor(range.start - 1..range.start - 1);
                self.document.force_selection = true;
                self.remember(before, false);
                ctx.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e,egui::Event::Text(t) if t==&delimiter.to_string()))
                });
            }
        }
    }
    fn list_shortcut(&mut self, ctx: &egui::Context, input: &ShortcutInput) {
        let line = input.line;
        let lines = &input.lines;
        if lines[line].is_empty()
            && matches!(
                self.document.engine.document.view.blocks[line],
                BlockKind::Bullet | BlockKind::Numbered | BlockKind::Description
            )
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.apply(Command::Block(BlockKind::Paragraph));
        }
    }
}
