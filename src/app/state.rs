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
