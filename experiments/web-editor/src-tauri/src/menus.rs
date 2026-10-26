pub fn install(app: &tauri::AppHandle) -> Result<(), tauri::Error> {
    #[cfg(target_os = "macos")]
    {
        use tauri::menu::{MenuBuilder, SubmenuBuilder};
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
            .about(None)
            .separator()
            .text("ui.preferences", label("Préférences…"))
            .separator()
            .hide()
            .hide_others()
            .separator()
            .item(&quit)
            .build()?;
        let file = SubmenuBuilder::new(app, label("Fichier"))
            .text("document.new", label("Nouveau document"))
            .text("document.open", label("Ouvrir…"))
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
            .cut()
            .copy()
            .paste()
            .select_all()
            .separator()
            .text("ui.find", label("Rechercher et remplacer…"))
            .text("ui.palette", label("Palette de commandes…"))
            .build()?;
        let view = SubmenuBuilder::new(app, label("Fenêtre"))
            .text("ui.peer", label("Deuxième fenêtre"))
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
