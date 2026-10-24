//! Explicit document-scoped grants. No listener is created at application startup.
use super::*;
use typstographe_mcp::local::LocalSession;

#[derive(Default)]
pub(super) struct AssistanceState {
    pub session: Option<LocalSession>,
    error: Option<String>,
}
impl App {
    /// Reproducible capture of the authorization UI, without creating a channel.
    pub fn capture_assistance(&mut self) {
        if !self.document.open {
            self.load_demo();
        }
        self.apply(Command::Assistance);
    }

    pub(super) fn poll_assistance(&mut self) {
        let Some(session) = &mut self.assistance.session else {
            return;
        };
        if !session.poll(&mut self.document.engine) || session.finished() {
            self.assistance.session = None;
            self.dialogs.message = Some(crate::tr!("assistance-disconnected"));
        }
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
