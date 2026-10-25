//! Whole-lot human review, separate from capability bands and agent context.
mod capture;
mod changes;
mod content;
mod preview;
use super::*;
use changes::Changes;
use typstographe_mcp::{ProposalReview, ProposalStatus};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Content,
    Information,
    Preview,
}
pub(super) struct ReviewState {
    pub snapshot: ProposalReview,
    pub status: ProposalStatus,
    pub open: bool,
    tab: Tab,
    changes: Changes,
    focus_pending: bool,
    preview: Option<preview::CandidatePreview>,
    textures: [std::collections::HashMap<String, egui::TextureHandle>; 2],
}
impl ReviewState {
    pub fn new(snapshot: ProposalReview) -> Self {
        let changes = Changes::new(&snapshot);
        Self {
            snapshot,
            changes,
            status: ProposalStatus::Pending,
            open: false,
            tab: Tab::Content,
            focus_pending: false,
            preview: None,
            textures: Default::default(),
        }
    }
}
impl App {
    pub(super) fn assistance_busy(&self) -> bool {
        self.dialogs.palette_open || self.assistance_busy_without_palette()
    }
    pub(super) fn assistance_busy_without_palette(&self) -> bool {
        self.document.ime.active()
            || self.assistance.native_ime
            || self.bands.parameter_ime
            || self.bands.open.is_some()
            || self.modal_without_palette()
            || self.file.export_task.is_some()
    }
    pub(super) fn review_open(&self) -> bool {
        self.assistance.review.as_ref().is_some_and(|r| r.open)
    }
    pub(super) fn review_command_enabled(&self, command: Command) -> Option<bool> {
        let r = self.assistance.review.as_ref();
        Some(match command {
            Command::ReviewProposal => {
                r.is_some_and(|r| {
                    !r.open && matches!(r.status, ProposalStatus::Pending | ProposalStatus::Stale)
                }) && self.document.open
                    && self
                        .assistance
                        .session
                        .as_ref()
                        .is_some_and(|s| !s.finished())
                    && !self.assistance_busy_without_palette()
            }
            Command::CloseReview => self.review_open(),
            Command::AcceptProposal | Command::RejectProposal => {
                self.review_open()
                    && r.is_some_and(|r| {
                        r.status == ProposalStatus::Pending
                            || (command == Command::RejectProposal
                                && r.status == ProposalStatus::Stale)
                    })
                    && self
                        .assistance
                        .session
                        .as_ref()
                        .is_some_and(|s| !s.finished())
                    && (command != Command::AcceptProposal
                        || r.is_some_and(|r| {
                            r.snapshot.guard == self.document.engine.target_guard()
                        }) && !self.document.engine.is_read_only())
                    && !self.assistance_busy_without_palette()
            }
            Command::RevealAgentChange => {
                self.assistance.notice.is_some()
                    && !self.review_open()
                    && !self.assistance_busy_without_palette()
            }
            _ => return None,
        })
    }
    pub(super) fn apply_review_command(&mut self, command: Command) {
        match command {
            Command::ReviewProposal => {
                if let Some(r) = &mut self.assistance.review {
                    r.open = true;
                    r.focus_pending = true;
                }
            }
            Command::CloseReview => self.leave_review(),
            Command::AcceptProposal | Command::RejectProposal => {
                if command == Command::RejectProposal
                    && self
                        .assistance
                        .review
                        .as_ref()
                        .is_some_and(|r| r.status == ProposalStatus::Stale)
                {
                    self.leave_review();
                    self.assistance.review = None;
                    return;
                }
                let busy = self.assistance_busy();
                let id = self
                    .assistance
                    .review
                    .as_ref()
                    .expect("review")
                    .snapshot
                    .id
                    .clone();
                let result = self
                    .assistance
                    .session
                    .as_mut()
                    .expect("invitation")
                    .decide(
                        &mut self.document.engine,
                        &id,
                        command == Command::AcceptProposal,
                        busy,
                    );
                match result {
                    Ok(outcome) => {
                        let changed = outcome.as_ref().is_some_and(|o| o.changed);
                        if let Some(outcome) = outcome {
                            self.apply_agent_outcome(&outcome);
                        }
                        self.dialogs.message =
                            Some(crate::i18n::text(if command == Command::AcceptProposal {
                                if changed {
                                    "review-accepted"
                                } else {
                                    "review-accepted-unchanged"
                                }
                            } else {
                                "review-rejected"
                            }));
                        self.leave_review();
                        self.assistance.review = None;
                    }
                    Err(error) => self.dialogs.message = Some(Self::command_error_text(&error)),
                }
            }
            Command::RevealAgentChange => {
                if let Some(target) = self.assistance.notice.take() {
                    if self.preview.sync.reveal_at(
                        &self.document.engine,
                        target,
                        RevealDestination::Both,
                    ) {
                        self.assistance.reveal_until =
                            Some(Instant::now() + Duration::from_secs(2));
                    } else {
                        self.dialogs.message = Some(crate::tr!("review-stale-help"));
                    }
                }
            }
            _ => {}
        }
    }
    fn leave_review(&mut self) {
        if let Some(r) = &mut self.assistance.review {
            r.open = false;
        }
        self.assistance.return_pending = true;
        self.document.force_selection = false;
    }
    pub(super) fn apply_agent_outcome(&mut self, outcome: &crate::engine::Outcome) {
        self.map_view_positions();
        self.document.selection = editor::to_editor(self.document.engine.selection());
        self.document.force_selection = false;
        self.preview.sync.apply(ViewEvent::CommandCompleted {
            result: outcome.into(),
            origin: ViewOrigin::Agent,
        });
        self.refresh_model_presentation();
        self.image_textures.clear();
        if self.search.open {
            self.refresh_search();
        }
        self.assistance.restore_visible = true;
        if let Some(r) = &self.assistance.review {
            self.assistance.notice = if let Some(range) = r.changes.added.first() {
                self.document
                    .engine
                    .selection_target(editor::to_core(editor::cursor(range.clone())))
                    .ok()
            } else if let Some(range) = r.changes.removed.first() {
                r.snapshot
                    .original
                    .structure
                    .selection_at(range.start, range.start)
                    .ok()
                    .and_then(|selection| {
                        self.document
                            .engine
                            .position_map()?
                            .map_retained(crate::engine::SelectionTarget {
                                guard: r.snapshot.guard,
                                selection,
                            })
                            .ok()
                    })
            } else {
                None
            };
        }
    }
    pub(super) fn track_assistance_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| !i.focused || i.pointer.any_pressed()) {
            self.assistance.native_ime = false;
        }
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    self.assistance.native_ime =
                        matches!(event, egui::ImeEvent::Enabled | egui::ImeEvent::Preedit(_));
                }
            }
        });
        if let Some(deadline) = self.assistance.reveal_until {
            if Instant::now() >= deadline {
                self.preview.sync.clear_reveal();
                self.assistance.reveal_until = None;
            } else {
                ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
            }
        }
        if self.assistance.return_pending {
            editor::store_cursor(ctx, self.document.selection);
            ctx.memory_mut(|m| {
                if let Some(id) = self.assistance.return_focus {
                    m.request_focus(id);
                } else if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            self.assistance.return_pending = false;
        }
        if !self.review_open() {
            self.assistance.return_focus = ctx.memory(|m| m.focused());
        }
    }
    pub(super) fn proposal_notification(&mut self, ui: &mut egui::Ui) {
        let status = self.assistance.review.as_ref().map(|r| r.status);
        if matches!(
            status,
            Some(ProposalStatus::Pending | ProposalStatus::Stale)
        ) {
            ui.horizontal_wrapped(|ui| {
                ui.label(crate::i18n::text(
                    if status == Some(ProposalStatus::Stale) {
                        "review-stale"
                    } else {
                        "review-pending"
                    },
                ));
                if ui
                    .add_enabled(
                        self.command_enabled(Command::ReviewProposal),
                        egui::Button::new(Command::ReviewProposal.label()),
                    )
                    .clicked()
                {
                    self.apply(Command::ReviewProposal);
                }
            });
            ui.separator();
        }
        if self.command_enabled(Command::RevealAgentChange)
            && ui.button(Command::RevealAgentChange.label()).clicked()
        {
            self.apply(Command::RevealAgentChange);
        }
    }
    pub(super) fn review_ui(&mut self, ui: &mut egui::Ui) {
        let mut r = self.assistance.review.take().expect("open review");
        let busy = self.assistance_busy();
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            ui.strong("typstOgraphe");
            ui.label(crate::tr!("review-title"));
            let back = ui.button(Command::CloseReview.label());
            if r.focus_pending {
                back.request_focus();
                r.focus_pending = false;
            }
            if back.clicked()
                || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                action = Some(Command::CloseReview);
            }
        });
        ui.separator();
        ui.label(crate::tr!(
            "review-count",
            passages = r.changes.removed.len().max(r.changes.added.len()) as i64,
            informations = r.changes.information_count() as i64
        ));
        if !r.snapshot.summary.is_empty() {
            ui.label(crate::tr!(
                "review-agent-summary",
                summary = r.snapshot.summary.clone()
            ));
        }
        if r.status == ProposalStatus::Stale {
            ui.colored_label(ui.visuals().warn_fg_color, crate::tr!("review-stale-help"));
        }
        if busy {
            ui.label(crate::tr!("review-busy"));
        }
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut r.tab, Tab::Content, crate::tr!("review-content"));
            if r.changes.information_count() > 0 {
                ui.selectable_value(
                    &mut r.tab,
                    Tab::Information,
                    crate::tr!("review-information"),
                );
            }
            ui.selectable_value(&mut r.tab, Tab::Preview, crate::tr!("review-preview"));
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.weak(crate::tr!("review-whole-lot"));
            let pending = r.status == ProposalStatus::Pending && !busy;
            if ui
                .add_enabled(
                    !busy && matches!(r.status, ProposalStatus::Pending | ProposalStatus::Stale),
                    egui::Button::new(Command::RejectProposal.label()),
                )
                .clicked()
            {
                action = Some(Command::RejectProposal);
            }
            if ui
                .add_enabled(pending, egui::Button::new(Command::AcceptProposal.label()))
                .clicked()
            {
                action = Some(Command::AcceptProposal);
            }
        });
        ui.separator();
        match r.tab {
            Tab::Content => content::comparison(ui, &mut r, self.preferences.editor),
            Tab::Information => content::information(ui, &mut r),
            Tab::Preview => preview::show(ui, &mut r),
        }
        self.assistance.review = Some(r);
        if let Some(action) = action {
            self.apply(action);
            ui.ctx().request_repaint();
        }
    }
}
