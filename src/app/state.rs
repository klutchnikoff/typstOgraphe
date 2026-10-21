use super::*;

pub(super) struct CaptureState {
    pub(super) path: Option<PathBuf>,
    pub(super) screenshot_requested: bool,
    pub(super) started: Instant,
}

pub(super) struct RecoveryState {
    pub(super) worker: Option<crate::recovery::Recovery>,
    pub(super) entries: Vec<crate::recovery::Entry>,
    pub(super) open: bool,
    pub(super) error: Option<String>,
    pub(super) recovered_name: Option<String>,
}

pub(super) struct SpellingState {
    pub(super) checker: crate::spelling::Checker,
    pub(super) target: Option<(u64, crate::spelling::Issue)>,
    pub(super) menu_pending: bool,
    pub(super) menu_position: Option<egui::Pos2>,
    pub(super) capture_active: bool,
}

pub(super) struct SearchState {
    pub(super) engine: crate::search::Search,
    pub(super) open: bool,
    pub(super) replace_open: bool,
    pub(super) focus_pending: bool,
}
