//! Événements natifs egui : le pont vers le presse-papiers de l’OS reste un essai manuel.
use super::*;
use crate::engine::{Command as Core, InsertTextParams, TargetParams, TextTarget};

fn editing() -> (egui::Context, App) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source("Un *été*.").unwrap();
    });
    app.document.selection = editor::cursor(3..6);
    app.document.force_selection = true;
    frame(&mut app, &ctx, vec![]);
    (ctx, app)
}
fn copied(output: &egui::FullOutput) -> Option<&str> {
    output
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.as_str()),
            _ => None,
        })
}

#[test]
fn native_copy_and_cut_use_visible_unicode_text_and_cut_matches_guarded_delete() {
    let (ctx, mut app) = editing();
    let before = app.stored_document();
    let guard = app.document.engine.target_guard();
    let output = search_draw(&mut app, &ctx, vec![egui::Event::Copy]);
    assert_eq!(copied(&output), Some("été"));
    assert_eq!(app.document.engine.target_guard(), guard);
    assert_eq!(app.stored_document(), before);
    assert!(!app.document.engine.can_undo());
    let mut expected: crate::engine::Engine =
        crate::engine::Engine::from_stored(before.clone(), None).unwrap();
    let target: TextTarget = expected
        .selection_target(editor::to_core(app.document.selection))
        .unwrap()
        .into();
    expected
        .run(Core::DeleteText(TargetParams { target }))
        .unwrap();
    let output = search_draw(&mut app, &ctx, vec![egui::Event::Cut]);
    assert_eq!(copied(&output), Some("été"));
    assert_eq!(app.stored_document(), expected.stored_document());
    // L’affinité visuelle de ligne d’egui n’appartient pas à la cible JSON.
    let actual = app.document.engine.selection();
    let expected = expected.selection();
    assert_eq!(
        (actual.secondary.index, actual.primary.index),
        (expected.secondary.index, expected.primary.index)
    );
    assert_eq!(app.document.engine.history().undo, 1);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.range(), 3..6);
}

#[test]
fn native_plain_paste_matches_literal_insert_and_keeps_one_complete_undo() {
    let (ctx, mut app) = editing();
    let before = app.stored_document();
    let mut expected: crate::engine::Engine =
        crate::engine::Engine::from_stored(before.clone(), None).unwrap();
    let target: TextTarget = expected
        .selection_target(editor::to_core(app.document.selection))
        .unwrap()
        .into();
    let text = "_Noël_ 🙂";
    expected
        .run(Core::InsertText(InsertTextParams {
            target,
            text: text.into(),
            input_rules: Default::default(),
        }))
        .unwrap();
    frame(&mut app, &ctx, vec![egui::Event::Paste(text.into())]);
    assert_eq!(app.stored_document(), expected.stored_document());
    // L’affinité visuelle de ligne d’egui n’appartient pas à la cible JSON.
    let actual = app.document.engine.selection();
    let expected = expected.selection();
    assert_eq!(
        (actual.secondary.index, actual.primary.index),
        (expected.secondary.index, expected.primary.index)
    );
    assert!(app.document.engine.document().view.text.contains(text));
    assert_eq!(app.document.engine.history().undo, 1);
    app.apply(Command::Undo);
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.range(), 3..6);
}
