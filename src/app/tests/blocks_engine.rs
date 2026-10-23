use super::annotations_engine::{equivalent, gesture, pair};
use super::*;
use crate::engine::{Command as Core, Empty};
use serde_json::json;

#[test]
fn list_description_and_note_enter_match_json_with_undo_and_redo() {
    for source in [
        "- Élément",
        "+ Élément",
        "/ Terme: Définition",
        "#postscript[Merci]",
        "#attachment[Rapport]",
    ] {
        let end = Document::from_source(source).unwrap().view.len();
        let (ctx, mut app, mut engine) = pair(source, end..end);
        for _ in 0..2 {
            gesture(
                &ctx,
                &mut app,
                &mut engine,
                egui::Key::Enter,
                egui::Modifiers::NONE,
                "block.split",
                json!({}),
            );
        }
        assert_eq!(
            *engine.document.view.blocks.last().unwrap(),
            BlockKind::Paragraph
        );
        for undo in [true, true, false, false] {
            app.apply(if undo { Command::Undo } else { Command::Redo });
            engine
                .run(if undo {
                    Core::Undo(Empty {})
                } else {
                    Core::Redo(Empty {})
                })
                .unwrap();
            equivalent(&app, &engine);
        }
    }
}

#[test]
fn description_tab_shift_tab_and_note_tab_match_json_navigation_and_exit() {
    let (ctx, mut app, mut engine) = pair("/ Échéance: Vendredi", 0..0);
    for part in ["definition", "definition", "term"] {
        gesture(
            &ctx,
            &mut app,
            &mut engine,
            egui::Key::Tab,
            if part == "term" {
                egui::Modifiers::SHIFT
            } else {
                egui::Modifiers::NONE
            },
            "text.select",
            json!({"part":part}),
        );
    }
    assert!(engine.undo.is_empty());
    for source in [
        "#postscript[Merci *encore*]",
        "#attachment[Rapport]\n#attachment[Formulaire]",
    ] {
        let (ctx, mut app, mut engine) = pair(source, 1..4);
        gesture(
            &ctx,
            &mut app,
            &mut engine,
            egui::Key::Tab,
            egui::Modifiers::NONE,
            "block.exit",
            json!({}),
        );
        assert_eq!(engine.undo.len(), 1);
    }
}

#[test]
fn native_text_shortcuts_and_typing_mode_json_keep_the_same_complete_literal_undo() {
    for (source, text) in [
        ("\\=", " "),
        ("\\==", " "),
        ("\\===", " "),
        ("\\-", " "),
        ("\\+", " "),
        ("1\\.", " "),
        ("\\/", " "),
        ("\\>", " "),
        ("\\*Été 日本", "*"),
        ("\\_Été 日本", "_"),
        ("/ Échéance\\:: Vendredi", " "),
    ] {
        let at = if source.starts_with("/ Échéance") {
            9
        } else {
            Document::from_source(source).unwrap().view.len()
        };
        let (ctx, mut app, mut engine) = pair(source, at..at);
        engine
            .run_call(
                Core::from_call("text.insert", json!({"text":text,"input_rules":"typing"}))
                    .unwrap()
                    .to_call(),
            )
            .unwrap();
        frame(&mut app, &ctx, vec![egui::Event::Text(text.into())]);
        equivalent(&app, &engine);
        assert_eq!(engine.undo.len(), 1);
        app.apply(Command::Undo);
        engine.run(Core::Undo(Empty {})).unwrap();
        equivalent(&app, &engine);
        app.apply(Command::Redo);
        engine.run(Core::Redo(Empty {})).unwrap();
        equivalent(&app, &engine);
    }
}

#[test]
fn ime_commit_keeps_literal_text_even_when_it_looks_like_a_typing_shortcut() {
    let (ctx, mut app, _) = pair("", 0..0);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Ime(egui::ImeEvent::Enabled),
            egui::Event::Ime(egui::ImeEvent::Preedit("= ".into())),
        ],
    );
    assert!(app.document.engine.document.view.text.is_empty());
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Commit("= ".into()))],
    );
    assert_eq!(app.document.engine.document.view.text, "= ");
    assert_eq!(
        app.document.engine.document.view.blocks,
        [BlockKind::Paragraph]
    );
    assert_eq!(app.document.engine.undo.len(), 1);
}
