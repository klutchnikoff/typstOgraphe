#![warn(clippy::too_many_lines)]

mod assistance;
mod availability;
mod bands;
mod setup;
mod state;
#[cfg(test)]
use crate::document::Document;
use crate::i18n::LocalizedLabel;
use state::*;
mod chrome;
mod commands;
mod contextual;
mod dialogs;
mod editor_view;
mod engine;
mod export;
mod files;
mod images;
mod ime;
mod information;
mod information_fields;
mod links;
mod models;
mod preferences;
mod preview;
mod recovery;
mod review;
mod search;
mod signature;
mod spelling;
mod text_input;
mod view_coordination;
use crate::{
    commands::{FileCommand, Keymap, Registry, ShortcutOverrides, UiCommand as Command},
    document::{BlockKind, DocumentLanguage, PaperFormat, Projection},
    document_types::{DocumentType, ModelPresentation, models::Model},
    editor, menus,
    navigation::PagePoint,
    preferences::{Appearance, EditorFont, Preferences},
    rendering::RenderWorker,
    storage::{self, StoredDocument},
    view_sync::{RevealDestination, ViewAction, ViewEvent, ViewOrigin, ViewSync},
};
use eframe::egui::{
    self, Color32, RichText,
    text::{CCursor, CCursorRange},
};
use std::{
    ops::Range,
    path::PathBuf,
    time::{Duration, Instant},
};

