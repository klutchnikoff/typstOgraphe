//! Adaptateur IME de l’éditeur : egui garde le curseur et le rendu, la session
//! ne reçoit que les compositions validées. Aucun état privé d’egui n’est lu.
use super::*;
use crate::document::edits::TextInput;

#[derive(Default)]
pub(super) struct Ime {
    pub(super) draft: Option<Composition>,
    enabled: bool,
    // Ignorer la fin d’une composition annulée, même si la cible a changé.
    discarding: bool,
    reset_cursor: bool,
    scroll_pending: bool,
}

pub(super) struct Composition {
    guard: crate::session::TargetGuard,
    before: Snapshot,
    pub(super) input: TextInput,
    pub(super) selection: CCursorRange,
}

impl Composition {
    fn replace(&mut self, text: &str) {
        // Chaque prédiction remplace la sélection d’origine. Les ancres et le
        // style ne dépendent donc pas des prédictions intermédiaires.
        let error = self.input.view.edit_error;
        self.input.view = self.before.document.view.clone();
        self.input.view.edit_error = error;
        let range = self.before.selection.range();
        self.input.delete_char_range(range.clone());
        let inserted = self.input.insert_text(text, range.start);
        self.selection = editor::cursor(range.start..range.start + inserted);
    }
}

impl App {
    pub(super) fn cancel_ime(&mut self) {
        let ime = &mut self.document.ime;
        if ime.enabled || ime.draft.is_some() {
            ime.draft = None;
            ime.enabled = false;
            ime.discarding = true;
            ime.reset_cursor = true;
            ime.scroll_pending = true;
        }
    }

    /// À appeler avant les commandes : aucune touche de composition ne doit
    /// devenir un raccourci de transformation de texte.
    pub(super) fn guard_ime(&mut self, ctx: &egui::Context) {
        let invalid = self.document.ime.draft.as_ref().is_some_and(|draft| {
            draft.guard != self.document.engine.target_guard()
                || draft.before.selection != self.document.selection.into()
                || self.document.force_selection
        });
        let interrupted = ctx.input(|input| {
            !input.focused || input.pointer.any_pressed() || input.viewport().close_requested()
        });
        let focused = ctx.memory(|memory| memory.has_focus(egui::Id::new(editor::EDITOR_ID)));
        if invalid || interrupted || !focused || !self.document.open || self.modal_open() {
            self.cancel_ime();
        }
        if (self.document.ime.enabled || self.document.ime.draft.is_some())
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_ime();
        }
    }

    pub(super) fn composing_input(&self, ctx: &egui::Context) -> bool {
        self.document.ime.draft.is_some()
            || ctx.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Ime(_)))
            })
    }

    /// Consommer uniquement le flux IME appartenant à l’éditeur. Les champs de
    /// métadonnées et les dialogues continuent d’utiliser l’IME natif d’egui.
    pub(super) fn prepare_ime(&mut self, ctx: &egui::Context) {
        self.guard_ime(ctx);
        let focused = ctx.memory(|memory| memory.has_focus(egui::Id::new(editor::EDITOR_ID)))
            && ctx.input(|input| input.focused)
            && self.document.open
            && !self.modal_open();
        let events = ctx.input_mut(|input| std::mem::take(&mut input.events));
        let mut remaining = Vec::with_capacity(events.len());
        for event in events {
            if let egui::Event::Ime(ime) = event {
                if self.document.ime.discarding {
                    match ime {
                        egui::ImeEvent::Enabled => self.document.ime.discarding = false,
                        egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => {
                            self.document.ime.discarding = false;
                            continue;
                        }
                        egui::ImeEvent::Preedit(_) => continue,
                    }
                }
                if focused {
                    self.ime_event(ime);
                    ctx.request_repaint();
                } else {
                    remaining.push(egui::Event::Ime(ime));
                }
            } else if matches!(
                event,
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                }
            ) && (self.document.ime.enabled || self.document.ime.draft.is_some())
            {
                self.cancel_ime();
            } else if self.document.ime.draft.is_none() || !text_event(&event) {
                // Une saisie directe reprend la main après l’abandon du flux
                // IME ; une future touche morte peut n’émettre que Commit.
                if matches!(event, egui::Event::Text(_) | egui::Event::Paste(_)) {
                    self.document.ime.discarding = false;
                }
                remaining.push(event);
            }
        }
        ctx.input_mut(|input| input.events = remaining);
        self.restore_ime_cursor(ctx);
    }

    pub(super) fn restore_ime_cursor(&mut self, ctx: &egui::Context) {
        if self.document.ime.reset_cursor {
            editor::store_cursor(ctx, self.document.selection);
            self.document.ime.reset_cursor = false;
        }
        if let Some(draft) = &self.document.ime.draft {
            editor::store_cursor(ctx, draft.selection);
        }
    }

    pub(super) fn scroll_ime(
        &mut self,
        ui: &mut egui::Ui,
        output: &egui::text_edit::TextEditOutput,
    ) {
        if std::mem::take(&mut self.document.ime.scroll_pending)
            && output.response.has_focus()
            && let Some(selection) = output.cursor_range
        {
            let cursor = output.galley.pos_from_cursor(selection.primary);
            ui.scroll_to_rect(cursor.translate(output.galley_pos.to_vec2()), None);
        }
    }

    fn ime_event(&mut self, event: egui::ImeEvent) {
        match event {
            egui::ImeEvent::Enabled => self.document.ime.enabled = true,
            egui::ImeEvent::Preedit(text) => self.ime_replace(&text),
            egui::ImeEvent::Commit(text) => {
                if text.is_empty() || text == "\n" || text == "\r" {
                    self.cancel_ime();
                } else {
                    self.ime_replace(&text);
                    let draft = self
                        .document
                        .ime
                        .draft
                        .take()
                        .expect("composition préparée");
                    let end = draft.selection.as_sorted_char_range().end;
                    self.document.selection = editor::cursor(end..end);
                    let result = self.document.engine.commit_projection(
                        draft.guard,
                        draft.before.clone(),
                        draft.input.view,
                        self.document.selection.into(),
                        EditGroup::Discrete,
                    );
                    self.accept_edit(result, draft.before.selection);
                    self.document.ime.reset_cursor = true;
                    self.preview.forward_pending = true;
                    if self.search.open {
                        self.refresh_search();
                    }
                }
                self.document.ime.enabled = false;
                self.document.ime.discarding = false;
            }
            egui::ImeEvent::Disabled => {
                self.cancel_ime();
                self.document.ime.discarding = false;
            }
        }
    }

    fn ime_replace(&mut self, text: &str) {
        if self.document.ime.draft.is_none() {
            self.document.ime.draft = Some(Composition {
                guard: self.document.engine.target_guard(),
                before: self.snapshot(),
                input: TextInput {
                    view: self.document.engine.document.view.clone(),
                },
                selection: self.document.selection,
            });
        }
        self.document.ime.draft.as_mut().unwrap().replace(text);
        self.document.ime.scroll_pending = true;
    }
}

fn text_event(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::Text(_)
            | egui::Event::Paste(_)
            | egui::Event::Cut
            | egui::Event::Copy
            | egui::Event::Key { .. }
    )
}
