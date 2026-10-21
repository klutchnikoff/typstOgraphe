use super::*;

pub(super) struct CaptureState {
    pub(super) path: Option<PathBuf>,
    pub(super) screenshot_requested: bool,
    pub(super) started: Instant,
}
