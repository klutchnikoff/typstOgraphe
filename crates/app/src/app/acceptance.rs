//! Trusted human-role driver for an isolated fixture; absent from ordinary builds.
//! These files are never MCP tools and must never be supplied to the test agent.
use super::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{io::Write, path::Path};

pub(super) struct State {
    root: PathBuf,
    sequence: u64,
    pending: Option<u64>,
    screenshot: Option<PathBuf>,
    requested: bool,
    error: Option<String>,
    pub frames: Vec<f64>,
    polls: Vec<(usize, f64)>,
    frame_started: Instant,
}
impl State {
    pub fn record_poll(&mut self, duration: Duration) {
        let ms = duration.as_secs_f64() * 1000.0;
        if ms > 1.0 {
            self.polls.push((self.frames.len(), ms));
        }
    }
}
#[derive(Deserialize)]
struct Request {
    sequence: u64,
    #[serde(flatten)]
    action: Action,
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Status,
    ResetFrames,
    Select { start: usize, end: usize },
    Text { text: String },
    Command { name: String },
    Ime { phase: String, text: String },
    Capture { name: String, dark: bool },
    Stop,
}
fn write_json(path: &Path, value: &Value) -> std::io::Result<()> {
    let mut file = atomic_write_file::AtomicWriteFile::options().open(path)?;
    file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    file.commit()
}
impl App {
    pub fn acceptance(
        cc: &eframe::CreationContext<'_>,
        root: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if !root.is_absolute() {
            return Err("diagnostic directory must be absolute".into());
        }
        std::fs::create_dir(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
        }
        let mut app = Self::new(cc, Some(root.join("unused.png")), None, None)?;
        let mut stored = StoredDocument::demo(DocumentType::Courrier);
        stored.source="Cheres collègues,\n\nJe voudrais savoir si vous avez pu regarder le document, envoyé hier, par mes soins, qui concerne la quantité de blé que la France pourrai produire l’année suivante.\n\nCette information pourra servir à être diffusée auprès des décideurs et a donc un charactère particulièrement importnt.\n\nJ’espère que vous pourrez me transmettre les informations demandées assez rapidement.".into();
        if let crate::document_types::Metadata::Courrier(letter) = &mut stored.metadata {
            letter.objet = "Production de blé — demande d’informations".into();
        }
        app.install_document(stored, None)
            .map_err(std::io::Error::other)?;
        std::fs::create_dir(root.join("exports"))?;
        if !app.save_path(root.join("courrier.typsto")) {
            return Err("cannot seed fixture".into());
        }
        app.start_recovery(root.join("recovery"), cc.egui_ctx.clone());
        app.assistance.save_current = true;
        app.assistance.export_root = Some(root.join("exports"));
        app.assistance.discovery_directory = Some(Ok(root.join("discovery")));
        app.start_assistance(&cc.egui_ctx);
        let session = app.assistance.session.as_ref().ok_or("no invitation")?;
        write_json(
            &root.join("ready.json"),
            &json!({"invitation":session.invitation_path(),"document":root.join("courrier.typsto"),"exports":root.join("exports"),"discovery_directory":root.join("discovery")}),
        )?;
        app.acceptance = Some(State {
            root,
            sequence: 0,
            pending: None,
            screenshot: None,
            requested: false,
            error: None,
            frames: vec![],
            polls: vec![],
            frame_started: Instant::now(),
        });
        app.document.force_selection = true;
        cc.egui_ctx
            .memory_mut(|m| m.request_focus(egui::Id::new(editor::EDITOR_ID)));
        Ok(app)
    }
    pub(super) fn acceptance_input(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let state = self.acceptance.as_ref().expect("diagnostic");
        if state.pending.is_some() {
            return;
        }
        let Ok(bytes) = std::fs::read(state.root.join("request.json")) else {
            return;
        };
        if bytes.len() > 16384 {
            return;
        }
        let Ok(request) = serde_json::from_slice::<Request>(&bytes) else {
            return;
        };
        if request.sequence <= state.sequence {
            return;
        }
        let mut state = self.acceptance.take().unwrap();
        state.sequence = request.sequence;
        state.pending = Some(request.sequence);
        state.error = None;
        let result = self.acceptance_action(ctx, input, request.action, &mut state);
        if let Err(error) = result {
            state.error = Some(error);
        }
        self.acceptance = Some(state);
    }
    fn acceptance_action(
        &mut self,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
        action: Action,
        state: &mut State,
    ) -> Result<(), String> {
        match action {
            Action::Status => {}
            Action::ResetFrames => {
                state.frames.clear();
                state.polls.clear();
            }
            Action::Select { start, end } => {
                if start.max(end) > self.document.engine.document().view.len() {
                    return Err("selection outside fixture".into());
                }
                self.document.selection = editor::cursor(start..end);
                self.document.force_selection = true;
                self.sync_published_selection();
                ctx.memory_mut(|m| m.request_focus(egui::Id::new(editor::EDITOR_ID)));
            }
            Action::Text { text } => input.events.push(egui::Event::Text(text)),
            Action::Ime { phase, text } => {
                let event = match phase.as_str() {
                    "enabled" => egui::ImeEvent::Enabled,
                    "preedit" => egui::ImeEvent::Preedit(text),
                    "commit" => egui::ImeEvent::Commit(text),
                    "disabled" => egui::ImeEvent::Disabled,
                    _ => return Err("unknown IME event".into()),
                };
                input.events.push(egui::Event::Ime(event));
            }
            Action::Command { name } => {
                let command = match name.as_str() {
                    "assistance" => Some(Command::Assistance),
                    "review" => Some(Command::ReviewProposal),
                    "share_selection" => Some(Command::ShareSelection),
                    "return" => Some(Command::CloseReview),
                    "accept" => Some(Command::AcceptProposal),
                    "reject" => Some(Command::RejectProposal),
                    "undo" => Some(Command::Undo),
                    "redo" => Some(Command::Redo),
                    "see" => Some(Command::RevealAgentChange),
                    "preferences" => Some(Command::Preferences),
                    "bold" => Some(Command::Bold),
                    "italic" => Some(Command::Italic),
                    "revoke" => {
                        self.assistance.session = None;
                        None
                    }
                    "close_dialog" => {
                        self.dialogs.preferences_open = false;
                        None
                    }
                    _ => return Err("unknown diagnostic UI command".into()),
                };
                if let Some(command) = command {
                    if !self.command_enabled(command) {
                        return Err(format!("UI command disabled: {name}"));
                    }
                    self.apply(command);
                }
            }
            Action::Capture { name, dark } => {
                if name.len() > 64
                    || !name.ends_with(".png")
                    || !name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
                    || name.contains("..")
                {
                    return Err("invalid screenshot basename".into());
                }
                self.preferences.appearance = if dark {
                    Appearance::Dark
                } else {
                    Appearance::Light
                };
                ctx.set_theme(self.preferences.appearance.theme());
                state.screenshot = Some(state.root.join(name));
                state.requested = false;
            }
            Action::Stop => {
                self.file.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        Ok(())
    }
    pub(super) fn acceptance_begin_frame(&mut self) {
        if let Some(state) = &mut self.acceptance {
            state.frame_started = Instant::now();
        }
    }
    pub(super) fn acceptance_after_frame(&mut self, ctx: &egui::Context) {
        let mut state = self.acceptance.take().expect("diagnostic");
        state
            .frames
            .push(state.frame_started.elapsed().as_secs_f64() * 1000.0);
        if state.screenshot.is_some() {
            if self.preview.revision != Some(self.document.engine.revision())
                || !self.review_capture_ready()
            {
                self.acceptance = Some(state);
                ctx.request_repaint_after(Duration::from_millis(16));
                return;
            }
            if !state.requested {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                state.requested = true;
                self.acceptance = Some(state);
                ctx.request_repaint_after(Duration::from_millis(16));
                return;
            }
            let image = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = image {
                if let Err(error) =
                    crate::write_screenshot(&state.screenshot.take().unwrap(), &image)
                {
                    state.error = Some(error.to_string());
                }
            } else {
                self.acceptance = Some(state);
                ctx.request_repaint_after(Duration::from_millis(16));
                return;
            }
        }
        if let Some(sequence) = state.pending.take() {
            let mut durations = state.frames.clone();
            durations.sort_by(f64::total_cmp);
            let percentile = |p: f64| {
                durations
                    .get(((durations.len().saturating_sub(1)) as f64 * p) as usize)
                    .copied()
                    .unwrap_or_default()
            };
            let value = json!({"sequence":sequence,"error":state.error,"guard":self.document.engine.target_guard(),
                "source":self.document.engine.document().source(),"text":self.document.engine.document().view.text,
                "primary":self.document.engine.primary_context().map(|s|json!({"selection":s.selection,"style":{"bold":s.typing_style.bold,"italic":s.typing_style.italic,"link":s.typing_style.link}})),
                "range":[self.range().start,self.range().end],"dirty":self.is_dirty(),"history":{"undo":self.document.engine.history().undo,"redo":self.document.engine.history().redo},
                "review":self.assistance.review.as_ref().map(|r|json!({"id":r.snapshot.id,"open":r.open,"status":r.status})),
                "agent":self.assistance.session.is_some(),"busy":self.assistance_busy(),"ime":self.document.ime.active(),
                "focus":ctx.memory(|m|m.focused().map(|id|format!("{id:?}"))),
                "mcp_poll_over_1ms":state.polls.iter().map(|(frame,ms)|json!({"frame":frame,"ms":ms})).collect::<Vec<_>>(),
                "frame_ms":{"count":durations.len(),"p50":percentile(0.5),"p95":percentile(0.95),"max":percentile(1.0),
                    "slow":state.frames.iter().enumerate().filter(|(_,ms)|**ms>50.0).map(|(frame,ms)|json!({"frame":frame,"ms":ms})).collect::<Vec<_>>()}});
            if let Err(error) = write_json(
                &state.root.join(format!("response-{sequence}.json")),
                &value,
            ) {
                eprintln!("diagnostic response: {error}");
            }
        }
        self.acceptance = Some(state);
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
