#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use typstographe::{app, engine, i18n, tr};

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // La même langue d’interface est utilisée par l’UI et le thread de rendu.
    if std::env::args().any(|arg| arg.starts_with("--capture")) {
        i18n::initialize(&["fr".parse().expect("langue de capture valide")]);
    } else {
        let _ = i18n::loader();
    }
    let mut args = std::env::args().skip(1);
    let mut capture = None;
    let mut capture_recovery = None;
    let mut capture_search = None;
    let mut capture_spelling = false;
    let mut capture_profile = None;
    let mut capture_creation = false;
    let mut open_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--commands-catalogue" | "--commands-catalogue-json" => {
                let value = engine::registry::catalogue_json();
                if arg.ends_with("-json") {
                    println!("{}", serde_json::to_string_pretty(&value)?);
                } else {
                    print!("{}", engine::registry::catalogue_markdown(&value));
                }
                return Ok(());
            }
            "--capture" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or(crate::tr!("cli-usage-capture"))?,
                ))
            }
            "--capture-new-document" => {
                capture = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-creation"))?,
                ));
                capture_creation = true;
            }
            "--capture-search" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or(crate::tr!("cli-usage-capture-search"))?,
                ));
                capture_search = Some(args.next().ok_or(crate::tr!("cli-usage-capture-search"))?);
            }
            "--capture-spelling" => {
                capture = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-spelling"))?,
                ));
                capture_spelling = true;
            }
            "--capture-profile" | "--capture-profile-letter" => {
                capture = Some(PathBuf::from(
                    args.next().ok_or(crate::tr!("cli-usage-capture-profile"))?,
                ));
                capture_profile = Some(arg == "--capture-profile");
            }
            "--capture-recovery" => {
                capture = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-recovery"))?,
                ));
                capture_recovery = Some(PathBuf::from(
                    args.next()
                        .ok_or(crate::tr!("cli-usage-capture-recovery"))?,
                ));
            }
            "--help" | "-h" => {
                println!("{}", crate::tr!("cli-help"));
                return Ok(());
            }
            _ if !arg.starts_with('-') && open_path.is_none() => {
                open_path = Some(PathBuf::from(arg))
            }
            _ => return Err(crate::tr!("cli-unknown-option", option = arg).into()),
        }
    }
    let options = eframe::NativeOptions {
        persist_window: capture.is_none(),
        viewport: eframe::egui::ViewportBuilder::default()
            // eframe emploie aussi ce titre initial pour le menu de l’application
            // macOS. Le titre du document est renseigné à la première frame.
            .with_title("typstOgraphe")
            .with_inner_size([1240.0, 900.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "typstOgraphe",
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc, capture, open_path, capture_recovery)?;
            if let Some(query) = capture_search {
                app.capture_search(query);
            }
            if capture_spelling {
                app.capture_spelling(&cc.egui_ctx);
            }
            if let Some(preferences) = capture_profile {
                app.capture_profile(&cc.egui_ctx, preferences);
            }
            if capture_creation {
                app.capture_creation(&cc.egui_ctx);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
