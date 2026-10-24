use super::*;
use crate::{
    engine::{Empty, LocateParams, Query, RenderPosition, SelectionBoxesParams},
    navigation::PagePoint,
};

fn reveals(actions: &[ViewAction]) -> Vec<PagePoint> {
    actions
        .iter()
        .filter_map(|action| match action {
            ViewAction::RevealInPreview { point, .. } => Some(*point),
            _ => None,
        })
        .collect()
}

fn boxes(actions: &[ViewAction]) -> &[crate::navigation::SelectionHighlight] {
    let ViewAction::Highlights { boxes, .. } = &actions[0] else {
        panic!("highlights update")
    };
    boxes
}

#[test]
fn latest_selection_uses_real_typst_geometry_once_without_document_mutation() {
    let engine = engine("Bonjour Noël et Élodie.");
    let render = publish_render(&engine);
    let mut view = view(&engine);
    view.apply(ViewEvent::RenderInstalled { render });
    for (anchor, head) in [(0, 7), (8, 12), (21, 15)] {
        view.apply(ViewEvent::SelectionChanged {
            selection: target(&engine, anchor, head),
            origin: ViewOrigin::User,
        });
    }
    let selected = view.selection().unwrap();
    let before = engine.query(Query::Inspect(Empty {})).unwrap();
    let stored = engine.stored_document();
    let history = engine.history();
    let actions = view.preview_actions(&engine);
    let expected = engine
        .locate(LocateParams {
            render,
            position: RenderPosition::ToPreview {
                target: selected.into(),
            },
        })
        .unwrap()
        .point
        .unwrap();
    assert_eq!(reveals(&actions), vec![expected]);
    assert_eq!(
        boxes(&actions),
        engine
            .selection_boxes(SelectionBoxesParams {
                render,
                target: selected.into()
            })
            .unwrap()
            .boxes
    );
    assert!(!boxes(&actions).is_empty());
    assert!(reveals(&view.preview_actions(&engine)).is_empty());
    assert_eq!(engine.query(Query::Inspect(Empty {})).unwrap(), before);
    assert_eq!(engine.stored_document(), stored);
    assert_eq!(engine.history(), history);
}

#[test]
fn stale_and_failed_renders_defer_navigation_and_clear_highlights_until_installation() {
    let mut engine = engine("Texte à modifier.");
    let old_render = publish_render(&engine);
    let mut view = view(&engine);
    view.apply(ViewEvent::RenderInstalled { render: old_render });
    view.preview_actions(&engine);
    let outcome = insert(&mut engine, "Suite ");
    view.apply(ViewEvent::CommandCompleted {
        result: (&outcome).into(),
        origin: ViewOrigin::User,
    });
    let actions = view.preview_actions(&engine);
    assert!(reveals(&actions).is_empty() && boxes(&actions).is_empty());
    assert!(view.preview_pending());
    assert!(!view.apply(ViewEvent::RenderInstalled { render: old_render }));
    assert!(!view.apply(ViewEvent::RenderFailed {
        guard: old_render.guard
    }));
    let render = publish_render(&engine);
    view.apply(ViewEvent::RenderInstalled { render });
    view.apply(ViewEvent::RenderFailed {
        guard: engine.target_guard(),
    });
    assert!(reveals(&view.preview_actions(&engine)).is_empty());
    assert!(view.preview_pending());
    assert!(
        view.preview_target(
            &engine,
            PagePoint {
                page: 0,
                point: [0.0, 0.0]
            }
        )
        .is_none()
    );
    view.apply(ViewEvent::RenderInstalled { render });
    assert_eq!(reveals(&view.preview_actions(&engine)).len(), 1);
    assert!(!view.preview_pending());
}

#[test]
fn inverse_navigation_requires_successful_selection_and_views_remain_independent() {
    let mut engine = engine("Bonjour Noël et Élodie.");
    let render = publish_render(&engine);
    let mut first = view(&engine);
    first.apply(ViewEvent::RenderInstalled { render });
    let mut second = first.clone();
    let point = engine
        .locate(LocateParams {
            render,
            position: RenderPosition::ToPreview {
                target: target(&engine, 10, 10).into(),
            },
        })
        .unwrap()
        .point
        .unwrap();
    let baseline = first.clone();
    let selected = first.preview_target(&engine, point).unwrap();
    assert_eq!(first, baseline); // La localisation ne sélectionne pas et n'annule rien.
    let outcome = engine
        .run(
            Command::from_call(
                "text.select",
                json!({ "target": crate::engine::TextTarget::from(selected) }),
            )
            .unwrap(),
        )
        .unwrap();
    first.apply(ViewEvent::CommandCompleted {
        result: (&outcome).into(),
        origin: ViewOrigin::Preview,
    });
    assert!(!first.preview_pending());
    assert!(first.take_editor_action(&engine).is_some());
    assert!(first.take_editor_action(&engine).is_none());
    assert!(reveals(&first.preview_actions(&engine)).is_empty());
    assert_eq!(reveals(&second.preview_actions(&engine)).len(), 1);
    assert_eq!(second.selection(), Some(target(&engine, 0, 0)));
    for point in [
        PagePoint {
            page: 0,
            point: [0.0, 0.0],
        },
        PagePoint {
            page: 100,
            point: [-1.0, 0.0],
        },
    ] {
        let baseline = first.clone();
        assert!(first.preview_target(&engine, point).is_none());
        assert_eq!(first, baseline);
    }
}

#[test]
fn valid_render_without_an_editable_position_consumes_the_forward_request() {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "Texte masqué par le modèle.".into();
    stored.template = "#let courrier(..args, body) = [Décoration seule.]".into();
    let engine = Engine::from_stored(stored, None).unwrap();
    let render = publish_render(&engine);
    let mut view = view(&engine);
    view.apply(ViewEvent::RenderInstalled { render });
    let actions = view.preview_actions(&engine);
    assert!(reveals(&actions).is_empty() && boxes(&actions).is_empty());
    assert!(!view.preview_pending());
    assert!(reveals(&view.preview_actions(&engine)).is_empty());
}

#[test]
fn replacement_close_and_engine_ahead_of_the_view_never_use_stale_geometry() {
    let mut engine = engine("Ancien texte.");
    let render = publish_render(&engine);
    let mut view = view(&engine);
    view.apply(ViewEvent::RenderInstalled { render });
    // Même en cas d'oubli de publication par l'adaptateur, les lectures du moteur gardent la géométrie.
    insert(&mut engine, "Suite ");
    assert!(reveals(&view.preview_actions(&engine)).is_empty());
    assert!(view.preview_pending());
    assert!(view.take_editor_action(&engine).is_none());
    let other = super::engine("Nouveau texte.");
    view.apply(ViewEvent::DocumentReplaced {
        selection: Some(target(&other, 0, 0)),
    });
    assert!(!view.apply(ViewEvent::RenderInstalled { render }));
    assert!(reveals(&view.preview_actions(&other)).is_empty());
    view.apply(ViewEvent::DocumentReplaced { selection: None });
    assert!(view.take_editor_action(&other).is_none());
    assert!(reveals(&view.preview_actions(&other)).is_empty());
    assert!(
        view.preview_target(
            &other,
            PagePoint {
                page: 0,
                point: [0.0, 0.0]
            }
        )
        .is_none()
    );
}
