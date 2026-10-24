use super::*;

#[test]
fn retained_selection_and_visible_passage_map_without_focus_or_navigation_requests() {
    let mut engine = engine("Avant puis fin");
    let mut view = idle_view(&engine);
    let user = target(&engine, 11, 14);
    view.apply(ViewEvent::SelectionChanged {
        selection: user,
        origin: ViewOrigin::Preview,
    });
    view.take_editor_action(&engine).unwrap();
    view.remember_visible_position(target(&engine, 6, 6));
    let outcome = insert(&mut engine, "Été ");
    let positions = engine.position_map().unwrap();
    assert!(view.map_retained_positions(positions));
    assert_eq!(
        engine.resolve_target(view.selection().unwrap()).unwrap(),
        (15, 18)
    );
    assert_eq!(
        engine
            .resolve_target(view.visible_position().unwrap())
            .unwrap(),
        (10, 10)
    );
    assert!(!view.editor_pending());
    assert!(!view.preview_pending());
    assert!(view.apply(ViewEvent::DocumentPublished {
        selection: view.selection().unwrap(),
        origin: ViewOrigin::Agent
    }));
    assert_eq!(view.selection().unwrap().guard, outcome.guard);
    let current = view.clone();
    assert!(!view.map_retained_positions(positions));
    assert_eq!(view, current);
    assert!(!view.remember_visible_position(user));
    view.apply(ViewEvent::DocumentReplaced {
        selection: outcome.selection,
    });
    assert_eq!(view.visible_position(), None);
}
#[test]
fn removing_the_visible_passage_places_its_anchor_at_the_replacement_end() {
    let mut engine = engine("Un passage puis fin");
    let mut view = idle_view(&engine);
    view.remember_visible_position(target(&engine, 5, 5));
    engine.run(Command::from_call("text.insert",json!({"text":"Bref","target":crate::engine::TextTarget::from(target(&engine,0,10))})).unwrap()).unwrap();
    assert!(view.map_retained_positions(engine.position_map().unwrap()));
    assert_eq!(
        engine
            .resolve_target(view.visible_position().unwrap())
            .unwrap(),
        (4, 4)
    );
    let mut other = super::engine("Autre");
    insert(&mut other, "X");
    let before = view.clone();
    assert!(!view.map_retained_positions(other.position_map().unwrap()));
    assert_eq!(view, before);
}
