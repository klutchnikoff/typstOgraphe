use super::*;
use crate::{
    document_types::DocumentType,
    engine::{Caret, Command, Engine, GuardParams, Outcome, Selection},
    storage::StoredDocument,
};
use serde_json::json;

mod geometry;
mod transitions;

fn engine(source: &str) -> Engine {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = source.into();
    stored.template = "#let courrier(..args, body) = { set page(width: 220pt, height: 150pt, margin: 10pt); body }".into();
    Engine::from_stored(stored, None).unwrap()
}

fn target(engine: &Engine, anchor: usize, head: usize) -> SelectionTarget {
    engine
        .selection_target(Selection {
            secondary: Caret {
                index: anchor,
                ..Default::default()
            },
            primary: Caret {
                index: head,
                ..Default::default()
            },
        })
        .unwrap()
}

fn view(engine: &Engine) -> ViewSync {
    let mut view = ViewSync::default();
    assert!(view.apply(ViewEvent::DocumentReplaced {
        selection: Some(target(engine, 0, 0)),
    }));
    view
}

fn idle_view(engine: &Engine) -> ViewSync {
    let mut view = view(engine);
    assert!(view.apply(ViewEvent::SelectionChanged {
        selection: target(engine, 0, 0),
        origin: ViewOrigin::Preview,
    }));
    assert!(view.take_editor_action(engine).is_some());
    assert!(!view.preview_pending() && !view.editor_pending());
    view
}

fn insert(engine: &mut Engine, text: &str) -> Outcome {
    engine
        .run(Command::from_call("text.insert", json!({ "text": text })).unwrap())
        .unwrap()
}

fn publish_render(engine: &Engine) -> RenderReference {
    engine
        .compile(GuardParams {
            guard: engine.target_guard(),
        })
        .unwrap()
        .render
        .unwrap()
        .reference
}

/// Les transitions seules n'utilisent pas le cache géométrique ; son identifiant
/// opaque est fourni comme une donnée de service. Les tests geometry compilent.
fn render_reference(guard: TargetGuard) -> RenderReference {
    serde_json::from_value(json!({ "guard": guard, "id": 1 })).unwrap()
}
