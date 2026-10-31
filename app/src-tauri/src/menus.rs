#[tauri::command]
pub fn set_menu_availability(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    states: std::collections::BTreeMap<String, bool>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // The menu bar belongs to the foreground window, not a background participant.
        if !window.is_focused().map_err(|e| e.to_string())? {
            return Ok(());
        }
        fn update(
            items: Vec<tauri::menu::MenuItemKind<tauri::Wry>>,
            states: &std::collections::BTreeMap<String, bool>,
        ) -> Result<(), tauri::Error> {
            for item in items {
                match item {
                    tauri::menu::MenuItemKind::MenuItem(item) => {
                        if let Some(enabled) = states.get(item.id().as_ref()) {
                            item.set_enabled(*enabled)?;
                        }
                    }
                    tauri::menu::MenuItemKind::Submenu(menu) => update(menu.items()?, states)?,
                    _ => {}
                }
            }
            Ok(())
        }
        if let Some(menu) = app.menu() {
            update(menu.items().map_err(|e| e.to_string())?, &states).map_err(|e| e.to_string())?;
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, app, states);
    Ok(())
}
pub fn install(app: &tauri::AppHandle) -> Result<(), tauri::Error> {
    #[cfg(target_os = "macos")]
    {
        use tauri::menu::{MenuBuilder, SubmenuBuilder};
        let native = typstographe_i18n::native_menu_labels();
        let labels = typstographe_i18n::web_messages();
        let label = |source: &str| {
            labels
                .get(source)
                .or_else(|| labels.get(source.trim_end_matches('…')))
                .cloned()
                .unwrap_or_else(|| source.into())
        };
        let quit = tauri::menu::MenuItem::with_id(
            app,
            "app.quit",
            label("Quitter"),
            true,
            Some("CmdOrCtrl+Q"),
        )?;
        let application = SubmenuBuilder::new(app, "typstOgraphe")
            .about_with_text(&native.about, None)
            .separator()
            .text("ui.preferences", label("Préférences…"))
            .separator()
            .hide_with_text(&native.hide)
            .hide_others_with_text(&native.hide_others)
            .separator()
            .item(&quit)
            .build()?;
        let file = SubmenuBuilder::new(app, label("Fichier"))
            .text("document.new", label("Nouveau document"))
            .text("document.open", label("Ouvrir…"))
            .text("recovery.list", label("Travail à reprendre"))
            .text("recovery.import", label("Ouvrir un brouillon de secours…"))
            .separator()
            .text("document.save", label("Enregistrer"))
            .text("document.save_as", label("Enregistrer sous…"))
            .text("document.export", label("Exporter…"))
            .separator()
            .text("document.close", label("Fermer le document"))
            .build()?;
        let edit = SubmenuBuilder::new(app, label("Édition"))
            .text("history.undo", typstographe_i18n::text("cmd-undo"))
            .text("history.redo", label("Rétablir"))
            .separator()
            .cut_with_text(&native.cut)
            .copy_with_text(&native.copy)
            .paste_with_text(&native.paste)
            .select_all_with_text(&native.select_all)
            .separator()
            .text("ui.find", label("Rechercher et remplacer…"))
            .text("find.next", label("Occurrence suivante"))
            .text("find.previous", label("Occurrence précédente"))
            .separator()
            .text("text.clear", label("Effacer la mise en forme"))
            .text("text.unlink", label("Supprimer le lien"))
            .text(
                "spelling.toggle",
                label("Activer / désactiver la correction orthographique"),
            )
            .text("ui.palette", label("Palette de commandes…"))
            .build()?;
        let view = SubmenuBuilder::new(app, label("Fenêtre"))
            .text("ui.versions", label("Versions"))
            .text("ui.peer", label("Deuxième fenêtre"))
            .text(
                "synchronization.enable",
                label("Activer la synchronisation"),
            )
            .text(
                "synchronization.disable",
                label("Arrêter la synchronisation"),
            )
            .text("ui.connection", label("Connexion / déconnexion"))
            .build()?;
        app.set_menu(
            MenuBuilder::new(app)
                .items(&[&application, &file, &edit, &view])
                .build()?,
        )?;
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    Ok(())
}
