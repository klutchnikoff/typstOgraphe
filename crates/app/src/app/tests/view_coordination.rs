use super::*;
use std::sync::mpsc::{Receiver, Sender};

/// Le service est contrôlé pour que seules les réponses envoyées par le test
/// puissent modifier la vue entre deux événements natifs.
fn controlled(
    source: &str,
) -> (
    egui::Context,
    App,
    Sender<crate::rendering::RenderResponse>,
    Receiver<crate::rendering::PreviewRequest>,
) {
    let (ctx, mut app) = demo_app();
    fixture(&mut app, |data| {
        data.document = Document::from_source(source).unwrap();
    });
    let (sender, requests) = std::sync::mpsc::channel();
    let (responses, receiver) = std::sync::mpsc::channel();
    app.preview.worker = RenderWorker { sender, receiver };
    app.preview.submitted = app.document.engine.revision();
    app.document.selection = editor::cursor(0..0);
    app.document.force_selection = true;
    ctx.style_mut(|style| style.animation_time = 0.0);
    frame(&mut app, &ctx, vec![]);
    (ctx, app, responses, requests)
}

fn published_target(app: &App) -> crate::engine::SelectionTarget {
    app.document
        .engine
        .selection_target(editor::to_core(app.document.selection))
        .unwrap()
}

#[test]
fn preedit_stays_out_of_view_sync_and_commit_waits_for_its_own_render() {
    let (ctx, mut app, responses, _requests) = controlled("Une phrase à compléter.");
    let baseline = app.preview.sync.clone();
    let initial = app.stored_document();
    for text in ["にほん", "日本"] {
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Ime(egui::ImeEvent::Preedit(text.into()))],
        );
        assert_eq!(app.preview.sync, baseline);
        assert_eq!(app.stored_document(), initial);
    }
    let old_guard = app.document.engine.target_guard();
    let old_request = app.document.engine.preview_request().unwrap();
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Commit("日本".into()))],
    );
    assert!(
        app.document
            .engine
            .document()
            .view
            .text
            .starts_with("日本Une")
    );
    assert_eq!(app.preview.sync.selection(), Some(published_target(&app)));
    assert!(app.preview.sync.preview_pending());
    assert_eq!(app.document.engine.history().undo, 1);
    // L'ancien rendu arrive après la validation : il ne doit pas installer sa géométrie.
    let old_output =
        crate::rendering::render(&mut crate::rendering::TypstWorld::default(), &old_request)
            .unwrap();
    responses
        .send(crate::rendering::RenderResponse {
            guard: old_guard,
            result: Ok(old_output),
            is_export: false,
        })
        .unwrap();
    frame(&mut app, &ctx, vec![]);
    assert!(app.preview.render.is_none());
    assert!(app.preview.sync.preview_pending());
    let guard = app.document.engine.target_guard();
    let output = crate::rendering::render(
        &mut crate::rendering::TypstWorld::default(),
        &app.document.engine.preview_request().unwrap(),
    )
    .unwrap();
    responses
        .send(crate::rendering::RenderResponse {
            guard,
            result: Ok(output),
            is_export: false,
        })
        .unwrap();
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.preview.render.as_ref().unwrap().reference.guard, guard);
    assert!(!app.preview.sync.preview_pending());
    assert_eq!(app.preview.sync.selection(), Some(published_target(&app)));
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
}

#[test]
fn pending_reveal_uses_the_native_frame_caret_when_typing_before_publication() {
    let (ctx, mut app, _responses, _requests) =
        controlled(&"Une ligne à retrouver.\n\n".repeat(100));
    let before = app.document.engine.document().view.text.clone();
    let end = before.chars().count();
    ctx.style_mut(|style| style.scroll_animation = egui::style::ScrollAnimation::none());
    let draw = |app: &mut App, events| {
        let mut offset = 0.0;
        let _ = ctx.run(frame_input(events), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let id = ui.make_persistent_id(egui::Id::new("editor-scroll"));
                app.editor_ui(ui);
                offset = egui::scroll_area::State::load(ctx, id).unwrap().offset.y;
            });
        });
        offset
    };
    // Le ScrollArea a besoin d'un placement préalable pour connaître sa hauteur.
    draw(&mut app, vec![]);
    draw(&mut app, vec![]);
    // Le widget possède déjà le curseur natif, mais son retour n'a pas encore
    // été publié au moteur ni dans le cache App.
    editor::store_cursor(&ctx, editor::cursor(end..end));
    assert_eq!(app.document.selection.primary.index, 0);
    app.request_view_reveal(RevealDestination::Editor);
    draw(&mut app, vec![egui::Event::Text("Ajout".into())]);
    // Même sans durée d'animation, egui applique sa cible au début de la frame suivante.
    let offset = draw(&mut app, vec![]);
    assert_eq!(
        app.document.engine.document().view.text,
        format!("{before}Ajout")
    );
    assert_eq!(app.document.selection.primary.index, end + 5);
    assert_eq!(app.preview.sync.selection(), Some(published_target(&app)));
    assert!(!app.preview.sync.editor_pending());
    assert!(app.preview.sync.preview_pending());
    assert_eq!(app.document.engine.history().undo, 1);
    // La vérification du placement emploie le galley natif de la frame : un
    // défilement vers la cible publiée (zéro) aurait laissé le curseur hors écran.
    assert!(offset > 1000.0, "editor offset {offset}");
    let state = egui::TextEdit::load_state(&ctx, egui::Id::new(editor::EDITOR_ID)).unwrap();
    assert_eq!(state.cursor.char_range().unwrap(), app.document.selection);
    assert!(ctx.memory(|m| m.has_focus(egui::Id::new(editor::EDITOR_ID))));
}

#[test]
fn revealing_a_search_result_keeps_search_focus_and_does_not_edit_the_body() {
    let (ctx, mut app, _responses, _requests) = controlled("Une phrase, une autre phrase.");
    let before = app.stored_document();
    app.apply(Command::Find);
    app.search.engine.query = "phrase".into();
    frame(&mut app, &ctx, vec![]);
    app.navigate_search(false);
    assert!(app.preview.sync.editor_pending());
    frame(&mut app, &ctx, vec![egui::Event::Text("!".into())]);
    // L'ouverture du panneau sélectionne la requête entière : la saisie doit
    // la remplacer, et ne doit jamais être envoyée au corps du document.
    assert_eq!(app.search.engine.query, "!");
    assert_eq!(app.search_focus(&ctx), Some(egui::Id::new(SEARCH_ID)));
    assert!(!app.preview.sync.editor_pending());
    assert_eq!(app.preview.sync.selection(), Some(published_target(&app)));
    assert_eq!(app.stored_document(), before);
    assert_eq!(app.document.engine.history().undo, 0);
}
