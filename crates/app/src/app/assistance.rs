//! Explicit document-scoped grants. No listener is created at application startup.
use super::*;
use typstographe_mcp::local::LocalSession;

#[derive(Default)]
pub(super) struct AssistanceState {
    pub session: Option<LocalSession>,
    error: Option<String>,
    pub review: Option<super::review::ReviewState>,
    pub notice: Option<crate::engine::SelectionTarget>,
    pub return_focus: Option<egui::Id>,
    pub return_pending: bool,
    pub restore_visible: bool,
    pub visible_offset: f32,
    pub reveal_until: Option<Instant>,
    pub native_ime: bool,
}
impl App {
    /// Reproducible capture of the authorization UI, without creating a channel.
    pub fn capture_assistance(&mut self) {
        if !self.document.open {
            self.load_demo();
        }
        self.apply(Command::Assistance);
    }

    pub(super) fn poll_assistance(&mut self) -> bool {
        let before = (
            self.assistance.session.is_some(),
            self.assistance
                .review
                .as_ref()
                .map(|r| (r.snapshot.id.clone(), r.status)),
        );
        let busy = self.assistance_busy();
        let reviewing = self.review_open();
        let Some(session) = &mut self.assistance.session else {
            self.assistance.review = None;
            self.assistance.return_pending |= reviewing;
            return before.1.is_some();
        };
        if !session.poll_with_input(&mut self.document.engine, busy) || session.finished() {
            self.assistance.session = None;
            self.assistance.review = None;
            self.assistance.return_pending |= reviewing;
            self.dialogs.message = Some(crate::tr!("assistance-disconnected"));
        } else {
            if let Some(snapshot) = session.review()
                && self
                    .assistance
                    .review
                    .as_ref()
                    .is_none_or(|r| !r.open && r.snapshot.id != snapshot.id)
            {
                self.assistance.review = Some(super::review::ReviewState::new(snapshot.clone()));
            }
            if let Some(review) = &mut self.assistance.review {
                review.status = session
                    .proposal_status(&review.snapshot.id)
                    .unwrap_or(typstographe_mcp::ProposalStatus::Cancelled);
            }
            if self.assistance.review.as_ref().is_some_and(|r| {
                !r.open
                    && !matches!(
                        r.status,
                        typstographe_mcp::ProposalStatus::Pending
                            | typstographe_mcp::ProposalStatus::Stale
                    )
            }) {
                self.assistance.review = None;
            }
        }
        before
            != (
                self.assistance.session.is_some(),
                self.assistance
                    .review
                    .as_ref()
                    .map(|r| (r.snapshot.id.clone(), r.status)),
            )
    }
    pub(super) fn start_assistance(&mut self, ctx: &egui::Context) {
        let repaint = ctx.clone();
        match LocalSession::start(
            &mut self.document.engine,
            std::sync::Arc::new(move || repaint.request_repaint()),
        ) {
            Ok(session) => {
                self.assistance.session = Some(session);
                self.assistance.error = None;
            }
            Err(error) => {
                self.assistance.error =
                    Some(crate::tr!("assistance-error", error = error.to_string()))
            }
        }
    }
    pub(super) fn assistance_ui(&mut self, ctx: &egui::Context) {
        if !self.dialogs.assistance_open {
            return;
        }
        let response = egui::Modal::new(egui::Id::new("agent-assistance")).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.heading(crate::tr!("assistance-title"));
            ui.label(crate::tr!("assistance-help"));
            ui.add_space(8.0);
            ui.label(crate::tr!("assistance-read-only"));
            if let Some(session) = &self.assistance.session {
                ui.separator();
                ui.label(if session.connected() { crate::tr!("assistance-connected") }
                    else { crate::tr!("assistance-ready") });
                if ui.button(crate::tr!("assistance-copy")).clicked() {
                    let configuration = serde_json::json!({"mcpServers":{"typstographe":{
                        "command":"typstographe-cli", "args":["mcp","--attach",session.invitation_path()]
                    }}});
                    ctx.copy_text(serde_json::to_string_pretty(&configuration).expect("MCP configuration"));
                }
                if ui.button(crate::tr!("assistance-revoke")).clicked() { self.assistance.session = None; }
            } else if ui.add_enabled(self.document.open, egui::Button::new(crate::tr!("assistance-authorize"))).clicked() {
                self.start_assistance(ctx);
            }
            if let Some(error) = &self.assistance.error { ui.colored_label(ui.visuals().error_fg_color, error); }
            ui.add_space(8.0);
            if ui.button(crate::tr!("close")).clicked() { self.dialogs.assistance_open = false; }
        });
        if response.should_close() {
            self.dialogs.assistance_open = false;
        }
        if !self.dialogs.assistance_open {
            self.document.force_selection = true;
        }
    }
}