const ACCENT: Color32 = Color32::from_rgb(40, 92, 119);
const PREVIEW_SELECTION: Color32 = Color32::from_rgba_premultiplied(16, 33, 59, 65);
const SEARCH_ID: &str = "document-search";
const REPLACE_ID: &str = "document-replacement";
#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
    Close,
    CloseDocument,
    Recover(usize),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum InformationGroup {
    #[default]
    General,
    Fields(&'static str),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PreferencesPage {
    Profile,
    #[default]
    Appearance,
    Spelling,
}

pub struct App {
    bands: bands::EditorBands,
    document: DocumentState,
    assistance: assistance::AssistanceState,
    dialogs: DialogState,
    file: FileState,
    preview: PreviewState,
    search: SearchState,
    spelling: SpellingState,
    recovery: RecoveryState,
    capture: CaptureState,
    preferences: Preferences,
    preferences_tracker: crate::preferences::SaveTracker,
    preferences_directory: Option<PathBuf>,
    profile_signature: Option<crate::assets::ImageData>,
    keymap: Keymap,
    #[cfg(target_os = "macos")]
    native_menus: Option<menus::native::NativeMenus>,
    image_textures: std::collections::HashMap<String, egui::TextureHandle>,
}

impl App {
    pub fn ui_frame(&mut self, ctx: &egui::Context) {
        self.track_assistance_input(ctx);
        if self.capture.path.is_some() {
            // Le pointeur réel ne doit pas modifier les survols des captures de référence.
            ctx.input_mut(|input| input.pointer = Default::default());
        }
        self.guard_ime(ctx);
        self.track_band_input(ctx);
        if ctx.input(|input| input.viewport().close_requested())
            && !self.file.allow_close
            && self.is_dirty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.dialogs.new_document_open = false;
            self.dialogs.preferences_open = false;
            self.dialogs.assistance_open = false;
            self.dialogs.palette_open = false;
            self.bands.open = None;
            self.file.export_open = false;
            self.dialogs.information_open = false;
            self.file.pending_action = Some(PendingAction::Close);
        }
        self.poll_export();
        self.rendering(ctx);
        self.refresh_spelling(ctx);
        if self.document.open && !self.modal_open() {
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            for file in dropped {
                if let Some(path) = file.path
                    && let Err(error) = self.import_image(&path, None)
                {
                    self.file.error = Some(error);
                    break;
                }
            }
        }
        self.track_history_focus(ctx);
        self.shortcuts(ctx);
        #[cfg(target_os = "macos")]
        if let Some(menus) = &self.native_menus {
            let commands = menus.drain();
            for command in commands {
                self.dispatch_action(command, ctx);
            }
        }
        self.prepare_ime(ctx);
        let modal = self.modal_open();
        let dirty = self.is_dirty();
        self.update_window_title(ctx, dirty);
        #[cfg(target_os = "macos")]
        let inline_menus = self.native_menus.is_none();
        #[cfg(not(target_os = "macos"))]
        let inline_menus = true;
        if inline_menus {
            egui::TopBottomPanel::top("menus").show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                if let Some(command) = menus::egui_bar(ui, &self.keymap, &self.menu_statuses()) {
                    self.dispatch_action(command, ctx);
                }
            });
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if modal {
                ui.disable();
            }
            if self.document.open {
                self.status_ui(ui);
            }
            if let Some(error) = &self.recovery.error {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    crate::tr!("recovery-unavailable"),
                )
                .on_hover_text(error);
            }
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                if self.document.open {
                    if self.review_open() {
                        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            self.review_ui(ui);
                        });
                    } else {
                        self.proposal_notification(ui);
                        self.columns_ui(ui, modal);
                    }
                } else {
                    self.welcome_ui(ui);
                }
            });
        self.preferences_ui(ctx);
        self.dialogs(ctx);
        if let Some(command) = self.file.command.take() {
            self.handle_file_command(command, ctx);
        }
        self.file_dialogs(ctx);
        self.recovery_dialog(ctx);
        self.checkpoint_recovery();
        self.refresh_spelling(ctx);
        if self.recovery.worker.is_some() {
            ctx.request_repaint_after(crate::recovery::INTERVAL);
        }
        self.track_history_focus(ctx);
        self.sync_published_selection();
        if self.poll_assistance() {
            ctx.request_repaint();
        }
        self.assistance_ui(ctx);
        #[cfg(target_os = "macos")]
        self.refresh_native_menus(ctx);
        self.capture(ctx);
    }
    fn update_window_title(&mut self, ctx: &egui::Context, dirty: bool) {
        let filename = self
            .document
            .engine
            .file_path()
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                self.recovery
                    .recovered_name
                    .as_ref()
                    .map(|name| crate::tr!("recovery-document-name", name = name.as_str()))
                    .unwrap_or_else(|| crate::tr!("untitled-letter"))
            });
        let title = if !self.document.open {
            "typstOgraphe".into()
        } else {
            crate::tr!(
                "window-title",
                name = filename.as_str(),
                marker = if dirty { " *" } else { "" }
            )
        };
        let title = if self.document.open && self.document.engine.is_read_only() {
            crate::tr!("file-read-only-title", name = title)
        } else {
            title
        };
        if title != self.file.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.file.window_title = title;
        }
    }
    fn capture(&mut self, ctx: &egui::Context) {
        if self.capture.path.is_none() {
            return;
        }
        if !self.review_capture_ready() {
            return;
        }
        if self.spelling.capture_active {
            if !self.spelling.checker.ready {
                return;
            }
            if self.spelling.target.is_none() {
                self.apply(Command::NextSpelling);
                self.apply(Command::SpellingSuggestions);
                ctx.request_repaint();
                return;
            }
            if self
                .spelling
                .checker
                .suggestions
                .as_ref()
                .is_none_or(|(_, values)| values.is_none())
            {
                return;
            }
        }
        if (!self.document.open || self.preview.revision == Some(self.document.engine.revision()))
            && !self.capture.screenshot_requested
            && self.capture.started.elapsed() > Duration::from_secs(1)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.capture.screenshot_requested = true;
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let (Some(image), Some(path)) = (image, self.capture.path.as_ref()) {
            let result = crate::write_screenshot(path, &image);
            if let Err(error) = result {
                eprintln!(
                    "{}",
                    crate::tr!("cli-capture-error", error = error.to_string())
                );
            }
            self.file.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
impl eframe::App for App {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.capture.path.is_some() {
            // Native diagnostics must not capture live keystrokes, IME, drops
            // or focus changes intended for another window. Keep backend images.
            input
                .events
                .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
            input.focused = true;
            input.modifiers = egui::Modifiers::NONE;
            input.hovered_files.clear();
            input.dropped_files.clear();
            for viewport in input.viewports.values_mut() {
                viewport.focused = Some(true);
            }
        }
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.capture.path.is_some() {
            return;
        }
        self.persist_preferences(storage, Instant::now(), true);
        self.checkpoint_recovery();
    }
    fn auto_save_interval(&self) -> Duration {
        // Récupération pilotée séparément par ui_frame ; préférences sur changement.
        Duration::MAX
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if !self.is_dirty() {
            self.clear_recovery();
        }
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let preferences_were_open = self.dialogs.preferences_open;
        self.ui_frame(ctx);
        if self.capture.path.is_none()
            && let Some(storage) = frame.storage_mut()
        {
            let force = (preferences_were_open && !self.dialogs.preferences_open)
                || ctx.input(|input| input.viewport().close_requested());
            self.persist_preferences(storage, Instant::now(), force);
            if let Some(delay) = self
                .preferences_tracker
                .observe(&self.preferences, Instant::now())
            {
                ctx.request_repaint_after(delay);
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
