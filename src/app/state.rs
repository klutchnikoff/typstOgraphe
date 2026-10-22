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

pub(super) struct PreviewState {
    pub(super) worker: RenderWorker,
    pub(super) pages: Vec<egui::TextureHandle>,
    pub(super) submitted: u64,
    pub(super) elapsed: Option<Duration>,
    pub(super) error: Option<String>,
    pub(super) revision: Option<u64>,
    pub(super) sync: Option<RenderSync>,
    pub(super) forward_pending: bool,
    pub(super) editor_scroll_pending: bool,
}

pub(super) struct FileState {
    pub(super) command: Option<FileCommand>,
    pub(super) pending_action: Option<PendingAction>,
    pub(super) error: Option<String>,
    pub(super) allow_close: bool,
    pub(super) window_title: String,
    pub(super) export_open: bool,
    pub(super) export_path: String,
}

pub(super) struct DialogState {
    pub(super) preferences_page: PreferencesPage,
    pub(super) preferences_open: bool,
    pub(super) new_document_open: bool,
    pub(super) new_document_type: DocumentType,
    pub(super) new_document_model: Option<Model>,
    pub(super) information_open: bool,
    pub(super) information_group: InformationGroup,
    pub(super) palette_open: bool,
    pub(super) palette_query: String,
    pub(super) palette_index: usize,
    pub(super) image_info: Option<u64>,
    pub(super) image_alt: String,
    pub(super) link_open: bool,
    pub(super) link_url: String,
    pub(super) link_error: Option<String>,
    pub(super) link_range: Range<usize>,
    pub(super) message: Option<String>,
    pub(super) profile_error: Option<String>,
}

// La session vide sert de tampon interne sans document ouvert : aucun rendu,
// aucune édition ni copie de récupération n’est alors déclenché.
pub(super) struct DocumentState {
    pub(super) open: bool,
    pub(super) session: Session,
    pub(super) selection: CCursorRange,
    pub(super) force_selection: bool,
    pub(super) ime: super::ime::Ime,
    pub(super) statistics: crate::statistics::Counter,
    pub(super) history_field: Option<egui::Id>,
    pub(super) presentation: ModelPresentation,
}
