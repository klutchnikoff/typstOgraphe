use super::annotations_engine::{equivalent, pair};
use super::*;
use crate::engine::{Command as Core, Empty, ImageInput};
use serde_json::json;

#[test]
fn image_import_replace_alt_and_remove_match_json_with_shared_history_and_archiving() {
    let (ctx, mut app, mut engine) = pair("Avant.\n\nAprès.", 6..6);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("image.svg");
    std::fs::write(&file, include_bytes!("../../../../../fixtures/parc.svg")).unwrap();
    engine
        .run_call(
            Core::from_call("insert.image", json!({"image":{"kind":"path","path":file}}))
                .unwrap()
                .to_call(),
        )
        .unwrap();
    let mut input = frame_input(vec![]);
    input.dropped_files.push(egui::DroppedFile {
        path: Some(file.clone()),
        ..Default::default()
    });
    let _ = ctx.run(input, |ctx| app.ui_frame(ctx));
    equivalent(&app, &engine);
    let target = engine.image_target(1).unwrap();
    engine
        .run_call(
            Core::from_call(
                "image.replace",
                json!({"target":target,"image":{"kind":"path","path":file}}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap();
    app.import_image(&file, Some(1)).unwrap();
    equivalent(&app, &engine);
    frame(&mut app, &ctx, vec![]);
    app.open_image_info(1);
    app.dialogs.image_alt = "日本 : des arbres".into();
    let target = engine.image_target(1).unwrap();
    engine
        .run_call(
            Core::from_call(
                "image.set_alt",
                json!({"target":target,"alt":app.dialogs.image_alt}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap();
    engine
        .run_call(
            Core::from_call("text.select", json!({"part":"annotation"}))
                .unwrap()
                .to_call(),
        )
        .unwrap();
    recovery_click(&mut app, &ctx, &crate::tr!("image-apply"));
    assert!(app.dialogs.image_info.is_none());
    equivalent(&app, &engine);
    let target = engine.image_target(1).unwrap();
    engine
        .run_call(
            Core::from_call("image.remove", json!({"target":target}))
                .unwrap()
                .to_call(),
        )
        .unwrap();
    app.remove_image(1);
    equivalent(&app, &engine);
    engine.run(Core::Undo(Empty {})).unwrap();
    app.apply(Command::Undo);
    equivalent(&app, &engine);
    let archive = dir.path().join("courrier.typsto");
    assert!(app.save_path(archive.clone()));
    std::fs::remove_file(&file).unwrap();
    let stored = storage::read(&archive).unwrap();
    assert_eq!(
        stored.assets.len(),
        1,
        "seule l’image utilisée est archivée"
    );
    assert_eq!(stored.source, engine.document().source());
}

#[test]
fn explicit_signature_commands_match_profile_copy_removal_and_undo_without_global_mutation() {
    let (_, mut app, mut engine) = pair("Bonjour", 0..0);
    let image = crate::assets::ImageData {
        extension: "svg".into(),
        data: include_bytes!("../../../../../fixtures/signature.svg")
            .as_slice()
            .into(),
    };
    app.profile_signature = Some(image.clone());
    app.preferences.profile.signature = Some("managed.svg".into());
    let preferences = app.preferences.clone();
    let guard = engine.target_guard();
    engine
        .run_call(
            Core::from_call(
                "document.set_signature",
                json!({"guard":guard,"image":ImageInput::from(&image)}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap();
    app.use_profile_signature().unwrap();
    equivalent(&app, &engine);
    let guard = engine.target_guard();
    engine
        .run_call(
            Core::from_call(
                "document.set_signature",
                json!({"guard":guard,"image":null}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap();
    app.set_document_signature(None).unwrap();
    equivalent(&app, &engine);
    engine.run(Core::Undo(Empty {})).unwrap();
    app.apply(Command::Undo);
    equivalent(&app, &engine);
    assert_eq!(app.preferences, preferences);
}

#[test]
fn stale_image_dialog_refuses_its_edit_without_retargeting_the_current_image() {
    let (ctx, mut app, _) = pair("Avant.", 0..0);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("image.svg");
    std::fs::write(&file, include_bytes!("../../../../../fixtures/parc.svg")).unwrap();
    app.import_image(&file, None).unwrap();
    frame(&mut app, &ctx, vec![]);
    app.open_image_info(1);
    app.dialogs.image_alt = "Texte devenu périmé".into();
    let target = app.document.engine.image_target(1).unwrap();
    app.document
        .engine
        .run_call(
            Core::from_call(
                "image.set_alt",
                json!({"target":target,"alt":"Modification plus récente"}),
            )
            .unwrap()
            .to_call(),
        )
        .unwrap();
    let revision = app.document.engine.revision();
    let history = app.document.engine.history().undo;
    recovery_click(&mut app, &ctx, &crate::tr!("image-apply"));
    assert_eq!(app.document.engine.revision(), revision);
    assert_eq!(app.document.engine.history().undo, history);
    assert_eq!(
        app.document.engine.document().view.images[&1].alt,
        "Modification plus récente"
    );
    assert!(app.dialogs.image_info.is_some());
    assert_eq!(app.dialogs.message, Some(crate::tr!("edit-stale-draft")));
}
