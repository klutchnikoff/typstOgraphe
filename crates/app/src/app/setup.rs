use super::*;

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        capture: Option<PathBuf>,
        path: Option<PathBuf>,
        capture_recovery: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if capture.is_some() {
            cc.egui_ctx
                .memory_mut(|memory| *memory = Default::default());
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1240.0, 900.0)));
        }
        let mut app = Self::with_context(cc.egui_ctx.clone(), capture);
        if app.capture.path.is_none() {
            app.preferences = Preferences::load(cc.storage);
            app.preferences_directory = eframe::storage_dir("typstOgraphe");
            if let Some(directory) = &app.preferences_directory {
                match app.preferences.profile.load_signature(directory) {
                    Ok(signature) => app.profile_signature = signature,
                    Err(error) => {
                        app.dialogs.profile_error =
                            Some(crate::tr!("profile-signature-error", error = error))
                    }
                }
            }
        }
        app.preferences_tracker = crate::preferences::SaveTracker::new(&app.preferences);
        cc.egui_ctx.set_theme(app.preferences.appearance.theme());
        if let Some(path) = path {
            app.open_path(path);
        }
        // Les captures et les tests restent isolés des données réelles.
        if let Some(directory) = capture_recovery {
            app.start_recovery(directory, cc.egui_ctx.clone());
        } else if app.capture.path.is_none() {
            match eframe::storage_dir("typstOgraphe") {
                Some(directory) => {
                    app.start_recovery(directory.join("recovery"), cc.egui_ctx.clone())
                }
                None => app.recovery.error = Some(crate::tr!("recovery-no-directory")),
            }
        }
        #[cfg(target_os = "macos")]
        {
            app.native_menus = Some(menus::native::NativeMenus::new(
                cc.egui_ctx.clone(),
                &app.keymap,
            )?);
        }
        Ok(app)
    }
    pub fn with_context(ctx: egui::Context, capture: Option<PathBuf>) -> Self {
        configure_context(&ctx);
        let initial_type = DocumentType::ALL
            .into_iter()
            .find(|kind| kind.can_create())
            .expect("au moins un type possède un modèle fourni");
        let saved = StoredDocument::blank(initial_type);
        let keymap = Keymap::new(
            Registry::new(saved.metadata.document_type()),
            ctx.os().is_mac(),
            &ShortcutOverrides::default(),
        )
        .expect("raccourcis par défaut cohérents");
        let mut preferences = Preferences::default();
        if capture.is_some() {
            // Diagnostic reproductible, isolé des préférences et de l’apparence du système.
            preferences.appearance = Appearance::Light;
            ctx.set_theme(egui::ThemePreference::Light);
            ctx.all_styles_mut(|style| {
                style.visuals.text_cursor.blink = false;
                style.animation_time = 0.0;
            });
        }
        let preferences_tracker = crate::preferences::SaveTracker::new(&preferences);
        let spelling_ctx = ctx.clone();
        let spelling = crate::spelling::Checker::new(move || spelling_ctx.request_repaint());
        let worker = RenderWorker::spawn(std::sync::Arc::new(move || ctx.request_repaint()));
        Self {
            #[cfg(feature = "mcp-acceptance")]
            acceptance: None,
            bands: Default::default(),
            document: DocumentState {
                open: false,
                engine: crate::engine::Engine::empty(),
                selection: CCursorRange::one(CCursor::new(0)),
                force_selection: false,
                ime: Default::default(),
                history_field: None,
                presentation: ModelPresentation::default(),
            },
            dialogs: DialogState {
                marker_draft: None,
                preferences_page: PreferencesPage::default(),
                preferences_open: false,
                assistance_open: false,
                new_document_open: false,
                new_document_type: initial_type,
                new_document_model: initial_type.models().next().map(|entry| entry.model),
                information_open: false,
                information_group: InformationGroup::General,
                palette_open: false,
                palette_query: String::new(),
                palette_index: 0,
                message: None,
                profile_error: None,
            },
            file: FileState {
                command: None,
                pending_action: None,
                error: None,
                allow_close: false,
                window_title: String::new(),
                read_only_offer: None,
                export_open: false,
                export_format: Default::default(),
                export_page_images: true,
                export_guard: None,
                export_task: None,
                export_path: std::env::current_dir()
                    .unwrap_or_default()
                    .join("output")
                    .join("courrier")
                    .display()
                    .to_string(),
            },
            preview: PreviewState {
                worker,
                pages: vec![],
                submitted: 0,
                elapsed: None,
                error: None,
                revision: None,
                render: None,
                sync: ViewSync::default(),
            },
            search: SearchState::default(),
            spelling: SpellingState {
                checker: spelling,
                target: None,
                capture_active: false,
            },
            recovery: RecoveryState {
                worker: None,
                entries: vec![],
                open: false,
                error: None,
                recovered_name: None,
            },
            capture: CaptureState {
                path: capture,
                screenshot_requested: false,
                started: Instant::now(),
            },
            preferences,
            preferences_tracker,
            preferences_directory: None,
            profile_signature: None,
            keymap,
            #[cfg(target_os = "macos")]
            native_menus: None,
            image_textures: Default::default(),
            assistance: Default::default(),
        }
    }
}

fn configure_context(ctx: &egui::Context) {
    editor::install_fonts(ctx);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.visuals.selection.bg_fill = if style.visuals.dark_mode {
            Color32::from_rgb(40, 80, 108)
        } else {
            Color32::from_rgb(174, 211, 235)
        };
        style.visuals.selection.stroke = egui::Stroke::new(
            1.0_f32,
            if style.visuals.dark_mode {
                Color32::LIGHT_BLUE
            } else {
                ACCENT
            },
        );
    });
}
